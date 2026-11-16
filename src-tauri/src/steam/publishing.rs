use super::*;
use crate::upload::{Outcome, Plan, Progress};
use std::{ffi::CString, fs, path::Path};

#[repr(C)]
pub(super) struct StringArray {
    pub(super) strings: *const *const c_char,
    pub(super) count: i32,
}
#[repr(C, align(8))]
#[derive(Default)]
struct Created {
    result: i32,
    id: u64,
    legal: bool,
}
#[repr(C, align(8))]
#[derive(Default)]
struct Submitted {
    result: i32,
    legal: bool,
    id: u64,
}
pub(super) trait Callback: Default {
    const ID: i32;
}
impl Callback for Created {
    const ID: i32 = 3403;
}
impl Callback for Submitted {
    const ID: i32 = 3404;
}
pub(super) fn c(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| "字符串包含空字符。".into())
}
pub(super) fn require(ok: bool, operation: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(format!("Steam {operation} 拒绝请求；未提交后续更新。"))
    }
}
pub(super) fn validate_owner(
    details: &Details,
    id: u64,
    owner: u64,
    app_id: u32,
) -> Result<(), String> {
    if details.id != id || details.owner != owner || details.consumer != app_id {
        return Err("物品 ID、所属游戏或所有者不匹配；只能操作当前 Steam 账号拥有的物品。".into());
    }
    Ok(())
}
impl Session {
    pub(super) fn owned_details(&self, id: u64, app_id: u32) -> Result<(Details, String), String> {
        unsafe {
            let a = &self.api;
            let handle = (a.SteamAPI_ISteamUGC_CreateQueryUGCDetailsRequest)(self.ugc, &id, 1);
            if handle == 0 || handle == u64::MAX {
                return Err("无法建立物品详情查询。".into());
            }
            let _query = Query {
                session: self,
                handle,
            };
            (a.SteamAPI_ISteamUGC_SetReturnMetadata)(self.ugc, handle, true);
            let done = self.wait((a.SteamAPI_ISteamUGC_SendQueryUGCRequest)(self.ugc, handle))?;
            if done.returned != 1 {
                return Err("工坊物品不存在或当前账号不可访问。".into());
            }
            let mut details: Details = std::mem::zeroed();
            if !(a.SteamAPI_ISteamUGC_GetQueryUGCResult)(self.ugc, handle, 0, &mut details)
                || details.result != 1
            {
                return Err("无法读取工坊物品详情。".into());
            }
            validate_owner(&details, id, self.id(), app_id)?;
            let mut metadata = [0u8; 5000];
            (a.SteamAPI_ISteamUGC_GetQueryUGCMetadata)(
                self.ugc,
                handle,
                0,
                metadata.as_mut_ptr(),
                metadata.len() as u32,
            );
            Ok((details, text(&metadata)))
        }
    }
    pub(super) fn owned_version(
        &self,
        id: u64,
        app_id: u32,
    ) -> Result<(String, Vec<String>), String> {
        let (details, metadata) = self.owned_details(id, app_id)?;
        if details.tags_truncated {
            return Err("线上 Tags 被查询接口截断，拒绝可能丢失标签的更新。".into());
        }
        let tags: Vec<String> = text(&details.tags)
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        Ok((version(&metadata, &tags), tags))
    }
    pub(super) fn wait_upload<T: Callback>(
        &self,
        call: u64,
        update: u64,
        id: u64,
        emit: &impl Fn(Progress),
    ) -> Result<T, String> {
        if call == 0 {
            return Err(
                "Steam 未返回有效上传调用句柄；请刷新工坊确认状态，不要直接重试新建。".into(),
            );
        }
        let start = Instant::now();
        let mut last = Instant::now() - Duration::from_secs(1);
        loop {
            unsafe {
                (self.api.SteamAPI_RunCallbacks)();
                if update != 0 && last.elapsed() > Duration::from_millis(300) {
                    let (mut processed, mut total) = (0, 0);
                    let state = (self.api.SteamAPI_ISteamUGC_GetItemUpdateProgress)(
                        self.ugc,
                        update,
                        &mut processed,
                        &mut total,
                    );
                    emit(Progress {
                        phase: match state {
                            1 => "准备配置",
                            2 => "准备文件",
                            3 => "上传内容",
                            4 => "上传封面",
                            5 => "提交更新",
                            _ => "等待 Steam",
                        }
                        .into(),
                        processed,
                        total,
                        id: id.to_string(),
                    });
                    last = Instant::now();
                }
                let mut failed = false;
                if (self.api.SteamAPI_ISteamUtils_IsAPICallCompleted)(self.utils, call, &mut failed)
                {
                    if failed {
                        return Err(
                            "Steam 上传通信失败；状态可能已改变，请刷新工坊后再操作。".into()
                        );
                    }
                    let mut result = T::default();
                    let ok = (self.api.SteamAPI_ISteamUtils_GetAPICallResult)(
                        self.utils,
                        call,
                        &mut result as *mut _ as Ptr,
                        size_of::<T>() as i32,
                        T::ID,
                        &mut failed,
                    );
                    if !ok || failed {
                        return Err("上传回调读取失败；请刷新确认线上状态。".into());
                    }
                    return Ok(result);
                }
            }
            if start.elapsed() > Duration::from_secs(900) {
                return Err(
                    "等待 Steam 上传超时（15 分钟），结果未知，请先刷新工坊，勿盲目重试新建。"
                        .into(),
                );
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}
pub fn preflight(path: &Path, plan: &mut Plan) -> Result<(), String> {
    let s = Session::open(path)?;
    plan.steam_id = s.id().to_string();
    if plan.request.app_id == 219740 {
        s.remote_storage()?;
        plan.summary.warnings.push("单机版使用 ZIP / Steam Cloud；不提交更新说明，无字节级上传进度。Cloud 暂存文件会保留，当前不自动清理。".into());
    }
    let id = plan.request.id.parse::<u64>().map_err(|_| "无效 ID")?;
    if id != 0 {
        let (previous, tags) = s.owned_version(id, plan.request.app_id)?;
        plan.check_version(&previous)?;
        // Content-only publication preserves unrelated online tags, not stale form values.
        if !plan.request.update_metadata {
            plan.tags = tags
                .iter()
                .filter(|s| {
                    ![
                        "version:",
                        "version=",
                        "version_compatible:",
                        "version_compatible=",
                    ]
                    .iter()
                    .any(|p| s.to_lowercase().starts_with(p))
                })
                .cloned()
                .collect();
            if !plan.version.is_empty() {
                plan.tags.push(format!("version:{}", plan.version));
            }
            if !plan.compatible.is_empty() {
                plan.tags
                    .push(format!("version_compatible:{}", plan.compatible));
            }
        }
        if !plan.request.update_content && plan.request.update_metadata {
            // Editing text must never erase version tags that were not re-read from Lua.
            plan.tags.extend(tags.into_iter().filter(|s| {
                [
                    "version:",
                    "version=",
                    "version_compatible:",
                    "version_compatible=",
                ]
                .iter()
                .any(|p| s.to_lowercase().starts_with(p))
            }));
        }
    }
    Ok(())
}
pub fn publish(
    path: &Path,
    plan: &Plan,
    force: bool,
    journal: &Path,
    emit: impl Fn(Progress),
) -> Outcome {
    if plan.request.app_id == 219740 {
        return super::remote::publish(path, plan, force, journal, emit);
    }
    let mut id = plan.request.id.parse::<u64>().unwrap_or(0);
    let mut legal = false;
    let result = (|| -> Result<(), String> {
        let s = Session::open(path)?;
        if s.id().to_string() != plan.steam_id {
            return Err("Steam 账号已切换，请重新准备上传。".into());
        }
        if id != 0 {
            let (current, _) = s.owned_version(id, plan.request.app_id)?;
            plan.validate_version(&current)?;
            if current != plan.summary.previous_version {
                return Err("线上版本在确认期间变化，请重新准备上传。".into());
            }
        }
        if plan.needs_force() && !force {
            return Err("需要明确确认线上版本未知。".into());
        }
        // Persist before any write. A timeout must never erase the recovery identity.
        let record = |state: &str, id: u64| -> Result<(), String> {
            fs::create_dir_all(journal).map_err(|e| e.to_string())?;
            let data = serde_json::json!({"id":id.to_string(),"state":state,"title":plan.request.title,"steamId":plan.steam_id});
            let file = fs::File::create(journal.join(format!("{}.json", plan.summary.token)))
                .map_err(|e| e.to_string())?;
            serde_json::to_writer_pretty(&file, &data).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())
        };
        record("starting", id)?;
        unsafe {
            let a = &s.api;
            if id == 0 {
                emit(Progress {
                    phase: "创建工坊物品".into(),
                    processed: 0,
                    total: 0,
                    id: "0".into(),
                });
                let created: Created = s.wait_upload(
                    (a.SteamAPI_ISteamUGC_CreateItem)(s.ugc, 322330, 0),
                    0,
                    0,
                    &emit,
                )?;
                if created.result != 1 {
                    return Err(format!("CreateItem 失败，EResult={}。", created.result));
                }
                id = created.id;
                legal = created.legal;
                if id == 0 {
                    return Err("Steam 返回空物品 ID，请刷新确认。".into());
                }
                record("created", id)?;
                emit(Progress {
                    phase: "新物品已创建，保存 ID 以便失败后恢复".into(),
                    processed: 0,
                    total: 0,
                    id: id.to_string(),
                });
            }
            let update = (a.SteamAPI_ISteamUGC_StartItemUpdate)(s.ugc, 322330, id);
            if update == 0 || update == u64::MAX {
                return Err("StartItemUpdate 失败。".into());
            }
            let r = &plan.request;
            if r.update_metadata {
                require(
                    (a.SteamAPI_ISteamUGC_SetItemTitle)(s.ugc, update, c(&r.title)?.as_ptr()),
                    "SetItemTitle",
                )?;
                require(
                    (a.SteamAPI_ISteamUGC_SetItemDescription)(
                        s.ugc,
                        update,
                        c(&r.description)?.as_ptr(),
                    ),
                    "SetItemDescription",
                )?;
            }
            if r.update_metadata || r.update_content {
                let strings = plan
                    .tags
                    .iter()
                    .map(|s| c(s))
                    .collect::<Result<Vec<_>, _>>()?;
                let pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
                let array = StringArray {
                    strings: pointers.as_ptr(),
                    count: pointers.len() as i32,
                };
                require(
                    (a.SteamAPI_ISteamUGC_SetItemTags)(s.ugc, update, &array),
                    "SetItemTags",
                )?;
                if !plan.version.is_empty() {
                    let metadata = format!(
                        "version:{}\nversion_compatible:{}",
                        plan.version, plan.compatible
                    );
                    require(
                        (a.SteamAPI_ISteamUGC_SetItemMetadata)(
                            s.ugc,
                            update,
                            c(&metadata)?.as_ptr(),
                        ),
                        "SetItemMetadata",
                    )?;
                }
            }
            if r.update_content {
                require(
                    (a.SteamAPI_ISteamUGC_SetItemContent)(
                        s.ugc,
                        update,
                        c(&plan.stage.path().join("content").to_string_lossy())?.as_ptr(),
                    ),
                    "SetItemContent",
                )?;
            }
            if r.update_preview {
                require(
                    (a.SteamAPI_ISteamUGC_SetItemPreview)(s.ugc, update, c(&r.preview)?.as_ptr()),
                    "SetItemPreview",
                )?;
            }
            require(
                (a.SteamAPI_ISteamUGC_SetItemVisibility)(s.ugc, update, r.visibility),
                "SetItemVisibility",
            )?;
            record("submitting", id)?;
            let note = c(&r.change_note)?;
            let submitted: Submitted = s.wait_upload(
                (a.SteamAPI_ISteamUGC_SubmitItemUpdate)(s.ugc, update, note.as_ptr()),
                update,
                id,
                &emit,
            )?;
            if submitted.result != 1 {
                return Err(format!(
                    "SubmitItemUpdate 失败，EResult={}。",
                    submitted.result
                ));
            }
            legal |= submitted.legal;
            record("success", id)?;
            emit(Progress {
                phase: "上传完成".into(),
                processed: 1,
                total: 1,
                id: id.to_string(),
            });
        }
        Ok(())
    })();
    Outcome {
        id: id.to_string(),
        needs_legal_agreement: legal,
        error: result.err(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_abi() {
        assert_eq!(size_of::<Created>(), 24);
        assert_eq!(std::mem::offset_of!(Created, id), 8);
        assert_eq!(std::mem::offset_of!(Created, legal), 16);
        assert_eq!(size_of::<Submitted>(), 16);
        assert_eq!(std::mem::offset_of!(Submitted, id), 8);
        assert_eq!(size_of::<StringArray>(), 8);
    }
}
