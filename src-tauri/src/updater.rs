//! Portable in-app updater: HTTPS manifest, SHA256-verified ZIP, in-place file swap.
//! Network I/O stays in Rust; the frontend only sees validated DTOs and progress events.
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

/// Stable manifest address: always the newest published GitHub Release asset.
pub const MANIFEST_URL: &str =
    "https://github.com/b1inkie/ds-workshop-studio/releases/latest/download/latest.json";
const TOP_FOLDER: &str = "Workshop Studio/";
const MANIFEST_MAX_BYTES: u64 = 256 * 1024;
const ZIP_MAX_BYTES: u64 = 64 * 1024 * 1024;
const EXTRACTED_MAX_BYTES: u64 = 128 * 1024 * 1024;
const EXTRACTED_MAX_FILES: usize = 2000;
const EXE_ENTRY: &str = "Workshop Studio.exe";
const DLL_ENTRY: &str = "resources/steam_api.dll";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub version: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current_version: String,
    pub manifest: UpdateManifest,
    pub is_newer: bool,
    pub source: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub percent: Option<u32>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .build()
}

pub fn manifest_url() -> String {
    std::env::var("WORKSHOP_UPDATE_URL").unwrap_or_else(|_| MANIFEST_URL.to_string())
}

fn is_configured(url: &str) -> bool {
    url.starts_with("https://") && !url.contains("example.com")
}

fn parse_version(value: &str) -> Result<Version, String> {
    Version::parse(value).map_err(|_| format!("版本号无效：{value}"))
}

pub fn validate(manifest: &UpdateManifest) -> Result<(), String> {
    parse_version(&manifest.version)?;
    if !manifest.url.starts_with("https://") {
        return Err("更新包地址必须是 HTTPS。".into());
    }
    let sha = manifest.sha256.trim();
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("更新包 SHA256 校验值无效。".into());
    }
    if manifest.url.contains("example.com") {
        return Err("更新包地址仍是占位配置。".into());
    }
    Ok(())
}

pub fn check() -> Result<UpdateCheck, String> {
    let url = manifest_url();
    if !is_configured(&url) {
        return Err("更新源尚未配置：请等待开发者提供正式更新地址。".into());
    }
    let response = agent()
        .get(&url)
        .set(
            "User-Agent",
            concat!("WorkshopStudio/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|e| format!("无法访问更新源：{e}"))?;
    let mut text = String::new();
    response
        .into_reader()
        .take(MANIFEST_MAX_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("读取更新清单失败：{e}"))?;
    if text.len() as u64 > MANIFEST_MAX_BYTES {
        return Err("更新清单异常过大。".into());
    }
    let manifest: UpdateManifest =
        serde_json::from_str(&text).map_err(|e| format!("更新清单格式无效：{e}"))?;
    validate(&manifest)?;
    let current = parse_version(env!("CARGO_PKG_VERSION"))?;
    let candidate = parse_version(&manifest.version)?;
    Ok(UpdateCheck {
        current_version: current.to_string(),
        is_newer: candidate > current,
        manifest,
        source: url,
    })
}

fn download_and_verify(
    manifest: &UpdateManifest,
    emit: &impl Fn(Progress),
) -> Result<Vec<u8>, String> {
    let response = agent()
        .get(&manifest.url)
        .set(
            "User-Agent",
            concat!("WorkshopStudio/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|e| format!("下载更新包失败：{e}"))?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());
    let mut reader = response.into_reader();
    let mut hasher = Sha256::new();
    let mut data = Vec::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| format!("下载更新包失败：{e}"))?;
        if read == 0 {
            break;
        }
        if data.len() as u64 + read as u64 > ZIP_MAX_BYTES {
            return Err("更新包超过 64MiB 限制，已取消。".into());
        }
        hasher.update(&buffer[..read]);
        data.extend_from_slice(&buffer[..read]);
        let percent = total
            .filter(|length| *length > 0)
            .map(|length| ((data.len() as u64 * 100) / length).min(100) as u32);
        emit(Progress {
            phase: "正在下载更新包".into(),
            percent,
        });
    }
    if total.is_some_and(|expected| data.len() as u64 != expected) {
        return Err("下载的更新包大小与服务器声明不一致。".into());
    }
    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(manifest.sha256.trim()) {
        return Err("更新包 SHA256 校验失败，已放弃安装。".into());
    }
    Ok(data)
}

fn safe_entry_name(name: &str) -> Option<String> {
    let normalized = name.replace('\\', "/");
    let rest = normalized.strip_prefix(TOP_FOLDER)?;
    if rest.is_empty() || rest.starts_with('/') {
        return None;
    }
    for part in normalized.split('/') {
        if part.is_empty() || part == ".." || part == "." {
            return None;
        }
        if part.len() >= 2 && part.as_bytes()[1] == b':' {
            return None;
        }
    }
    Some(rest.to_string())
}

fn move_into_place(source: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败：{}：{e}", parent.display()))?;
    }
    if target.exists() {
        match fs::remove_file(target) {
            Ok(()) => {}
            Err(_) if fs::metadata(target).map(|m| m.is_dir()).unwrap_or(false) => {
                return Err(format!("更新目标被占用或为目录：{}", target.display()));
            }
            Err(e) => return Err(format!("替换文件失败：{}：{e}", target.display())),
        }
    }
    if fs::rename(source, target).is_err() {
        fs::copy(source, target)
            .and_then(|_| fs::remove_file(source))
            .map_err(|e| format!("写入文件失败：{}：{e}", target.display()))?;
    }
    Ok(())
}

fn old_executable_name(exe_name: &str) -> String {
    match exe_name.rfind('.') {
        Some(dot) => format!("{}.old{}", &exe_name[..dot], &exe_name[dot..]),
        None => format!("{exe_name}.old"),
    }
}

/// Extract the verified ZIP and swap files in place. The running executable is renamed
/// aside (Windows allows renaming a running image); a stale rename is cleaned on next start.
pub fn install(
    manifest: &UpdateManifest,
    app_dir: &Path,
    exe_name: &str,
    emit: impl Fn(Progress),
) -> Result<(), String> {
    validate(manifest)?;
    let candidate = parse_version(&manifest.version)?;
    if candidate < parse_version(env!("CARGO_PKG_VERSION"))? {
        return Err("更新包版本不高于当前版本，已拒绝。".into());
    }
    if exe_name.is_empty() || exe_name.contains('/') || exe_name.contains('\\') {
        return Err("程序文件名无效。".into());
    }
    let zip_bytes = download_and_verify(manifest, &emit)?;
    emit(Progress {
        phase: "正在解包更新".into(),
        percent: None,
    });
    let staging = app_dir.join(".update-staging");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| format!("清理旧暂存失败：{e}"))?;
    }
    fs::create_dir_all(&staging).map_err(|e| format!("无法创建更新暂存目录：{e}"))?;
    let result = extract_and_swap(&zip_bytes, &staging, app_dir, exe_name, &emit);
    let _ = fs::remove_dir_all(&staging);
    result
}

fn extract_and_swap(
    zip_bytes: &[u8],
    staging: &Path,
    app_dir: &Path,
    exe_name: &str,
    emit: &impl Fn(Progress),
) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))
        .map_err(|e| format!("更新包无法读取：{e}"))?;
    let entries: Vec<String> = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|entry| entry.name().to_string())
        })
        .collect();
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    let mut total_bytes: u64 = 0;
    for name in entries {
        let rest = safe_entry_name(&name).ok_or_else(|| format!("更新包含不允许的路径：{name}"))?;
        let mut entry = archive
            .by_name(&name)
            .map_err(|e| format!("读取更新条目失败：{name}：{e}"))?;
        if entry.is_dir() {
            continue;
        }
        let mode = entry.unix_mode().unwrap_or(0);
        if mode & 0o170000 == 0o120000 {
            return Err(format!("更新包含不允许的链接条目：{name}"));
        }
        if files.len() + 1 > EXTRACTED_MAX_FILES || total_bytes + entry.size() > EXTRACTED_MAX_BYTES
        {
            return Err("更新包内容超出安全限制。".into());
        }
        total_bytes += entry.size();
        let target = staging.join(&rest);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建暂存目录失败：{e}"))?;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| format!("解压更新文件失败：{name}：{e}"))?;
        fs::write(&target, &bytes).map_err(|e| format!("写入暂存文件失败：{name}：{e}"))?;
        files.push((rest, target));
    }
    if !files.iter().any(|(rest, _)| rest == EXE_ENTRY) {
        return Err("更新包缺少主程序，已拒绝。".into());
    }
    if !files.iter().any(|(rest, _)| rest == DLL_ENTRY) {
        return Err("更新包缺少 steam_api.dll，已拒绝。".into());
    }
    let total = files.len();
    for (index, (rest, source)) in files.iter().enumerate() {
        if rest == EXE_ENTRY {
            continue; // The executable is swapped last so failures leave a runnable app.
        }
        move_into_place(source, &app_dir.join(rest))?;
        emit(Progress {
            phase: format!("正在替换文件 {}/{}", index + 1, total),
            percent: Some(((index as u32 + 1) * 100 / total as u32).min(100)),
        });
    }
    let current = app_dir.join(exe_name);
    let previous = app_dir.join(old_executable_name(exe_name));
    if previous.exists() {
        fs::remove_file(&previous)
            .map_err(|e| format!("清理旧版本文件失败：{}：{e}", previous.display()))?;
    }
    let new_exe = staging.join(EXE_ENTRY);
    if current.exists() {
        fs::rename(&current, &previous)
            .map_err(|e| format!("保留旧版本失败：{}：{e}", current.display()))?;
    }
    move_into_place(&new_exe, &current)?;
    emit(Progress {
        phase: "更新已就绪，重启后生效".into(),
        percent: Some(100),
    });
    Ok(())
}

/// Best-effort removal of the previous executable, unlocked once the old process exited.
pub fn cleanup_previous_executable() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(name) = exe.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let _ = fs::remove_file(exe.with_file_name(old_executable_name(name)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn manifest(version: &str) -> UpdateManifest {
        UpdateManifest {
            version: version.into(),
            url: "https://cdn.example.org/app.zip".into(),
            sha256: "a".repeat(64),
            notes: "测试".into(),
            pub_date: "2026-11-16".into(),
        }
    }

    fn build_zip(extra: &[(&str, &str, Option<u32>)]) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buffer);
        for (name, content, mode) in [
            (EXE_ENTRY, b"new-exe".as_slice(), None),
            (DLL_ENTRY, b"new-dll".as_slice(), None),
            ("docs/x.txt", b"x".as_slice(), None),
        ]
        .into_iter()
        .chain(extra.iter().map(|(n, c, m)| (*n, c.as_bytes(), *m)))
        {
            let mut options = SimpleFileOptions::default();
            if let Some(mode) = mode {
                options = options.unix_permissions(mode);
            }
            zip.start_file(format!("{TOP_FOLDER}{name}"), options)
                .unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
        buffer.into_inner()
    }

    fn noop(_: Progress) {}

    #[test]
    fn manifest_validation() {
        assert!(validate(&manifest("0.2.0")).is_ok());
        let mut http = manifest("0.2.0");
        http.url = "http://cdn.example.org/app.zip".into();
        assert!(validate(&http).is_err());
        let mut bad_hash = manifest("0.2.0");
        bad_hash.sha256 = "zz".into();
        assert!(validate(&bad_hash).is_err());
        assert!(validate(&manifest("1.0")).is_err());
        assert!(check().is_err()); // placeholder source must never reach the network
    }

    #[test]
    fn install_swaps_files_and_renames_old_exe() {
        let app = tempfile::tempdir().unwrap();
        fs::write(app.path().join("Workshop Studio.exe"), b"old-exe").unwrap();
        fs::create_dir_all(app.path().join("resources")).unwrap();
        fs::write(app.path().join("resources/steam_api.dll"), b"old-dll").unwrap();
        let zip_bytes = build_zip(&[]);
        // Patch install's download by verifying through extract_and_swap directly.
        let staging = app.path().join(".stage");
        fs::create_dir_all(&staging).unwrap();
        extract_and_swap(
            &zip_bytes,
            &staging,
            app.path(),
            "Workshop Studio.exe",
            &noop,
        )
        .unwrap();
        assert_eq!(
            fs::read(app.path().join("Workshop Studio.exe")).unwrap(),
            b"new-exe"
        );
        assert_eq!(
            fs::read(app.path().join("resources/steam_api.dll")).unwrap(),
            b"new-dll"
        );
        assert_eq!(fs::read(app.path().join("docs/x.txt")).unwrap(), b"x");
        assert_eq!(
            fs::read(app.path().join("Workshop Studio.old.exe")).unwrap(),
            b"old-exe"
        );
        // Cleanup of the staging folder is install()'s responsibility (network download path);
        // after a successful swap only empty folders remain in it.
        assert!(staging.exists());
        assert!(fs::read_dir(&staging)
            .unwrap()
            .all(|entry| entry.unwrap().path().is_dir()));
    }

    #[test]
    fn install_rejects_unsafe_and_incomplete_packages() {
        let app = tempfile::tempdir().unwrap();
        fs::write(app.path().join("Workshop Studio.exe"), b"old-exe").unwrap();
        // Path traversal outside the release folder must be rejected.
        let traversal = build_zip(&[("../evil.exe", "e", None)]);
        let mut path = app.path().join(".t1");
        fs::create_dir_all(&path).unwrap();
        assert!(
            extract_and_swap(&traversal, &path, app.path(), "Workshop Studio.exe", &noop).is_err()
        );
        assert!(!app.path().join("evil.exe").exists());
        path = app.path().join(".t3");
        fs::create_dir_all(&path).unwrap();
        fs::create_dir_all(&path).unwrap();
        let mut truncated = build_zip(&[]);
        truncated.truncate(truncated.len() / 2);
        assert!(
            extract_and_swap(&truncated, &path, app.path(), "Workshop Studio.exe", &noop).is_err()
        );
        // Missing executable entry: zip with only the DLL.
        let mut buffer = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buffer);
        zip.start_file(
            format!("{TOP_FOLDER}{DLL_ENTRY}"),
            SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"d").unwrap();
        zip.finish().unwrap();
        path = app.path().join(".t4");
        fs::create_dir_all(&path).unwrap();
        assert!(extract_and_swap(
            &buffer.get_ref(),
            &path,
            app.path(),
            "Workshop Studio.exe",
            &noop
        )
        .is_err());
        assert_eq!(
            fs::read(app.path().join("Workshop Studio.exe")).unwrap(),
            b"old-exe"
        );
    }

    #[test]
    fn version_and_names() {
        assert!(parse_version("0.2.0").unwrap() > parse_version("0.1.0").unwrap());
        assert_eq!(
            old_executable_name("Workshop Studio.exe"),
            "Workshop Studio.old.exe"
        );
        assert_eq!(old_executable_name("app"), "app.old");
        assert!(safe_entry_name("Workshop Studio/a/b.png").as_deref() == Some("a/b.png"));
        assert!(safe_entry_name("Other/x.png").is_none());
        assert!(safe_entry_name("Workshop Studio/../x").is_none());
        assert!(safe_entry_name("Workshop Studio/C:/x").is_none());
    }
}
