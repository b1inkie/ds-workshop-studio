#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod modinfo;
mod steam;
mod updater;
mod upload;
use std::{path::PathBuf, sync::Mutex};
use tauri::{Emitter, Manager};

static STEAM_LOCK: Mutex<()> = Mutex::new(());
static PREPARED: Mutex<Option<upload::Plan>> = Mutex::new(None);
static PUBLISHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DELETE_PREPARED: Mutex<Option<steam::deletion::Plan>> = Mutex::new(None);
struct ActiveWrite;
impl ActiveWrite {
    fn begin() -> Self {
        PUBLISHING.store(true, std::sync::atomic::Ordering::SeqCst);
        Self
    }
}
impl Drop for ActiveWrite {
    fn drop(&mut self) {
        PUBLISHING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
#[tauri::command]
async fn prepare_delete(
    app: tauri::AppHandle,
    app_id: u32,
    id: String,
) -> Result<steam::deletion::Summary, String> {
    let path = dll_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STEAM_LOCK
            .try_lock()
            .map_err(|_| "Steam 正忙，请等待当前操作。")?;
        let mut pending = DELETE_PREPARED.lock().map_err(|e| e.to_string())?;
        *pending = None;
        *PREPARED.lock().map_err(|e| e.to_string())? = None;
        let plan = steam::deletion::prepare(&path, app_id, &id)?;
        let summary = plan.summary.clone();
        *pending = Some(plan);
        Ok(summary)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn discard_delete(token: String) -> Result<(), String> {
    let mut pending = DELETE_PREPARED.lock().map_err(|e| e.to_string())?;
    if pending.as_ref().is_some_and(|p| p.summary.token == token) {
        *pending = None;
    }
    Ok(())
}
#[tauri::command]
async fn submit_delete(
    app: tauri::AppHandle,
    token: String,
    confirmed_id: String,
) -> Result<steam::deletion::Outcome, String> {
    let path = dll_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STEAM_LOCK
            .try_lock()
            .map_err(|_| "Steam 正忙，请等待当前操作。")?;
        let mut pending = DELETE_PREPARED.lock().map_err(|e| e.to_string())?;
        let plan = steam::deletion::take_confirmed(&mut pending, &token, &confirmed_id)?;
        drop(pending);
        let _active = ActiveWrite::begin();
        let result = steam::deletion::submit(&path, &plan);
        let message = match &result.error {
            Some(error) => format!("删除物品 {} 未确认成功：{error}", result.id),
            None => format!(
                "Steam 已确认删除物品 {}。本地模组文件与草稿未删除。",
                result.id
            ),
        };
        let _ = app.emit("workshop-log", message);
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn prepare_upload(
    app: tauri::AppHandle,
    request: upload::Request,
) -> Result<upload::Summary, String> {
    let path = dll_path(&app)?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("upload-staging");
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STEAM_LOCK
            .try_lock()
            .map_err(|_| "Steam 正忙，请等待当前操作。")?;
        let mut pending = PREPARED.lock().map_err(|e| e.to_string())?;
        *pending = None;
        *DELETE_PREPARED.lock().map_err(|e| e.to_string())? = None;
        let mut plan = upload::prepare(request, &cache)?;
        steam::preflight(&path, &mut plan)?;
        let summary = plan.summary.clone();
        *pending = Some(plan);
        Ok(summary)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn discard_upload(token: String) -> Result<(), String> {
    let mut pending = PREPARED.lock().map_err(|e| e.to_string())?;
    if pending.as_ref().is_some_and(|p| p.summary.token == token) {
        *pending = None;
    }
    Ok(())
}
#[tauri::command]
async fn submit_upload(
    app: tauri::AppHandle,
    token: String,
    allow_overwrite: bool,
) -> Result<upload::Outcome, String> {
    let path = dll_path(&app)?;
    let journal = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("upload-history");
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STEAM_LOCK
            .try_lock()
            .map_err(|_| "Steam 正忙，请等待当前操作。")?;
        let mut pending = PREPARED.lock().map_err(|e| e.to_string())?;
        let plan = pending.as_ref().ok_or("没有待确认的上传，请重新准备。")?;
        if plan.summary.token != token || plan.created.elapsed().as_secs() > 600 {
            return Err("上传确认已失效，请重新准备。".into());
        }
        plan.validate_version(&plan.summary.previous_version)?;
        if plan.needs_force() && !allow_overwrite {
            return Err("请明确确认线上版本未知。".into());
        }
        let plan = pending.take().ok_or("上传已被消费。")?;
        drop(pending);
        let _active = ActiveWrite::begin();
        Ok(steam::publish(
            &path,
            &plan,
            allow_overwrite,
            &journal,
            |progress| {
                let _ = app.emit("upload-progress", progress);
            },
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn save_mod_version(
    folder: String,
    version: String,
    expected_hash: String,
) -> Result<modinfo::VersionSaved, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STEAM_LOCK
            .try_lock()
            .map_err(|_| "工坊正忙，请等待当前操作结束。")?;
        *PREPARED.lock().map_err(|e| e.to_string())? = None;
        modinfo::save_version(std::path::Path::new(&folder), &version, &expected_hash)
    })
    .await
    .map_err(|e| e.to_string())?
}
fn dll_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/steam_api.dll"));
    }
    Ok(app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("resources/steam_api.dll"))
}
#[tauri::command]
async fn steam_status(app: tauri::AppHandle) -> Result<steam::Status, String> {
    let path = dll_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STEAM_LOCK
            .try_lock()
            .map_err(|_| "Steam 正在查询，请等待当前操作结束。")?;
        steam::status(&path)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn list_workshop(app: tauri::AppHandle, app_id: u32) -> Result<steam::QueryResult, String> {
    let path = dll_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STEAM_LOCK
            .try_lock()
            .map_err(|_| "Steam 正在查询，请等待当前操作结束。")?;
        steam::list(&path, app_id, |message| {
            let _ = app.emit("workshop-log", message);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn read_mod_info(folder: String) -> Result<modinfo::ModInfo, String> {
    tauri::async_runtime::spawn_blocking(move || modinfo::read(std::path::Path::new(&folder)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
#[tauri::command]
async fn check_update() -> Result<updater::UpdateCheck, String> {
    tauri::async_runtime::spawn_blocking(updater::check)
        .await
        .map_err(|e| e.to_string())?
}
static UPDATE_RUNNING: Mutex<()> = Mutex::new(());
#[tauri::command]
async fn apply_update(
    app: tauri::AppHandle,
    manifest: updater::UpdateManifest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _update = UPDATE_RUNNING
            .try_lock()
            .map_err(|_| "已有一次更新正在进行。")?;
        let _steam = STEAM_LOCK
            .try_lock()
            .map_err(|_| "Steam 操作进行中，请等待结束后再更新。")?;
        // One-shot confirmations cannot survive files being replaced beneath them.
        *PREPARED.lock().map_err(|e| e.to_string())? = None;
        *DELETE_PREPARED.lock().map_err(|e| e.to_string())? = None;
        let _active = ActiveWrite::begin();
        let exe = std::env::current_exe().map_err(|e| format!("无法定位程序：{e}"))?;
        let app_dir = exe.parent().ok_or("无法定位程序目录。")?.to_path_buf();
        let exe_name = exe
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("程序文件名无效。")?
            .to_string();
        updater::install(&manifest, &app_dir, &exe_name, |progress| {
            let _ = app.emit("update-progress", progress);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn restart_app(app: tauri::AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("无法定位程序：{e}"))?;
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|e| format!("无法启动新版本：{e}"))?;
    app.exit(0);
    Ok(())
}
fn main() {
    // Steamworks SDK initializes with the tool's AppID, not the consumer game.
    // Set once before any worker threads start; never change CWD or a user's Steam files.
    std::env::set_var("SteamAppId", "245850");
    std::env::set_var("SteamGameId", "245850");
    // The previous run's executable rename unlocks once that process exits.
    updater::cleanup_previous_executable();
    std::env::set_var("SteamGameId", "245850");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if PUBLISHING.load(std::sync::atomic::Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.emit(
                        "workshop-log",
                        "正在写入（上传、删除或程序更新），暂不能关闭窗口，请等待完成。",
                    );
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            steam_status,
            list_workshop,
            read_mod_info,
            save_mod_version,
            prepare_upload,
            discard_upload,
            submit_upload,
            prepare_delete,
            discard_delete,
            submit_delete,
            app_version,
            check_update,
            apply_update,
            restart_app
        ])
        .run(tauri::generate_context!())
        .expect("启动 Workshop Studio 失败");
}
