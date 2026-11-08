//! 键位方案组件：**fork 自研实现**（原公开空壳已被本实现替换）。
//!
//! 与上游接缝保持兼容：[`install`] 的签名不变，`src-tauri` 的命令层
//! （snapshot / access / dispatch / pick_import / pick_export）与引擎的
//! 覆盖层消费逻辑零改动。`COMPONENT_MARKER` 是注入脚本与校验脚本的稳定
//! 契约，本实现将其改为 `custom` 以区分空壳与官方私有实现。
//!
//! 功能：方案增删改查、手动切换、按 (按键, 触发) 独立继承的覆盖层合成
//! （[`composition`]）、关联 App 的前台自动应用（惰性求值，挂在引擎的
//! `sync_foreground` / `target_opened` 调用点，见 [`host`]）。
//!
//! 持久化：`button-profiles.json`（与 `button-mappings.json` 同目录），
//! 读写护栏见 [`store`]。

pub mod composition;
pub mod host;
pub mod model;
pub mod store;

use std::path::PathBuf;
use std::sync::Arc;

use shengjian_windows::button_profiles::{ButtonProfileHost, ButtonProfileHostContext};
use shengjian_windows::gatt_note;

use crate::host::CustomButtonProfileHost;
use crate::model::ButtonProfilesDocument;
use crate::store::ProfileStore;

/// 组件标识：`custom` 表示 fork 的自研实现（上游空壳为
/// `windows-button-profiles/stub`，官方私有实现为非 stub 值）。
pub const COMPONENT_MARKER: &str = "windows-button-profiles/custom";

/// 安装宿主。存储损坏时按空库降级继续（保留原文件不回写），功能仍可用。
pub fn install(
    state_dir: PathBuf,
    context: Arc<dyn ButtonProfileHostContext>,
) -> Option<Arc<dyn ButtonProfileHost>> {
    let store = ProfileStore::new(&state_dir);
    let document = match store.load() {
        Ok(document) => document,
        Err(error) => {
            gatt_note(format!(
                "button_profiles store_load result=degraded error_note={error}"
            ));
            ButtonProfilesDocument::default()
        }
    };
    Some(Arc::new(CustomButtonProfileHost::new(
        store, context, document,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_windows::button_profiles::ProfileAccess;
    use shengjian_windows::send_input::KeyCode;
    use shengjian_windows::send_input::{ButtonMappings, BUTTON_MAPPINGS_SCHEMA_VERSION};
    use std::collections::BTreeMap;

    /// 可编程的宿主上下文：记录覆盖层推送，提供固定基础映射与前台。
    struct RecorderContext {
        base: std::sync::Mutex<ButtonMappings>,
        foreground: std::sync::Mutex<Option<String>>,
        overlays: std::sync::Mutex<Vec<Option<ButtonMappings>>>,
    }

    impl RecorderContext {
        fn new() -> Self {
            Self {
                base: std::sync::Mutex::new(ButtonMappings::default()),
                foreground: std::sync::Mutex::new(None),
                overlays: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn set_base(&self, mappings: ButtonMappings) {
            *self.base.lock().unwrap() = mappings;
        }

        fn set_foreground(&self, executable: Option<&str>) {
            *self.foreground.lock().unwrap() = executable.map(str::to_owned);
        }

        fn overlay_count(&self) -> usize {
            self.overlays.lock().unwrap().len()
        }

        fn last_overlay(&self) -> Option<ButtonMappings> {
            self.overlays.lock().unwrap().last().cloned().flatten()
        }
    }

    impl ButtonProfileHostContext for RecorderContext {
        fn set_overlay(&self, overlay: Option<ButtonMappings>) {
            self.overlays.lock().unwrap().push(overlay);
        }

        fn base_mappings(&self) -> ButtonMappings {
            self.base.lock().unwrap().clone()
        }

        fn foreground_executable(&self) -> Option<String> {
            self.foreground.lock().unwrap().clone()
        }
    }

    fn test_host() -> (
        tempdir::TempDir,
        Arc<CustomButtonProfileHost>,
        Arc<RecorderContext>,
    ) {
        let dir = tempdir::TempDir::new();
        let store = ProfileStore::new(dir.path());
        let context = Arc::new(RecorderContext::new());
        let host = Arc::new(CustomButtonProfileHost::new(
            store,
            context.clone(),
            ButtonProfilesDocument::default(),
        ));
        (dir, host, context)
    }

    /// 极简临时目录：测试结束递归删除。
    mod tempdir {
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicU64, Ordering};

        pub struct TempDir {
            path: PathBuf,
        }

        impl TempDir {
            pub fn new() -> Self {
                static COUNTER: AtomicU64 = AtomicU64::new(0);
                let path = std::env::temp_dir().join(format!(
                    "shengjian-windows-host-test-{}-{}",
                    std::process::id(),
                    COUNTER.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&path).unwrap();
                Self { path }
            }

            pub fn path(&self) -> &Path {
                &self.path
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }

    fn dispatch_json(host: &CustomButtonProfileHost, command: &str, payload: serde_json::Value) {
        let result = host
            .dispatch(command, Some(&serde_json::to_string(&payload).unwrap()))
            .unwrap();
        let envelope: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            envelope["ok"], true,
            "dispatch {command} failed: {envelope}"
        );
    }

    fn chord_action(key: KeyCode) -> shengjian_windows::send_input::ButtonAction {
        shengjian_windows::send_input::ButtonAction::Shortcut {
            chord: shengjian_windows::send_input::KeyChord { keys: vec![key] },
        }
    }

    #[test]
    fn install_returns_a_live_host() {
        let (_dir, host, _context) = test_host();
        assert!(host.is_available());
        assert_eq!(host.access(), ProfileAccess::Allowed { valid_until: None });
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        assert_eq!(snapshot["marker"], COMPONENT_MARKER);
        assert_eq!(snapshot["activeSource"], "default");
        assert_eq!(snapshot["profiles"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn create_and_activate_profile_applies_overlay_on_sync() {
        let (_dir, host, context) = test_host();
        dispatch_json(&host, "create_profile", serde_json::json!({"name": "音乐"}));
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        let id = snapshot["profiles"][0]["id"].as_str().unwrap().to_owned();
        dispatch_json(
            &host,
            "set_binding",
            serde_json::json!({"id": id, "button": "volume_up", "trigger": "single",
                               "action": {"type": "shortcut", "chord": {"keys": ["w"]}}}),
        );
        dispatch_json(&host, "activate", serde_json::json!({"id": id}));

        // 基础映射此时对 volume_up 无动作：激活后 sync_foreground 应推送覆盖层。
        host.sync_foreground();
        assert_eq!(context.overlay_count(), 1);
        let overlay = context.last_overlay().unwrap();
        assert_eq!(
            overlay.actions[&shengjian_windows::raw_input::RemoteButton::VolumeUp].single,
            chord_action(KeyCode::W)
        );

        // 未变化时不重复推送。
        host.sync_foreground();
        assert_eq!(context.overlay_count(), 1);
    }

    #[test]
    fn auto_apply_matches_foreground_executable_case_insensitively() {
        let (_dir, host, context) = test_host();
        dispatch_json(&host, "create_profile", serde_json::json!({"name": "音乐"}));
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        let id = snapshot["profiles"][0]["id"].as_str().unwrap().to_owned();
        dispatch_json(
            &host,
            "update_profile",
            serde_json::json!({"id": id, "appTargets": ["CloudMusic.exe"], "autoApply": true}),
        );
        dispatch_json(
            &host,
            "set_binding",
            serde_json::json!({"id": id, "button": "ok", "trigger": "single",
                               "action": {"type": "disabled"}}),
        );

        // 未命中前台：保持默认（不推送覆盖层）。
        host.sync_foreground();
        assert!(context.last_overlay().is_none());

        // 命中（大小写不敏感）：自动应用。
        context.set_foreground(Some("cloudmusic.EXE"));
        host.sync_foreground();
        let overlay = context.last_overlay().unwrap();
        assert_eq!(
            overlay.actions[&shengjian_windows::raw_input::RemoteButton::Ok].single,
            shengjian_windows::send_input::ButtonAction::Disabled
        );

        // 快照反映 auto 生效。
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        assert_eq!(snapshot["activeSource"], "auto");
        assert_eq!(snapshot["activeProfileId"], id);
        assert_eq!(snapshot["activeAppTarget"], "cloudmusic.exe");
        assert_eq!(snapshot["manualProfileId"], serde_json::Value::Null);

        // 前台离开：回落默认，覆盖层清除。
        context.set_foreground(None);
        host.sync_foreground();
        assert!(context.last_overlay().is_none());
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        assert_eq!(snapshot["activeSource"], "default");
    }

    #[test]
    fn inherited_bindings_follow_base_mapping_updates() {
        let (_dir, host, context) = test_host();
        // 基础映射：up 单击 = W。
        context.set_base(ButtonMappings {
            schema_version: BUTTON_MAPPINGS_SCHEMA_VERSION,
            enabled: true,
            actions: BTreeMap::from([(
                shengjian_windows::raw_input::RemoteButton::Up,
                shengjian_windows::send_input::ButtonActions {
                    single: chord_action(KeyCode::W),
                    double: shengjian_windows::send_input::ButtonAction::Disabled,
                    long: shengjian_windows::send_input::ButtonAction::Disabled,
                    hold_repeat: None,
                    ok_context_click: false,
                },
            )]),
            applications: Vec::new(),
            focus_profiles: BTreeMap::new(),
        });
        dispatch_json(&host, "create_profile", serde_json::json!({"name": "P"}));
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        let id = snapshot["profiles"][0]["id"].as_str().unwrap().to_owned();
        // 方案只显式覆盖 long = disabled；single/double 继承。
        dispatch_json(
            &host,
            "set_binding",
            serde_json::json!({"id": id, "button": "up", "trigger": "long",
                               "action": {"type": "disabled"}}),
        );
        dispatch_json(&host, "activate", serde_json::json!({"id": id}));
        host.sync_foreground();

        let overlay = context.last_overlay().unwrap();
        let up = &overlay.actions[&shengjian_windows::raw_input::RemoteButton::Up];
        assert_eq!(up.single, chord_action(KeyCode::W), "继承基础单击");
        assert_eq!(
            up.long,
            shengjian_windows::send_input::ButtonAction::Disabled
        );

        // 用户改基础单击 → 下一次 sync 后覆盖层里的继承位跟随更新。
        context.set_base(ButtonMappings {
            schema_version: BUTTON_MAPPINGS_SCHEMA_VERSION,
            enabled: true,
            actions: BTreeMap::from([(
                shengjian_windows::raw_input::RemoteButton::Up,
                shengjian_windows::send_input::ButtonActions {
                    single: chord_action(KeyCode::Q),
                    double: shengjian_windows::send_input::ButtonAction::Disabled,
                    long: shengjian_windows::send_input::ButtonAction::Disabled,
                    hold_repeat: None,
                    ok_context_click: false,
                },
            )]),
            applications: Vec::new(),
            focus_profiles: BTreeMap::new(),
        });
        host.sync_foreground();
        let overlay = context.last_overlay().unwrap();
        assert_eq!(
            overlay.actions[&shengjian_windows::raw_input::RemoteButton::Up].single,
            chord_action(KeyCode::Q)
        );
    }

    #[test]
    fn delete_active_profile_falls_back_to_default() {
        let (_dir, host, context) = test_host();
        dispatch_json(&host, "create_profile", serde_json::json!({"name": "P"}));
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        let id = snapshot["profiles"][0]["id"].as_str().unwrap().to_owned();
        dispatch_json(&host, "activate", serde_json::json!({"id": id}));
        host.sync_foreground();
        dispatch_json(&host, "delete_profile", serde_json::json!({"id": id}));
        host.sync_foreground();
        assert!(context.last_overlay().is_none());
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        assert_eq!(snapshot["activeSource"], "default");
        assert_eq!(snapshot["profiles"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn duplicate_app_target_across_auto_profiles_is_rejected() {
        let (_dir, host, _context) = test_host();
        dispatch_json(&host, "create_profile", serde_json::json!({"name": "A"}));
        dispatch_json(&host, "create_profile", serde_json::json!({"name": "B"}));
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        let ids: Vec<String> = snapshot["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|profile| profile["id"].as_str().unwrap().to_owned())
            .collect();
        dispatch_json(
            &host,
            "update_profile",
            serde_json::json!({"id": ids[0], "appTargets": ["a.exe"], "autoApply": true}),
        );
        let result = host
            .dispatch(
                "update_profile",
                Some(
                    &serde_json::json!({"id": ids[1], "appTargets": ["A.EXE"], "autoApply": true})
                        .to_string(),
                ),
            )
            .unwrap();
        let envelope: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(envelope["ok"], false);
    }

    #[test]
    fn export_and_import_round_trip_through_files() {
        let (_dir, host, _context) = test_host();
        dispatch_json(&host, "create_profile", serde_json::json!({"name": "P"}));
        let export_path = std::env::temp_dir().join(format!(
            "shengjian-windows-host-export-{}.json",
            std::process::id()
        ));
        dispatch_json(
            &host,
            "export_to_file",
            serde_json::json!({"path": export_path.to_string_lossy()}),
        );
        // 清空后回灌。
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        let id = snapshot["profiles"][0]["id"].as_str().unwrap().to_owned();
        dispatch_json(&host, "delete_profile", serde_json::json!({"id": id}));
        dispatch_json(
            &host,
            "import_from_file",
            serde_json::json!({"path": export_path.to_string_lossy()}),
        );
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        assert_eq!(snapshot["profiles"].as_array().unwrap().len(), 1);
        let _ = std::fs::remove_file(&export_path);
    }

    #[test]
    fn target_opened_prearms_for_exe_targets_only() {
        let (_dir, host, context) = test_host();
        dispatch_json(&host, "create_profile", serde_json::json!({"name": "音乐"}));
        let snapshot: serde_json::Value =
            serde_json::from_str(&host.snapshot_json().unwrap()).unwrap();
        let id = snapshot["profiles"][0]["id"].as_str().unwrap().to_owned();
        dispatch_json(
            &host,
            "update_profile",
            serde_json::json!({"id": id, "appTargets": ["cloudmusic.exe"], "autoApply": true}),
        );
        dispatch_json(
            &host,
            "set_binding",
            serde_json::json!({"id": id, "button": "ok", "trigger": "single",
                               "action": {"type": "disabled"}}),
        );
        // 自定义路径 target：预置覆盖层。
        host.target_opened("D:\\Apps\\CloudMusic.exe");
        assert!(context.last_overlay().is_some());
        // 预设 id target：不做预置（无 .exe 形态），等惰性接管。
        host.target_opened("netease_music");
    }

    #[test]
    fn business_errors_travel_in_the_envelope_not_as_rejections() {
        let (_dir, host, _context) = test_host();
        let result = host
            .dispatch(
                "activate",
                Some(&serde_json::json!({"id": "nope"}).to_string()),
            )
            .unwrap();
        let envelope: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(envelope["ok"], false);
        assert!(envelope["error"].as_str().unwrap().contains("方案不存在"));
        let result = host.dispatch("no_such_command", None).unwrap();
        let envelope: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(envelope["ok"], false);
    }
}
