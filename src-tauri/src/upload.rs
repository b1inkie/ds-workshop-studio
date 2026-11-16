//! Immutable, local upload plans. Preparing never mutates Steam or source files.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tempfile::TempDir;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub app_id: u32,
    pub id: String,
    pub folder: String,
    pub preview: String,
    pub title: String,
    pub description: String,
    pub tags: String,
    pub visibility: i32,
    pub change_note: String,
    pub update_content: bool,
    pub update_preview: bool,
    pub update_metadata: bool,
    pub redundant: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub token: String,
    pub app_id: u32,
    pub id: String,
    pub title: String,
    pub files: u32,
    pub bytes: u64,
    pub skipped: u32,
    pub version: String,
    pub previous_version: String,
    pub update_content: bool,
    pub update_preview: bool,
    pub update_metadata: bool,
    pub visibility: i32,
    pub warnings: Vec<String>,
}
pub struct Plan {
    pub request: Request,
    pub summary: Summary,
    pub stage: TempDir,
    pub version: String,
    pub compatible: String,
    pub tags: Vec<String>,
    pub steam_id: String,
    pub created: std::time::Instant,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub processed: u64,
    pub total: u64,
    pub id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub id: String,
    pub needs_legal_agreement: bool,
    pub error: Option<String>,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn safe_string(value: &str, max: usize, label: &str) -> Result<(), String> {
    if value.contains('\0') || value.len() > max {
        return Err(format!("{label} 含空字符或超过 {max} 字节限制。"));
    }
    Ok(())
}
fn reparse(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}
fn redundant(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "fla" | "scml" | "mp3" | "wav"
    )
}
fn copy_tree(
    source: &Path,
    target: &Path,
    root: bool,
    redundant_files: bool,
    stats: &mut (u32, u64, u32),
    depth: u32,
) -> Result<(), String> {
    if depth > 64 {
        return Err("目录层级超过 64，拒绝上传。".into());
    }
    fs::create_dir_all(target).map_err(err)?;
    let mut entries = fs::read_dir(source)
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "文件名必须可表示为 UTF-8。")?;
        let meta = fs::symlink_metadata(entry.path()).map_err(err)?;
        if name.starts_with('.') || reparse(&meta) || meta.file_type().is_symlink() {
            stats.2 += 1;
            continue;
        }
        if meta.is_dir() {
            copy_tree(
                &entry.path(),
                &target.join(name),
                false,
                redundant_files,
                stats,
                depth + 1,
            )?;
        } else if meta.is_file() {
            let ext = Path::new(&name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if (root && name.eq_ignore_ascii_case("mod.manifest"))
                || ext == "illegaldummyformat"
                || (!redundant_files && redundant(ext))
            {
                stats.2 += 1;
                continue;
            }
            if stats.0 >= 100_000 || stats.1.saturating_add(meta.len()) > 4 * 1024 * 1024 * 1024 {
                return Err("本预览版限制 10 万个文件、4GiB 内容。".into());
            }
            let copied = fs::copy(entry.path(), target.join(name)).map_err(err)?;
            stats.0 += 1;
            stats.1 += copied;
        }
    }
    Ok(())
}
fn manifest_paths(root: &Path, current: &Path, output: &mut Vec<String>) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    for entry in entries.iter().filter(|e| e.path().is_file()) {
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(err)?
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase();
        if relative != "mod.manifest" {
            output.push(relative);
        }
    }
    for entry in entries.iter().filter(|e| e.path().is_dir()) {
        manifest_paths(root, &entry.path(), output)?;
    }
    Ok(())
}
fn manifest(root: &Path) -> Result<(), String> {
    let mut paths = Vec::new();
    manifest_paths(root, root, &mut paths)?;
    let mut f = fs::File::create(root.join("mod.manifest")).map_err(err)?;
    f.write_all(b"MNFS").map_err(err)?;
    f.write_all(&1u32.to_le_bytes()).map_err(err)?;
    f.write_all(&(paths.len() as u32).to_le_bytes())
        .map_err(err)?;
    for path in paths {
        let hash = path.bytes().fold(0u32, |h, b| {
            (b as u32)
                .wrapping_add(h.wrapping_shl(6))
                .wrapping_add(h.wrapping_shl(16))
                .wrapping_sub(h)
        });
        f.write_all(&hash.to_le_bytes()).map_err(err)?;
    }
    Ok(())
}
pub fn normalized(value: &str) -> String {
    value
        .trim()
        .trim_start_matches(['v', 'V'])
        .trim()
        .to_lowercase()
}
fn tags(raw: &str, version: &str, compatible: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::<String>::new();
    for value in raw
        .split([',', '，', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if [
            "version:",
            "version=",
            "version_compatible:",
            "version_compatible=",
        ]
        .iter()
        .any(|p| value.to_lowercase().starts_with(p))
        {
            continue;
        }
        safe_string(value, 255, "Tag")?;
        if !result.iter().any(|s| s.eq_ignore_ascii_case(value)) {
            result.push(value.to_string());
        }
    }
    if !version.is_empty() {
        result.push(format!("version:{version}"));
    }
    if !compatible.is_empty() {
        result.push(format!("version_compatible:{compatible}"));
    }
    if result.len() > 100 || result.iter().map(String::len).sum::<usize>() > 4096 {
        return Err("Tags 数量或总长度过大。".into());
    }
    Ok(result)
}
fn package_ds(source: &Path, target: &Path) -> Result<(), String> {
    fn append(
        root: &Path,
        current: &Path,
        zip: &mut zip::ZipWriter<fs::File>,
    ) -> Result<(), String> {
        let mut entries = fs::read_dir(current)
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            if entry.file_type().map_err(err)?.is_dir() {
                append(root, &entry.path(), zip)?;
            } else {
                let path = entry.path();
                let name = path
                    .strip_prefix(root)
                    .map_err(err)?
                    .to_string_lossy()
                    .replace('\\', "/");
                zip.start_file(
                    name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .map_err(err)?;
                std::io::copy(&mut fs::File::open(path).map_err(err)?, zip).map_err(err)?;
            }
        }
        Ok(())
    }
    let mut zip = zip::ZipWriter::new(fs::File::create(target).map_err(err)?);
    append(source, source, &mut zip)?;
    zip.finish().map_err(err)?;
    // The SDK FileWrite uses a signed 32-bit size and a contiguous allocation. Cap before loading.
    if fs::metadata(target).map_err(err)?.len() > 100 * 1024 * 1024 {
        return Err("单机版 ZIP 超过当前 100MiB 安全限制，请精简资源后重试。".into());
    }
    Ok(())
}
pub fn prepare(mut request: Request, cache: &Path) -> Result<Plan, String> {
    if ![322330, 219740].contains(&request.app_id) {
        return Err("仅支持饥荒联机版和单机版。".into());
    }
    let id = request
        .id
        .parse::<u64>()
        .map_err(|_| "工坊 ID 必须是 0 或有效数字。")?;
    if id == u64::MAX {
        return Err("工坊 ID 无效。".into());
    }
    request.id = id.to_string();
    if id == 0 {
        request.update_content = true;
        request.update_preview = true;
        request.update_metadata = true;
    }
    if !request.update_content && !request.update_preview && !request.update_metadata {
        return Err("请至少选择一种更新内容。".into());
    }
    if !(0..=3).contains(&request.visibility) {
        return Err("可见性无效。".into());
    }
    safe_string(&request.title, 128, "标题")?;
    safe_string(&request.description, 7999, "简介")?;
    safe_string(&request.change_note, 7999, "更新说明")?;
    if request.update_metadata && request.title.trim().is_empty() {
        return Err("标题不能为空。".into());
    }
    fs::create_dir_all(cache).map_err(err)?;
    let stage = tempfile::Builder::new()
        .prefix("upload-")
        .tempdir_in(cache)
        .map_err(err)?;
    let mut stats = (0, 0, 0);
    let mut version = String::new();
    let mut compatible = String::new();
    let mut warnings = Vec::new();
    if request.update_content {
        let source = PathBuf::from(&request.folder).canonicalize().map_err(err)?;
        if source.parent().is_none() || !source.join("modinfo.lua").is_file() {
            return Err("必须选择含 modinfo.lua 的模组目录，不能选择磁盘根目录。".into());
        }
        let stage_path = stage.path().canonicalize().map_err(err)?;
        if stage_path.starts_with(&source) {
            return Err("暂存目录位于源目录中，请选择独立的模组文件夹。".into());
        }
        copy_tree(
            &source,
            &stage.path().join("content"),
            true,
            request.redundant,
            &mut stats,
            0,
        )?;
        let info = crate::modinfo::read(&stage.path().join("content"))?;
        version = info.version.trim().to_string();
        compatible = info.version_compatible.trim().to_string();
        warnings.extend(info.warnings);
        if version.is_empty() {
            return Err(
                "不能安全读取 modinfo.lua 的独立 version 字符串，请先修正再上传文件。".into(),
            );
        }
        safe_string(&version, 128, "版本")?;
        safe_string(&compatible, 128, "兼容版本")?;
        if request.app_id == 322330 {
            manifest(&stage.path().join("content"))?;
        } else {
            package_ds(
                &stage.path().join("content"),
                &stage.path().join("mod_publish_data_file.zip"),
            )?;
        }
    }
    if request.update_preview {
        let source = PathBuf::from(&request.preview);
        let ext = source
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["png", "jpg", "jpeg", "gif"].contains(&ext.as_str()) {
            return Err("封面仅支持 PNG/JPG/GIF。".into());
        }
        let metadata = fs::metadata(&source).map_err(|_| "请选择存在的封面图片。")?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 1024 * 1024 {
            return Err("封面必须为非空图片且不超过 1MiB。".into());
        }
        let mut bytes = Vec::new();
        fs::File::open(&source)
            .map_err(err)?
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        if bytes.len() > 1024 * 1024
            || !(bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                || bytes.starts_with(b"\xff\xd8\xff")
                || bytes.starts_with(b"GIF87a")
                || bytes.starts_with(b"GIF89a"))
        {
            return Err("封面格式头无效或超过 1MiB。".into());
        }
        request.preview = stage
            .path()
            .join(format!("preview.{ext}"))
            .to_string_lossy()
            .to_string();
        fs::write(&request.preview, bytes).map_err(err)?;
    }
    let tags = tags(&request.tags, &version, &compatible)?;
    let token = format!(
        "{:x}",
        Sha256::digest(stage.path().to_string_lossy().as_bytes())
    );
    let summary = Summary {
        token,
        app_id: request.app_id,
        id: request.id.clone(),
        title: request.title.clone(),
        files: stats.0,
        bytes: stats.1,
        skipped: stats.2,
        version: version.clone(),
        previous_version: String::new(),
        update_content: request.update_content,
        update_preview: request.update_preview,
        update_metadata: request.update_metadata,
        visibility: request.visibility,
        warnings,
    };
    Ok(Plan {
        request,
        summary,
        stage,
        version,
        compatible,
        tags,
        steam_id: String::new(),
        created: std::time::Instant::now(),
    })
}
impl Plan {
    pub fn validate_version(&self, previous: &str) -> Result<(), String> {
        if self.request.id != "0"
            && self.request.update_content
            && !previous.is_empty()
            && normalized(previous) == normalized(&self.version)
        {
            return Err(format!("本地与线上版本相同（{}），禁止上传模组文件。请在本地版本编辑区修改并保存 modinfo.lua，再重新准备。", self.version));
        }
        Ok(())
    }
    pub fn check_version(&mut self, previous: &str) -> Result<(), String> {
        self.validate_version(previous)?;
        self.summary.previous_version = previous.to_string();
        if self.needs_force() {
            self.summary
                .warnings
                .push("线上版本未知，请核对后明确确认。".into());
        }
        Ok(())
    }
    pub fn needs_force(&self) -> bool {
        self.request.id != "0"
            && self.request.update_content
            && self.summary.previous_version.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(folder: &Path) -> Request {
        Request {
            app_id: 322330,
            id: "123".into(),
            folder: folder.to_string_lossy().into(),
            preview: String::new(),
            title: "测试".into(),
            description: String::new(),
            tags: "Utility,utility,version:old".into(),
            visibility: 0,
            change_note: String::new(),
            update_content: true,
            update_preview: false,
            update_metadata: false,
            redundant: false,
        }
    }
    #[test]
    fn ds_zip_root_snapshot_and_version_guard() {
        use std::io::Read;
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        fs::write(source.path().join("modinfo.lua"), "version='2.1'").unwrap();
        fs::create_dir(source.path().join("scripts")).unwrap();
        fs::write(source.path().join("scripts/中文.lua"), "return 42").unwrap();
        fs::write(source.path().join("mod.manifest"), "stale").unwrap();
        let mut r = request(source.path());
        r.app_id = 219740;
        let mut p = prepare(r, cache.path()).unwrap();
        assert_eq!(p.summary.app_id, 219740);
        let file = fs::File::open(p.stage.path().join("mod_publish_data_file.zip")).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        assert_eq!(zip.len(), 2);
        assert!(zip.by_name("mod.manifest").is_err());
        let mut data = String::new();
        zip.by_name("scripts/中文.lua")
            .unwrap()
            .read_to_string(&mut data)
            .unwrap();
        assert_eq!(data, "return 42");
        assert!(p.check_version(" V2.1 ").is_err());
        assert!(p.check_version("2.0").is_ok());
        let path = p.stage.path().to_path_buf();
        drop(zip);
        drop(p);
        assert!(!path.exists());
    }
    #[test]
    fn staged_snapshot_and_manifest() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("modinfo.lua"),
            "name='中文'\nversion='1.2'\nauthor='A'\ndescription='D'",
        )
        .unwrap();
        fs::write(source.path().join("modmain.lua"), "return").unwrap();
        fs::write(source.path().join("art.png"), "source").unwrap();
        fs::write(source.path().join("mod.manifest"), "stale").unwrap();
        fs::create_dir(source.path().join(".git")).unwrap();
        let mut p = prepare(request(source.path()), cache.path()).unwrap();
        assert_eq!(p.summary.files, 2);
        assert_eq!(p.summary.skipped, 3);
        assert_eq!(p.tags, vec!["Utility", "version:1.2"]);
        let manifest = fs::read(p.stage.path().join("content/mod.manifest")).unwrap();
        assert_eq!(&manifest[..4], b"MNFS");
        assert_eq!(u32::from_le_bytes(manifest[8..12].try_into().unwrap()), 2);
        fs::write(source.path().join("modmain.lua"), "changed").unwrap();
        assert_eq!(
            fs::read_to_string(p.stage.path().join("content/modmain.lua")).unwrap(),
            "return"
        );
        assert!(p.check_version("v1.2").is_err());
        assert!(p.validate_version(" 1.2 ").is_err());
        assert!(p.check_version("1.1").is_ok());
        assert!(!p.needs_force());
        assert!(p.check_version("").is_ok());
        assert!(p.needs_force());
        p.request.update_content = false;
        assert!(p.validate_version("1.2").is_ok());
        p.request.update_content = true;
        p.request.id = "0".into();
        assert!(p.validate_version("1.2").is_ok());
        let path = p.stage.path().to_path_buf();
        drop(p);
        assert!(!path.exists());
    }
    #[test]
    fn rejects_unsupported_missing_version_and_noop() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut r = request(source.path());
        r.app_id = 42;
        assert!(prepare(r, cache.path()).is_err());
        fs::write(source.path().join("modinfo.lua"), "version='1.'..value").unwrap();
        assert!(prepare(request(source.path()), cache.path()).is_err());
        let mut r = request(source.path());
        r.update_content = false;
        assert!(prepare(r, cache.path()).is_err());
    }
    #[test]
    fn rejects_invalid_preview_and_nul() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut r = request(source.path());
        r.update_content = false;
        r.update_preview = true;
        r.preview = source.path().join("fake.png").to_string_lossy().into();
        fs::write(&r.preview, "not image").unwrap();
        assert!(prepare(r, cache.path()).is_err());
        let mut r = request(source.path());
        r.title = "bad\0title".into();
        assert!(prepare(r, cache.path()).is_err());
    }
    #[test]
    fn all_standard_tags_survive_and_versions_are_rebuilt() {
        let values = "art,character,creature,environment,interface,item,language,other,pet,scenario,tutorial,tweak,utility,worldgen,server_admin";
        let actual = tags(
            &format!("{values},ART,custom,version:old,version_compatible=old"),
            "2.0",
            "1.0",
        )
        .unwrap();
        assert_eq!(actual.len(), 18);
        for value in values.split(',') {
            assert!(actual.contains(&value.to_string()));
        }
        assert!(actual.contains(&"custom".to_string()));
        assert!(actual.contains(&"version:2.0".to_string()));
        assert!(actual.contains(&"version_compatible:1.0".to_string()));
        assert!(!actual.contains(&"ART".to_string()));
        assert!(!actual.contains(&"version:old".to_string()));
    }
}
