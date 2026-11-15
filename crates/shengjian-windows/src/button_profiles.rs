//! 键位方案的可选宿主接缝。
//!
//! 默认只有 [`DisabledButtonProfileHost`]：所有调用都是 no-op。
//! 需要扩展时可在启动时通过 [`set_button_profile_host`] 安装宿主；
//! 公开层不读取方案文件、不解析 `.shengjian`，也不做 App 匹配。

use std::sync::{Arc, OnceLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::send_input::ButtonMappings;

/// 可选方案宿主需要的公开能力。
///
/// 宿主只负责转发：写覆盖层、读基础映射、读前台可执行文件名。
/// 宿主不直接持有引擎句柄，也不执行 SendInput。
pub trait ButtonProfileHostContext: Send + Sync {
    /// 安装/清除当前方案覆盖层。
    fn set_overlay(&self, overlay: Option<ButtonMappings>);

    /// 当前基础映射（用于默认方案概览与生成覆盖层）。
    fn base_mappings(&self) -> ButtonMappings;

    /// 当前前台进程的可执行文件名（例如 `cloudmusic.exe`）；读不到为 `None`。
    fn foreground_executable(&self) -> Option<String> {
        None
    }
}

/// 方案宿主是否可用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum ProfileAccess {
    Unavailable,
    Allowed,
}

/// 键位方案宿主：可选实现，公开命令层只转发。
pub trait ButtonProfileHost: Send + Sync {
    /// 宿主是否已安装（不读取方案文件）。
    fn is_available(&self) -> bool {
        false
    }

    fn access(&self) -> ProfileAccess {
        ProfileAccess::Unavailable
    }

    /// 兼容入口：更新访问状态（默认 no-op）。
    fn set_access(&self, _access: ProfileAccess) {}

    /// 页面快照（JSON）。`None` = 宿主不可用。
    fn snapshot_json(&self) -> Option<String> {
        None
    }

    /// 页面命令分发。公开层不理解命令内容。
    fn dispatch(&self, _command: &str, _payload: Option<&str>) -> Result<String, String> {
        Err("button_profile_host_unavailable".to_owned())
    }

    /// 前台应用变化（可执行文件名；`None` = 无前台应用）。
    fn foreground_changed(&self, _executable: Option<&str>) {}

    /// 按键动作执行前同步当前前台应用，确保覆盖层与刚刚切换的 App 一致。
    fn sync_foreground(&self) {}

    /// 按键动作刚刚打开了某个应用：在真实前台窗口就绪前，先按目标应用
    /// 预备覆盖层，避免紧随其后的按键仍落在上一个应用的方案上。
    fn target_opened(&self, _target: &str) {}
}

/// 未安装宿主时的空实现。
#[derive(Debug, Default)]
pub struct DisabledButtonProfileHost;

impl ButtonProfileHost for DisabledButtonProfileHost {}

/// 全局宿主槽：启动时安装一次。
static BUTTON_PROFILE_HOST: OnceLock<RwLock<Arc<dyn ButtonProfileHost>>> = OnceLock::new();

fn button_profile_slot() -> &'static RwLock<Arc<dyn ButtonProfileHost>> {
    BUTTON_PROFILE_HOST.get_or_init(|| RwLock::new(Arc::new(DisabledButtonProfileHost)))
}

/// 安装/替换宿主（重复调用以后写入者为准）。
pub fn set_button_profile_host(host: Arc<dyn ButtonProfileHost>) {
    let slot = button_profile_slot();
    match slot.write() {
        Ok(mut guard) => *guard = host,
        Err(poisoned) => *poisoned.into_inner() = host,
    }
}

/// 当前宿主；默认是 [`DisabledButtonProfileHost`]。
pub fn current_button_profile_host() -> Arc<dyn ButtonProfileHost> {
    match button_profile_slot().read() {
        Ok(guard) => Arc::clone(&guard),
        Err(poisoned) => Arc::clone(&poisoned.into_inner()),
    }
}

/// 是否已安装可选宿主。
pub fn is_button_profile_host_available() -> bool {
    current_button_profile_host().is_available()
}

/// 兼容入口：更新访问状态（默认 no-op）。
pub fn set_button_profile_access(access: ProfileAccess) {
    current_button_profile_host().set_access(access);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_host_is_unavailable_and_forwards_nothing() {
        let host = DisabledButtonProfileHost;
        assert_eq!(host.access(), ProfileAccess::Unavailable);
        assert!(host.snapshot_json().is_none());
        assert_eq!(
            host.dispatch("snapshot", None),
            Err("button_profile_host_unavailable".to_owned())
        );
        host.foreground_changed(Some("notepad.exe"));
    }

    #[test]
    fn default_slot_has_no_optional_host() {
        assert!(!is_button_profile_host_available());
        assert_eq!(
            current_button_profile_host().access(),
            ProfileAccess::Unavailable
        );
    }

    #[test]
    fn access_serializes_with_the_frontend_contract() {
        assert_eq!(
            serde_json::to_value(ProfileAccess::Allowed).unwrap()["status"],
            "allowed"
        );
        assert_eq!(
            serde_json::to_value(ProfileAccess::Unavailable).unwrap()["status"],
            "unavailable"
        );
    }
}
