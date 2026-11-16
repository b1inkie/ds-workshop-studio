//! DS workshop uses Steam Cloud ZIP files, not DST's directory UGC upload.
use super::publishing::{c, require, Callback, StringArray};
use super::*;
use crate::upload::{Outcome, Plan, Progress};
use std::{ffi::CString, fs};

#[repr(C, align(8))]
struct Shared {
    result: i32,
    file: u64,
    filename: [u8; 260],
}
impl Default for Shared {
    fn default() -> Self {
        Self {
            result: 0,
            file: 0,
            filename: [0; 260],
        }
    }
}
impl Callback for Shared {
    const ID: i32 = 1307;
}
#[repr(C, align(8))]
#[derive(Default)]
struct Published {
    result: i32,
    id: u64,
    legal: bool,
}
impl Callback for Published {
    const ID: i32 = 1309;
}
#[repr(C, align(8))]
#[derive(Default)]
struct Updated {
    result: i32,
    id: u64,
    legal: bool,
}
// SDK callback: RemoteStorageUpdatePublishedFileResult_t = 1300 + 16.
impl Callback for Updated {
    const ID: i32 = 1316;
}
impl Session {
    pub(super) fn remote_storage(&self) -> Result<Ptr, String> {
        unsafe {
            let a = &self.api;
            let remote = (a.SteamAPI_ISteamClient_GetISteamRemoteStorage)(
                (a.SteamClient)(),
                (a.SteamAPI_GetHSteamUser)(),
                (a.SteamAPI_GetHSteamPipe)(),
                c"STEAMREMOTESTORAGE_INTERFACE_VERSION014".as_ptr(),
            );
            if remote.is_null() {
                Err("SteamRemoteStorage014 接口不可用。".into())
            } else {
                Ok(remote)
            }
        }
    }
    fn share_file(
        &self,
        remote: Ptr,
        name: &CString,
        local: &Path,
        id: u64,
        emit: &impl Fn(Progress),
    ) -> Result<(), String> {
        let length = fs::metadata(local).map_err(|e| e.to_string())?.len();
        if length == 0 || length > 100 * 1024 * 1024 {
            return Err("单机版 Cloud 文件必须非空且不超过 100MiB。".into());
        }
        // Bounded read: a changed local stage must not trigger unbounded allocation.
        use std::io::Read;
        let mut data = Vec::new();
        fs::File::open(local)
            .map_err(|e| e.to_string())?
            .take(100 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        if data.len() as u64 != length {
            return Err("单机版暂存文件在读取时变化。".into());
        }
        emit(Progress {
            phase: "写入并共享 Steam Cloud 文件（阶段进度）".into(),
            processed: 0,
            total: 0,
            id: id.to_string(),
        });
        unsafe {
            let a = &self.api;
            require(
                (a.SteamAPI_ISteamRemoteStorage_FileWrite)(
                    remote,
                    name.as_ptr(),
                    data.as_ptr(),
                    data.len() as i32,
                ),
                "FileWrite",
            )?;
            if !(a.SteamAPI_ISteamRemoteStorage_FileExists)(remote, name.as_ptr())
                || (a.SteamAPI_ISteamRemoteStorage_GetFileSize)(remote, name.as_ptr())
                    != data.len() as i32
            {
                return Err("Steam Cloud 写入后文件不可见或大小不一致，已停止发布。".into());
            }
            let result: Shared = self.wait_upload(
                (a.SteamAPI_ISteamRemoteStorage_FileShare)(remote, name.as_ptr()),
                0,
                id,
                emit,
            )?;
            require(
                result.result == 1 && result.file != 0,
                &format!("FileShare EResult={}", result.result),
            )?;
        }
        Ok(())
    }
}
fn validate_result(result: i32, returned_id: u64, expected: u64) -> Result<(), String> {
    if result != 1 {
        return Err(format!(
            "单机版提交失败，EResult={result}。请刷新核对状态。"
        ));
    }
    if returned_id == 0 || (expected != 0 && returned_id != expected) {
        return Err("单机版提交返回的工坊 ID 无效或不一致，结果未知，请刷新核对。".into());
    }
    Ok(())
}
pub(super) fn publish(
    path: &Path,
    plan: &Plan,
    force: bool,
    journal: &Path,
    emit: impl Fn(Progress),
) -> Outcome {
    let mut id = plan.request.id.parse::<u64>().unwrap_or(0);
    let mut legal = false;
    let operation = (|| -> Result<(), String> {
        let session = Session::open(path)?;
        if session.id().to_string() != plan.steam_id {
            return Err("Steam 账号已切换，请重新准备。".into());
        }
        if id != 0 {
            let (version, _) = session.owned_version(id, 219740)?;
            plan.validate_version(&version)?;
            if version != plan.summary.previous_version {
                return Err("线上版本在确认期间变化，请重新准备。".into());
            }
        }
        if plan.needs_force() && !force {
            return Err("请明确确认线上版本未知。".into());
        }
        let remote = session.remote_storage()?;
        let r = &plan.request;
        // Per-plan names prevent this application from overwriting unrelated Cloud files.
        let content_name = c(&format!("mod_publish_data_{}.zip", plan.summary.token))?;
        let ext = Path::new(&r.preview)
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("png");
        let preview_name = c(&format!(
            "mod_publish_preview_{}.{}",
            plan.summary.token, ext
        ))?;
        let record = |state: &str, id: u64| -> Result<(), String> {
            fs::create_dir_all(journal).map_err(|e| e.to_string())?;
            let file = fs::File::create(journal.join(format!("{}.json", plan.summary.token)))
                .map_err(|e| e.to_string())?;
            serde_json::to_writer_pretty(&file, &serde_json::json!({"appId":219740,"id":id.to_string(),"state":state,"title":r.title,"steamId":plan.steam_id,"cloudContent":content_name.to_string_lossy(),"cloudPreview":preview_name.to_string_lossy()})).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())
        };
        record("starting", id)?;
        if r.update_content {
            session.share_file(
                remote,
                &content_name,
                &plan.stage.path().join("mod_publish_data_file.zip"),
                id,
                &emit,
            )?;
        }
        if r.update_preview {
            session.share_file(remote, &preview_name, Path::new(&r.preview), id, &emit)?;
        }
        let strings = plan
            .tags
            .iter()
            .map(|s| c(s))
            .collect::<Result<Vec<_>, _>>()?;
        let pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
        let tags = StringArray {
            strings: pointers.as_ptr(),
            count: pointers.len() as i32,
        };
        let title = c(&r.title)?;
        let description = c(&r.description)?;
        unsafe {
            let a = &session.api;
            emit(Progress {
                phase: "提交单机版工坊请求（无字节进度）".into(),
                processed: 0,
                total: 0,
                id: id.to_string(),
            });
            if id == 0 {
                record("publishing", id)?;
                let published: Published = session.wait_upload(
                    (a.SteamAPI_ISteamRemoteStorage_PublishWorkshopFile)(
                        remote,
                        content_name.as_ptr(),
                        preview_name.as_ptr(),
                        219740,
                        title.as_ptr(),
                        description.as_ptr(),
                        r.visibility,
                        &tags,
                        0,
                    ),
                    0,
                    id,
                    &emit,
                )?;
                // Preserve a nonzero allocated ID even when the result reports a failure.
                id = published.id;
                legal = published.legal;
                if id != 0 {
                    record("created", id)?;
                    emit(Progress {
                        phase: "新单机物品已创建，保存 ID".into(),
                        processed: 0,
                        total: 0,
                        id: id.to_string(),
                    });
                }
                validate_result(published.result, id, 0)?;
            } else {
                let update =
                    (a.SteamAPI_ISteamRemoteStorage_CreatePublishedFileUpdateRequest)(remote, id);
                if update == 0 || update == u64::MAX {
                    return Err("CreatePublishedFileUpdateRequest 失败，未提交。".into());
                }
                if r.update_content {
                    require(
                        (a.SteamAPI_ISteamRemoteStorage_UpdatePublishedFileFile)(
                            remote,
                            update,
                            content_name.as_ptr(),
                        ),
                        "UpdatePublishedFileFile",
                    )?;
                }
                if r.update_preview {
                    require(
                        (a.SteamAPI_ISteamRemoteStorage_UpdatePublishedFilePreviewFile)(
                            remote,
                            update,
                            preview_name.as_ptr(),
                        ),
                        "UpdatePublishedFilePreviewFile",
                    )?;
                }
                if r.update_metadata {
                    require(
                        (a.SteamAPI_ISteamRemoteStorage_UpdatePublishedFileTitle)(
                            remote,
                            update,
                            title.as_ptr(),
                        ),
                        "UpdatePublishedFileTitle",
                    )?;
                    require(
                        (a.SteamAPI_ISteamRemoteStorage_UpdatePublishedFileDescription)(
                            remote,
                            update,
                            description.as_ptr(),
                        ),
                        "UpdatePublishedFileDescription",
                    )?;
                }
                if r.update_metadata || r.update_content {
                    require(
                        (a.SteamAPI_ISteamRemoteStorage_UpdatePublishedFileTags)(
                            remote, update, &tags,
                        ),
                        "UpdatePublishedFileTags",
                    )?;
                }
                require(
                    (a.SteamAPI_ISteamRemoteStorage_UpdatePublishedFileVisibility)(
                        remote,
                        update,
                        r.visibility,
                    ),
                    "UpdatePublishedFileVisibility",
                )?;
                record("submitting", id)?;
                let updated: Updated = session.wait_upload(
                    (a.SteamAPI_ISteamRemoteStorage_CommitPublishedFileUpdate)(remote, update),
                    0,
                    id,
                    &emit,
                )?;
                legal = updated.legal;
                validate_result(
                    updated.result,
                    if updated.id == 0 { id } else { updated.id },
                    id,
                )?;
            }
        }
        record("success", id)?;
        emit(Progress {
            phase: "单机版发布完成".into(),
            processed: 1,
            total: 1,
            id: id.to_string(),
        });
        Ok(())
    })();
    Outcome {
        id: id.to_string(),
        needs_legal_agreement: legal,
        error: operation.err(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_callback_abi_and_ids() {
        assert_eq!(Shared::ID, 1307);
        assert_eq!(Published::ID, 1309);
        assert_eq!(Updated::ID, 1316);
        assert_eq!(size_of::<Shared>(), 280);
        assert_eq!(std::mem::offset_of!(Shared, file), 8);
        assert_eq!(size_of::<Published>(), 24);
        assert_eq!(std::mem::offset_of!(Published, legal), 16);
        assert_eq!(size_of::<Updated>(), 24);
        assert!(validate_result(1, 42, 42).is_ok());
        assert!(validate_result(1, 43, 42).is_err());
        assert!(validate_result(1, 0, 0).is_err());
        assert!(validate_result(15, 42, 42).is_err());
    }
}
