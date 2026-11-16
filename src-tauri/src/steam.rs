//! x86 Steamworks adapter. All access is serialized by the command boundary.
pub mod deletion;
mod publishing;
mod remote;
use libloading::Library;
pub use publishing::{preflight, publish};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_void},
    path::Path,
    thread,
    time::{Duration, Instant},
};

type Ptr = *mut c_void;
macro_rules! api {
    ($($name:ident: ($($arg:ty),*) -> $ret:ty),* $(,)?) => {
        #[allow(non_snake_case)]
        struct Api { $($name: unsafe extern "C" fn($($arg),*) -> $ret,)* _library: Library }
        impl Api {
            unsafe fn load(path: &Path) -> Result<Self, String> {
                let library = unsafe { Library::new(path) }.map_err(|e| format!("无法加载 {}：{e}。请使用 x86 构建。", path.display()))?;
                Ok(Self { $($name: *unsafe { library.get(concat!(stringify!($name), "\0").as_bytes()) }.map_err(|e| e.to_string())?,)* _library: library })
            }
        }
    }
}
api! {
    SteamAPI_Init: () -> bool,
    SteamAPI_Shutdown: () -> (),
    SteamAPI_RunCallbacks: () -> (),
    SteamAPI_IsSteamRunning: () -> bool,
    SteamClient: () -> Ptr,
    SteamAPI_GetHSteamUser: () -> i32,
    SteamAPI_GetHSteamPipe: () -> i32,
    SteamAPI_ISteamClient_GetISteamUser: (Ptr, i32, i32, *const c_char) -> Ptr,
    SteamAPI_ISteamClient_GetISteamUtils: (Ptr, i32, *const c_char) -> Ptr,
    SteamAPI_ISteamClient_GetISteamUGC: (Ptr, i32, i32, *const c_char) -> Ptr,
    SteamAPI_ISteamUser_BLoggedOn: (Ptr) -> bool,
    SteamAPI_ISteamUser_GetSteamID: (Ptr) -> u64,
    SteamAPI_ISteamUGC_CreateQueryUserUGCRequest: (Ptr, u32, i32, i32, i32, u32, u32, u32) -> u64,
    SteamAPI_ISteamUGC_SetReturnMetadata: (Ptr, u64, bool) -> bool,
    SteamAPI_ISteamUGC_SetReturnLongDescription: (Ptr, u64, bool) -> bool,
    SteamAPI_ISteamUGC_SetLanguage: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamUGC_SendQueryUGCRequest: (Ptr, u64) -> u64,
    SteamAPI_ISteamUGC_GetQueryUGCResult: (Ptr, u64, u32, *mut Details) -> bool,
    SteamAPI_ISteamUGC_GetQueryUGCMetadata: (Ptr, u64, u32, *mut u8, u32) -> bool,
    SteamAPI_ISteamUGC_GetQueryUGCPreviewURL: (Ptr, u64, u32, *mut u8, u32) -> bool,
    SteamAPI_ISteamUGC_ReleaseQueryUGCRequest: (Ptr, u64) -> bool,
    SteamAPI_ISteamUtils_IsAPICallCompleted: (Ptr, u64, *mut bool) -> bool,
    SteamAPI_ISteamUtils_GetAPICallResult: (Ptr, u64, *mut c_void, i32, i32, *mut bool) -> bool,
    SteamAPI_ISteamUGC_CreateQueryUGCDetailsRequest: (Ptr, *const u64, u32) -> u64,
    SteamAPI_ISteamUGC_CreateItem: (Ptr, u32, i32) -> u64,
    SteamAPI_ISteamUGC_StartItemUpdate: (Ptr, u32, u64) -> u64,
    SteamAPI_ISteamUGC_SetItemTitle: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamUGC_SetItemDescription: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamUGC_SetItemMetadata: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamUGC_SetItemContent: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamUGC_SetItemPreview: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamUGC_SetItemTags: (Ptr, u64, *const publishing::StringArray) -> bool,
    SteamAPI_ISteamUGC_SetItemVisibility: (Ptr, u64, i32) -> bool,
    SteamAPI_ISteamUGC_SubmitItemUpdate: (Ptr, u64, *const c_char) -> u64,
    SteamAPI_ISteamUGC_GetItemUpdateProgress: (Ptr, u64, *mut u64, *mut u64) -> i32,
    SteamAPI_ISteamUGC_DeleteItem: (Ptr, u64) -> u64,
    SteamAPI_ISteamClient_GetISteamRemoteStorage: (Ptr, i32, i32, *const c_char) -> Ptr,
    SteamAPI_ISteamRemoteStorage_FileWrite: (Ptr, *const c_char, *const u8, i32) -> bool,
    SteamAPI_ISteamRemoteStorage_FileExists: (Ptr, *const c_char) -> bool,
    SteamAPI_ISteamRemoteStorage_GetFileSize: (Ptr, *const c_char) -> i32,
    SteamAPI_ISteamRemoteStorage_FileShare: (Ptr, *const c_char) -> u64,
    SteamAPI_ISteamRemoteStorage_PublishWorkshopFile: (Ptr, *const c_char, *const c_char, u32, *const c_char, *const c_char, i32, *const publishing::StringArray, i32) -> u64,
    SteamAPI_ISteamRemoteStorage_CreatePublishedFileUpdateRequest: (Ptr, u64) -> u64,
    SteamAPI_ISteamRemoteStorage_UpdatePublishedFileFile: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamRemoteStorage_UpdatePublishedFilePreviewFile: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamRemoteStorage_UpdatePublishedFileTitle: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamRemoteStorage_UpdatePublishedFileDescription: (Ptr, u64, *const c_char) -> bool,
    SteamAPI_ISteamRemoteStorage_UpdatePublishedFileVisibility: (Ptr, u64, i32) -> bool,
    SteamAPI_ISteamRemoteStorage_UpdatePublishedFileTags: (Ptr, u64, *const publishing::StringArray) -> bool,
    SteamAPI_ISteamRemoteStorage_CommitPublishedFileUpdate: (Ptr, u64) -> u64,
    SteamAPI_ISteamRemoteStorage_DeletePublishedFile: (Ptr, u64) -> u64,
}

#[repr(C, align(8))]
#[derive(Default)]
struct QueryCompleted {
    handle: u64,
    result: i32,
    returned: u32,
    total: u32,
    cached: bool,
}

#[repr(C, align(8))]
struct Details {
    id: u64,
    result: i32,
    file_type: i32,
    creator: u32,
    consumer: u32,
    title: [u8; 129],
    description: [u8; 8000],
    owner: u64,
    created: u32,
    updated: u32,
    added: u32,
    visibility: i32,
    banned: bool,
    accepted: bool,
    tags_truncated: bool,
    tags: [u8; 1025],
    file: u64,
    preview: u64,
    filename: [u8; 260],
    file_size: i32,
    preview_size: i32,
    url: [u8; 256],
    votes_up: u32,
    votes_down: u32,
    score: f32,
    children: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub title_english: String,
    pub description_english: String,
    pub tags: Vec<String>,
    pub version: String,
    pub preview_url: String,
    pub updated: u32,
    pub visibility: i32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub steam_id: String,
    pub items: Vec<WorkshopItem>,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub steam_id: String,
    pub online: bool,
}

struct Session {
    api: Api,
    user: Ptr,
    utils: Ptr,
    ugc: Ptr,
}
impl Session {
    fn open(path: &Path) -> Result<Self, String> {
        if cfg!(not(target_arch = "x86")) {
            return Err("当前 DLL 为 x86，请用 npm run desktop:dev 启动 32 位程序。".into());
        }
        // SAFETY: symbols and layouts mirror the Steamworks C API.
        // All callers hold the process-wide Steam mutex; the library outlives all handles.
        unsafe {
            let api = Api::load(path)?;
            if !(api.SteamAPI_IsSteamRunning)() {
                return Err("Steam 客户端未运行，请打开 Steam 并登录。".into());
            }
            if !(api.SteamAPI_Init)() {
                return Err(
                    "Steam 初始化失败。请确认已登录并拥有 Don't Starve Mod Tools（245850）。"
                        .into(),
                );
            }
            let mut session = Self {
                api,
                user: std::ptr::null_mut(),
                utils: std::ptr::null_mut(),
                ugc: std::ptr::null_mut(),
            };
            let a = &session.api;
            let client = (a.SteamClient)();
            let user = (a.SteamAPI_GetHSteamUser)();
            let pipe = (a.SteamAPI_GetHSteamPipe)();
            if client.is_null() || user == 0 || pipe == 0 {
                return Err("无法获取 Steam 客户端句柄。".into());
            }
            session.user = (a.SteamAPI_ISteamClient_GetISteamUser)(
                client,
                user,
                pipe,
                c"SteamUser019".as_ptr(),
            );
            session.utils =
                (a.SteamAPI_ISteamClient_GetISteamUtils)(client, pipe, c"SteamUtils009".as_ptr());
            session.ugc = (a.SteamAPI_ISteamClient_GetISteamUGC)(
                client,
                user,
                pipe,
                c"STEAMUGC_INTERFACE_VERSION010".as_ptr(),
            );
            if session.user.is_null() || session.utils.is_null() || session.ugc.is_null() {
                return Err("所需的 Steamworks 接口版本不可用。".into());
            }
            if !(session.api.SteamAPI_ISteamUser_BLoggedOn)(session.user) {
                return Err("Steam 尚未在线登录。".into());
            }
            Ok(session)
        }
    }
    fn id(&self) -> u64 {
        unsafe { (self.api.SteamAPI_ISteamUser_GetSteamID)(self.user) }
    }
    fn wait(&self, call: u64) -> Result<QueryCompleted, String> {
        if call == 0 {
            return Err("Steam 没有返回有效的查询句柄。".into());
        }
        let start = Instant::now();
        loop {
            unsafe {
                (self.api.SteamAPI_RunCallbacks)();
                let mut failed = false;
                if (self.api.SteamAPI_ISteamUtils_IsAPICallCompleted)(self.utils, call, &mut failed)
                {
                    if failed {
                        return Err("Steam 查询通信失败。".into());
                    }
                    let mut result = QueryCompleted::default();
                    let ok = (self.api.SteamAPI_ISteamUtils_GetAPICallResult)(
                        self.utils,
                        call,
                        &mut result as *mut _ as Ptr,
                        size_of::<QueryCompleted>() as i32,
                        3401,
                        &mut failed,
                    );
                    if !ok || failed {
                        return Err("Steam 查询回调读取失败。".into());
                    }
                    if result.result != 1 {
                        return Err(format!("Steam 查询失败，EResult={}。", result.result));
                    }
                    return Ok(result);
                }
            }
            if start.elapsed() > Duration::from_secs(30) {
                return Err("Steam 查询超时（30 秒），请稍后重试。".into());
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe { (self.api.SteamAPI_Shutdown)() }
    }
}
struct Query<'a> {
    session: &'a Session,
    handle: u64,
}
impl Drop for Query<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.session.api.SteamAPI_ISteamUGC_ReleaseQueryUGCRequest)(
                self.session.ugc,
                self.handle,
            );
        }
    }
}
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())])
        .into_owned()
}
fn version(metadata: &str, tags: &[String]) -> String {
    metadata
        .lines()
        .chain(tags.iter().map(String::as_str))
        .find_map(|line| {
            let (key, value) = line.split_once(':').or_else(|| line.split_once('='))?;
            key.trim()
                .eq_ignore_ascii_case("version")
                .then(|| value.trim().trim_matches(['\'', '"']).to_string())
        })
        .unwrap_or_default()
}
pub fn status(path: &Path) -> Result<Status, String> {
    let s = Session::open(path)?;
    Ok(Status {
        steam_id: s.id().to_string(),
        online: true,
    })
}
pub fn list(path: &Path, app_id: u32, log: impl Fn(&str)) -> Result<QueryResult, String> {
    if ![322330, 219740].contains(&app_id) {
        return Err("仅支持饥荒及饥荒联机版。".into());
    }
    let s = Session::open(path)?;
    let mut items: BTreeMap<u64, WorkshopItem> = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut successful_queries = 0;
    // The tool publishes under creator AppID 245850 for both supported games.
    // English fills localized fallback fields; an Items miss retries with All.
    for creator in [245850] {
        for (language, english) in [(c"schinese", false), (c"english", true)] {
            for matching in [0, -1] {
                let before = items.len();
                for page in 1..=20 {
                    log(&format!(
                        "查询 creator={creator} / type={matching} / {} / 第 {page} 页",
                        if english { "English" } else { "简体中文" }
                    ));
                    let result = (|| -> Result<bool, String> {
                        unsafe {
                            let a = &s.api;
                            let handle = (a.SteamAPI_ISteamUGC_CreateQueryUserUGCRequest)(
                                s.ugc,
                                s.id() as u32,
                                0,
                                matching,
                                3,
                                creator,
                                app_id,
                                page,
                            );
                            if handle == 0 || handle == u64::MAX {
                                return Err("创建工坊查询失败。".into());
                            }
                            let query = Query {
                                session: &s,
                                handle,
                            };
                            (a.SteamAPI_ISteamUGC_SetReturnMetadata)(s.ugc, handle, true);
                            (a.SteamAPI_ISteamUGC_SetReturnLongDescription)(s.ugc, handle, true);
                            (a.SteamAPI_ISteamUGC_SetLanguage)(s.ugc, handle, language.as_ptr());
                            let completed =
                                s.wait((a.SteamAPI_ISteamUGC_SendQueryUGCRequest)(s.ugc, handle))?;
                            successful_queries += 1;
                            for i in 0..completed.returned {
                                let mut details: Details = std::mem::zeroed();
                                if !(a.SteamAPI_ISteamUGC_GetQueryUGCResult)(
                                    s.ugc,
                                    handle,
                                    i,
                                    &mut details,
                                ) || details.result != 1
                                    || details.id == 0
                                {
                                    continue;
                                }
                                let mut metadata = [0u8; 5000];
                                let mut preview = [0u8; 2048];
                                (a.SteamAPI_ISteamUGC_GetQueryUGCMetadata)(
                                    s.ugc,
                                    handle,
                                    i,
                                    metadata.as_mut_ptr(),
                                    metadata.len() as u32,
                                );
                                (a.SteamAPI_ISteamUGC_GetQueryUGCPreviewURL)(
                                    s.ugc,
                                    handle,
                                    i,
                                    preview.as_mut_ptr(),
                                    preview.len() as u32,
                                );
                                let tags: Vec<String> = text(&details.tags)
                                    .split(',')
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                    .map(String::from)
                                    .collect();
                                if details.tags_truncated {
                                    warnings.push(format!(
                                        "物品 {} 的 Tags 过长被查询接口截断。",
                                        details.id
                                    ));
                                }
                                let title = text(&details.title);
                                let description = text(&details.description);
                                let entry =
                                    items.entry(details.id).or_insert_with(|| WorkshopItem {
                                        id: details.id.to_string(),
                                        title: title.clone(),
                                        description: description.clone(),
                                        title_english: String::new(),
                                        description_english: String::new(),
                                        tags: tags.clone(),
                                        version: version(&text(&metadata), &tags),
                                        preview_url: text(&preview),
                                        updated: details.updated,
                                        visibility: details.visibility,
                                    });
                                if english {
                                    entry.title_english = title;
                                    entry.description_english = description;
                                } else {
                                    if !title.is_empty() {
                                        entry.title = title;
                                    }
                                    if !description.is_empty() {
                                        entry.description = description;
                                    }
                                }
                            }
                            drop(query);
                            if page == 20 && page * 50 < completed.total {
                                warnings.push("超过 1000 个物品，当前列表仅显示前 20 页。".into());
                            }
                            Ok(completed.returned == 0 || page * 50 >= completed.total)
                        }
                    })();
                    match result {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(e) => {
                            log(&e);
                            warnings.push(e);
                            break;
                        }
                    }
                }
                if matching == 0 && items.len() > before {
                    break;
                }
            }
        }
    }
    if successful_queries == 0 {
        return Err(warnings
            .first()
            .cloned()
            .unwrap_or_else(|| "全部工坊查询失败。".into()));
    }
    let mut items: Vec<_> = items.into_values().collect();
    items.sort_by(|a, b| b.updated.cmp(&a.updated).then(a.id.cmp(&b.id)));
    warnings.sort();
    warnings.dedup();
    Ok(QueryResult {
        steam_id: s.id().to_string(),
        items,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_abi_layout() {
        assert_eq!(size_of::<QueryCompleted>(), 24);
        assert_eq!(std::mem::offset_of!(Details, owner), 8160);
        assert_eq!(std::mem::offset_of!(Details, tags), 8187);
        assert_eq!(std::mem::offset_of!(Details, file), 9216);
        assert_eq!(size_of::<Details>(), 9776);
    }
    #[test]
    fn utf8_and_version() {
        assert_eq!(text("中文\0剩余".as_bytes()), "中文");
        assert_eq!(version("version:1.2\nversion_compatible:1.0", &[]), "1.2");
        assert_eq!(version("", &["version=2.0".into()]), "2.0");
    }
}
