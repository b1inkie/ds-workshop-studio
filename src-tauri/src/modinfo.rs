//! Conservative literal reader: never executes untrusted Lua from a mod folder.
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    pub folder: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub version_compatible: String,
    pub warnings: Vec<String>,
    pub file_hash: String,
}

fn long_open(bytes: &[u8], i: usize) -> Option<(usize, usize)> {
    if bytes.get(i) != Some(&b'[') {
        return None;
    }
    let mut j = i + 1;
    while bytes.get(j) == Some(&b'=') {
        j += 1;
    }
    (bytes.get(j) == Some(&b'[')).then_some((j - i - 1, j + 1))
}
fn long_end(bytes: &[u8], from: usize, equals: usize) -> usize {
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b']'
            && bytes
                .get(i + 1..i + 1 + equals)
                .is_some_and(|s| s.iter().all(|b| *b == b'='))
            && bytes.get(i + 1 + equals) == Some(&b']')
        {
            return i;
        }
        i += 1;
    }
    bytes.len()
}
fn skip(bytes: &[u8], mut i: usize) -> usize {
    loop {
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if bytes.get(i..i + 2) != Some(b"--") {
            return i;
        }
        i += 2;
        if let Some((eq, start)) = long_open(bytes, i) {
            i = (long_end(bytes, start, eq) + eq + 2).min(bytes.len());
        } else {
            while bytes.get(i).is_some_and(|b| *b != b'\n') {
                i += 1;
            }
        }
    }
}
fn quoted(bytes: &[u8], i: usize) -> Option<(String, usize)> {
    let quote = *bytes.get(i)?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    let mut j = i + 1;
    let mut output = Vec::new();
    while j < bytes.len() {
        match bytes[j] {
            b if b == quote => return Some((String::from_utf8_lossy(&output).into_owned(), j + 1)),
            b'\\' => {
                j += 1;
                let escaped = *bytes.get(j)?;
                output.push(match escaped {
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    other => other,
                });
            }
            b => output.push(b),
        }
        j += 1;
    }
    None
}
fn ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
fn literals(lua: &str, key: &str) -> Vec<(String, std::ops::Range<usize>)> {
    let bytes = lua.as_bytes();
    let mut i = 0;
    let mut depth: usize = 0;
    let mut blocks: usize = 0;
    let mut found = Vec::new();
    while i < bytes.len() {
        i = skip(bytes, i);
        if i >= bytes.len() {
            break;
        }
        if let Some((_, next)) = quoted(bytes, i) {
            i = next;
            continue;
        }
        if let Some((eq, start)) = long_open(bytes, i) {
            i = (long_end(bytes, start, eq) + eq + 2).min(bytes.len());
            continue;
        }
        match bytes[i] {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if ident(bytes[i]) {
            let start = i;
            while bytes.get(i).is_some_and(|b| ident(*b)) {
                i += 1;
            }
            match &lua[start..i] {
                "function" | "then" | "do" | "repeat" => {
                    blocks += 1;
                    continue;
                }
                "end" | "until" => {
                    blocks = blocks.saturating_sub(1);
                    continue;
                }
                "elseif" => {
                    blocks = blocks.saturating_sub(1);
                    continue;
                }
                _ => {}
            }
            let line_start = lua[..start].rfind('\n').map_or(0, |n| n + 1);
            // Only bare top-level line assignments. Conditionals/functions and expressions
            // are intentionally not evaluated; missing fields surface as warnings.
            if depth != 0
                || blocks != 0
                || &lua[start..i] != key
                || !lua[line_start..start].trim().is_empty()
            {
                continue;
            }
            let eq = skip(bytes, i);
            if bytes.get(eq) != Some(&b'=') || bytes.get(eq + 1) == Some(&b'=') {
                continue;
            }
            let start = skip(bytes, eq + 1);
            let literal = if let Some(value) = quoted(bytes, start) {
                Some(value)
            } else if let Some((eq, from)) = long_open(bytes, start) {
                let end = long_end(bytes, from, eq);
                (end < bytes.len()).then(|| {
                    (
                        lua[from..end].trim_start_matches(['\r', '\n']).to_string(),
                        end + eq + 2,
                    )
                })
            } else {
                None
            };
            if let Some((value, next)) = literal {
                let tail = lua[next..].trim_start_matches([' ', '\t', '\r']);
                if tail.is_empty()
                    || tail.starts_with('\n')
                    || tail.starts_with("--")
                    || tail.starts_with(';')
                {
                    found.push((value, start..next));
                }
            }
        } else {
            i += 1;
        }
    }
    found
}
pub fn field(lua: &str, key: &str) -> Option<String> {
    literals(lua, key)
        .into_iter()
        .next()
        .map(|(value, _)| value)
}

pub fn read(folder: &Path) -> Result<ModInfo, String> {
    let folder = folder
        .canonicalize()
        .map_err(|e| format!("模组目录不可用：{e}"))?;
    if !folder.is_dir() {
        return Err("请选择模组文件夹。".into());
    }
    let path = folder.join("modinfo.lua");
    if fs::metadata(&path)
        .map_err(|_| "目录内没有 modinfo.lua。")?
        .len()
        > 2 * 1024 * 1024
    {
        return Err("modinfo.lua 超过 2MB，拒绝读取。".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let file_hash = file_hash(&bytes);
    let lua =
        std::str::from_utf8(&bytes).map_err(|e| format!("无法以 UTF-8 读取 modinfo.lua：{e}"))?;
    let lua = lua.trim_start_matches('\u{feff}');
    let mut warnings = Vec::new();
    let mut get = |key: &str| {
        field(lua, key).unwrap_or_else(|| {
            warnings.push(format!("未能静态读取 {key}（可能使用了变量或多语言条件），请在工具中手动核对；不会执行 Lua。"));
            String::new()
        })
    };
    Ok(ModInfo {
        folder: folder
            .to_string_lossy()
            .trim_start_matches("\\\\?\\")
            .to_string(),
        name: get("name"),
        description: get("description"),
        author: get("author"),
        version: get("version"),
        version_compatible: field(lua, "version_compatible").unwrap_or_default(),
        warnings,
        file_hash,
    })
}

fn file_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSaved {
    pub info: ModInfo,
    pub backup: String,
}
fn version_replacement(lua: &str, value: &str) -> Result<String, String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
    {
        return Err("版本号限 1–64 个英文字母、数字及 . _ + -，例如 2.0.1。".into());
    }
    // Count assignment candidates outside comments/strings, including conditional assignments.
    // Refuse ambiguity instead of changing just one of several runtime branches.
    let bytes = lua.as_bytes();
    let mut i = 0;
    let mut assignments = 0;
    while i < bytes.len() {
        i = skip(bytes, i);
        if i >= bytes.len() {
            break;
        }
        if let Some((_, next)) = quoted(bytes, i) {
            i = next;
            continue;
        }
        if let Some((eq, start)) = long_open(bytes, i) {
            i = (long_end(bytes, start, eq) + eq + 2).min(bytes.len());
            continue;
        }
        if ident(bytes[i]) {
            let start = i;
            while bytes.get(i).is_some_and(|b| ident(*b)) {
                i += 1;
            }
            let eq = skip(bytes, i);
            if &lua[start..i] == "version"
                && bytes.get(eq) == Some(&b'=')
                && bytes.get(eq + 1) != Some(&b'=')
            {
                assignments += 1;
            }
        } else {
            i += 1;
        }
    }
    let matches = literals(lua.trim_start_matches('\u{feff}'), "version");
    if assignments != 1 || matches.len() != 1 {
        return Err("version 不是唯一的顶层独立字符串赋值，无法安全修改；请用编辑器处理动态或多分支版本定义。".into());
    }
    let bom = lua.len() - lua.trim_start_matches('\u{feff}').len();
    let range = &matches[0].1;
    let mut output = lua.to_string();
    output.replace_range(range.start + bom..range.end + bom, &format!("\"{value}\""));
    Ok(output)
}
pub fn save_version(
    folder: &Path,
    value: &str,
    expected_hash: &str,
) -> Result<VersionSaved, String> {
    use std::io::Write;
    let folder = folder.canonicalize().map_err(|e| e.to_string())?;
    let path = folder.join("modinfo.lua");
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("拒绝修改链接或重解析点文件。".into());
        }
    }
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 2 * 1024 * 1024
    {
        return Err("modinfo.lua 不是可安全修改的普通文件。".into());
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    if expected_hash.is_empty() || file_hash(&bytes) != expected_hash {
        return Err("modinfo.lua 已变化，请重新读取后再保存版本。".into());
    }
    let lua = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let output = version_replacement(lua, value)?;
    if output.as_bytes() == bytes {
        return Err("新版本与当前文件相同，无需保存。".into());
    }
    let backups = folder.join(".workshop-studio-backups");
    if !backups.exists() {
        fs::create_dir(&backups).map_err(|e| e.to_string())?;
    }
    let meta = fs::symlink_metadata(&backups).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err("备份目录不能是重解析点。".into());
        }
    }
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("备份目录不安全。".into());
    }
    let mut backup = tempfile::Builder::new()
        .prefix("modinfo-")
        .suffix(".lua.bak")
        .tempfile_in(&backups)
        .map_err(|e| e.to_string())?;
    backup.write_all(&bytes).map_err(|e| e.to_string())?;
    backup.as_file().sync_all().map_err(|e| e.to_string())?;
    let (_, backup_path) = backup.keep().map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&folder).map_err(|e| e.to_string())?;
    temp.write_all(output.as_bytes())
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    if fs::read(&path).map_err(|e| e.to_string())? != bytes {
        return Err("保存前文件又发生变化，已中止；请重新读取。".into());
    }
    temp.persist(&path)
        .map_err(|e| format!("无法保存版本（原文件已备份）：{e}"))?;
    Ok(VersionSaved {
        info: read(&folder)?,
        backup: backup_path
            .to_string_lossy()
            .trim_start_matches("\\\\?\\")
            .into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_utf8_literals() {
        assert_eq!(
            field("name = \"中文\\n模组\"", "name").as_deref(),
            Some("中文\n模组")
        );
        assert_eq!(
            field("description = [=[\n多行\n内容]=]", "description").as_deref(),
            Some("多行\n内容")
        );
    }
    #[test]
    fn skips_comments_nested_fields_and_strings() {
        let lua = "-- name='错误'\n--[=[name='也错']=]\nx = {\nname='嵌套'\n}\ny='name=假'\nname = '正确'";
        assert_eq!(field(lua, "name").as_deref(), Some("正确"));
    }
    #[test]
    fn refuses_dynamic_expression() {
        assert_eq!(
            field("name = GLOBAL.language\nversion = '1.' .. value", "name"),
            None
        );
        assert_eq!(field("version = '1.' .. value", "version"), None);
    }
    #[test]
    fn skips_functions_and_conditionals() {
        let lua = "if chinese then\nname='条件值'\nend\nfunction helper()\nname='函数值'\nend\nname='独立值'";
        assert_eq!(field(lua, "name").as_deref(), Some("独立值"));
        assert_eq!(
            field(
                "if chinese then\nname='中文'\nelse\nname='English'\nend",
                "name"
            ),
            None
        );
    }
    #[test]
    fn rejects_missing_folder() {
        assert!(read(Path::new("Z:/nonexistent-workshop-test")).is_err());
    }
    #[test]
    fn version_edit_preserves_file_and_rejects_ambiguity() {
        let source = "\u{feff}-- version='fake'\r\nname='中文'\r\nversion = '2.0.0' -- 注释\r\nversion_compatible='1.0'\r\n";
        let edited = version_replacement(source, "2.0.1").unwrap();
        assert_eq!(edited, source.replace("'2.0.0'", "\"2.0.1\""));
        for lua in [
            "version='1.' .. build",
            "version='1'\nversion='2'",
            "if x then\nversion='1'\nend",
            "version='1'\nif x then\nversion='2'\nend",
        ] {
            assert!(version_replacement(lua, "2").is_err());
        }
        for value in ["", "1\n2", "1\";os.execute('x')", "版本2"] {
            assert!(version_replacement(source, value).is_err());
        }
    }
    #[test]
    fn version_save_backup_and_conflict() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("modinfo.lua");
        let source = "name='中文'\r\nversion='2.0.0'\r\ndescription='D'\r\nauthor='A'\r\n";
        fs::write(&path, source).unwrap();
        let info = read(folder.path()).unwrap();
        let saved = save_version(folder.path(), "2.0.1", &info.file_hash).unwrap();
        assert_eq!(saved.info.version, "2.0.1");
        assert_eq!(fs::read_to_string(saved.backup).unwrap(), source);
        assert!(save_version(folder.path(), "2.0.2", &info.file_hash).is_err());
        assert_eq!(read(folder.path()).unwrap().version, "2.0.1");
    }
}
