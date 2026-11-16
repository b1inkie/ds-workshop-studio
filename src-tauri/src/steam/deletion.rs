//! Explicit two-step DST deletion. No fallback/retry after an ambiguous Steam response.
use super::*;
use sha2::{Digest, Sha256};
// Steam SDK: DeleteItemResult_t::k_iCallback = k_iSteamUGCCallbacks (3400) + 17.
// 3418 is UserSubscribedItemsListChanged_t and must not be used for DeleteItem.
const DELETE_ITEM_CALLBACK: i32 = 3417;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub token: String,
    pub id: String,
    pub title: String,
    pub app_id: u32,
    pub steam_id: String,
}
pub struct Plan {
    pub summary: Summary,
    pub created: Instant,
    updated: u32,
    _nonce: tempfile::TempDir,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub id: String,
    pub error: Option<String>,
}
#[repr(C, align(8))]
#[derive(Default)]
struct Deleted {
    result: i32,
    id: u64,
}
fn target(app_id: u32, id: &str) -> Result<u64, String> {
    if ![322330, 219740].contains(&app_id) {
        return Err("仅支持饥荒联机版与单机版物品。".into());
    }
    let parsed = id
        .parse::<u64>()
        .map_err(|_| "工坊 ID 必须是完整的正整数。")?;
    if parsed == 0 || parsed.to_string() != id {
        return Err("无效工坊 ID。".into());
    }
    Ok(parsed)
}
impl Plan {
    pub fn validate(&self, token: &str, confirmed_id: &str) -> Result<(), String> {
        if self.summary.token != token || self.created.elapsed() > Duration::from_secs(300) {
            return Err("删除确认已失效，请重新选择并确认。".into());
        }
        if confirmed_id != self.summary.id {
            return Err("输入的工坊 ID 与删除目标不一致。".into());
        }
        target(self.summary.app_id, confirmed_id)?;
        Ok(())
    }
}
pub fn take_confirmed(pending: &mut Option<Plan>, token: &str, id: &str) -> Result<Plan, String> {
    pending
        .as_ref()
        .ok_or("没有待确认的删除，请重新准备。")?
        .validate(token, id)?;
    pending.take().ok_or_else(|| "删除确认已被消费。".into())
}
pub fn prepare(path: &Path, app_id: u32, id: &str) -> Result<Plan, String> {
    let id_number = target(app_id, id)?;
    let s = Session::open(path)?;
    let (details, _) = s.owned_details(id_number, app_id)?;
    let nonce = tempfile::tempdir().map_err(|e| e.to_string())?;
    let token = format!(
        "{:x}",
        Sha256::digest(nonce.path().to_string_lossy().as_bytes())
    );
    Ok(Plan {
        summary: Summary {
            token,
            id: id.into(),
            title: text(&details.title),
            app_id,
            steam_id: s.id().to_string(),
        },
        created: Instant::now(),
        updated: details.updated,
        _nonce: nonce,
    })
}
fn result(result: Deleted, id: u64) -> Result<(), String> {
    if result.result != 1 {
        return Err(format!(
            "DeleteItem 失败，EResult={}。未确认删除成功，请刷新核对。",
            result.result
        ));
    }
    if result.id != 0 && result.id != id {
        return Err("Steam 返回的物品 ID 不一致，删除结果未知，请刷新核对。".into());
    }
    Ok(())
}
pub fn submit(path: &Path, plan: &Plan) -> Outcome {
    let operation = (|| -> Result<(), String> {
        let id = target(plan.summary.app_id, &plan.summary.id)?;
        let s = Session::open(path)?;
        if s.id().to_string() != plan.summary.steam_id {
            return Err("Steam 账号已切换，已阻止删除，请重新确认。".into());
        }
        // Check again immediately before the only write. Confirmation never trusts form metadata.
        let (details, _) = s.owned_details(id, plan.summary.app_id)?;
        if details.updated != plan.updated || text(&details.title) != plan.summary.title {
            return Err("物品在确认期间发生变化，已阻止删除，请重新确认。".into());
        }
        unsafe {
            let (call, callback) = if plan.summary.app_id == 219740 {
                (
                    (s.api.SteamAPI_ISteamRemoteStorage_DeletePublishedFile)(
                        s.remote_storage()?,
                        id,
                    ),
                    1311,
                )
            } else {
                (
                    (s.api.SteamAPI_ISteamUGC_DeleteItem)(s.ugc, id),
                    DELETE_ITEM_CALLBACK,
                )
            };
            if call == 0 {
                return Err("Steam 未返回有效删除句柄，结果未知；请刷新核对，勿直接重试。".into());
            }
            let started = Instant::now();
            loop {
                (s.api.SteamAPI_RunCallbacks)();
                let mut failed = false;
                if (s.api.SteamAPI_ISteamUtils_IsAPICallCompleted)(s.utils, call, &mut failed) {
                    if failed {
                        return Err("删除通信异常，结果未知；请刷新核对，勿直接重试。".into());
                    }
                    let mut deleted = Deleted::default();
                    let ok = (s.api.SteamAPI_ISteamUtils_GetAPICallResult)(
                        s.utils,
                        call,
                        &mut deleted as *mut _ as Ptr,
                        size_of::<Deleted>() as i32,
                        callback,
                        &mut failed,
                    );
                    if !ok || failed {
                        return Err("删除回调读取失败，结果未知；请刷新核对，勿直接重试。".into());
                    }
                    return result(deleted, id);
                }
                if started.elapsed() > Duration::from_secs(30) {
                    return Err("等待删除结果超时，结果未知；请刷新核对，勿直接重试。".into());
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    })();
    Outcome {
        id: plan.summary.id.clone(),
        error: operation.err(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delete_layout_and_result() {
        assert_eq!(DELETE_ITEM_CALLBACK, 3417);
        assert_eq!(size_of::<Deleted>(), 16);
        assert_eq!(std::mem::offset_of!(Deleted, id), 8);
        assert!(result(Deleted { result: 1, id: 42 }, 42).is_ok());
        assert!(result(Deleted { result: 1, id: 0 }, 42).is_ok());
        assert!(result(Deleted { result: 1, id: 43 }, 42).is_err());
        assert!(result(Deleted { result: 15, id: 42 }, 42).is_err());
    }
    #[test]
    fn confirmation_and_target_guards() {
        for bad in ["0", "-1", " 42", "042", "preview-1", "18446744073709551616"] {
            assert!(target(322330, bad).is_err());
        }
        assert!(target(219740, "42").is_ok());
        assert!(target(42, "42").is_err());
        let mut plan = Plan {
            summary: Summary {
                token: "test".into(),
                id: "18446744073709551615".into(),
                title: "fixture".into(),
                app_id: 322330,
                steam_id: "123".into(),
            },
            created: Instant::now(),
            updated: 0,
            _nonce: tempfile::tempdir().unwrap(),
        };
        assert!(plan.validate("test", &plan.summary.id).is_ok());
        assert!(plan.validate("wrong", &plan.summary.id).is_err());
        assert!(plan.validate("test", "42").is_err());
        plan.created = Instant::now() - Duration::from_secs(301);
        assert!(plan.validate("test", &plan.summary.id).is_err());
        plan.created = Instant::now();
        let id = plan.summary.id.clone();
        let mut pending = Some(plan);
        assert!(take_confirmed(&mut pending, "wrong", &id).is_err());
        assert!(pending.is_some());
        let consumed = take_confirmed(&mut pending, "test", &id).unwrap();
        assert!(pending.is_none());
        assert!(take_confirmed(&mut pending, "test", &id).is_err());
        drop(consumed);
    }
    #[test]
    fn ownership_game_and_identity() {
        let mut details: Details = unsafe { std::mem::zeroed() };
        details.id = 42;
        details.owner = 123;
        details.consumer = 322330;
        assert!(publishing::validate_owner(&details, 42, 123, 322330).is_ok());
        assert!(publishing::validate_owner(&details, 43, 123, 322330).is_err());
        assert!(publishing::validate_owner(&details, 42, 124, 322330).is_err());
        assert!(publishing::validate_owner(&details, 42, 123, 219740).is_err());
    }
}
