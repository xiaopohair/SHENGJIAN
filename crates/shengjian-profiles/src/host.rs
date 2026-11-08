//! 自研键位方案宿主：方案存储、覆盖层生命周期、前台自动应用、页面命令协议。
//!
//! 覆盖层应用策略是**惰性求值**：引擎在每次手势解析动作前调用
//! [`ButtonProfileHost::sync_foreground`]，宿主在该调用点重算「当前应生效的
//! 方案」（前台自动应用命中 > 手动激活 > 默认）并按需推送覆盖层——无需
//! 前台窗口钩子，且继承的基础动作始终取现值（用户改基础映射立即生效）。

use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde::{Deserialize, Serialize};
use serde_json::json;

use shengjian_windows::button_profiles::{
    ButtonProfileHost, ButtonProfileHostContext, ProfileAccess,
};
use shengjian_windows::raw_input::RemoteButton;
use shengjian_windows::send_input::{
    ButtonAction, ButtonMappings, ButtonTrigger, BUTTON_MAPPINGS_SCHEMA_VERSION,
};

use crate::composition::{compose_overlay, is_profile_button};
use crate::model::{new_profile_id, ButtonProfile, ButtonProfilesDocument};
use crate::store::ProfileStore;

/// 快照中的单键基础动作摘要（默认方案卡与「当前基础动作」提示）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BaseButtonSummary {
    button: RemoteButton,
    single: ButtonAction,
    double: ButtonAction,
    long: ButtonAction,
}

/// 页面快照（`button_profiles_snapshot` 的 JSON 形状；每次 dispatch 后整体返回）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostSnapshot {
    marker: &'static str,
    access: ProfileAccess,
    /// 11 键基础动作摘要（只读展示）。
    base_bindings: Vec<BaseButtonSummary>,
    profiles: Vec<ButtonProfile>,
    /// 当前生效方案（auto 命中 > 手动激活）；`null` = 默认方案。
    active_profile_id: Option<String>,
    /// 生效来源：`default` / `manual` / `auto`。
    active_source: &'static str,
    /// auto 命中的前台应用（小写）；其余来源为 `null`。
    active_app_target: Option<String>,
    /// 手动激活的方案（与 `active_profile_id` 独立：auto 命中时两者不同）。
    manual_profile_id: Option<String>,
}

/// 当前已应用到引擎的生效方案（用于变化检测，避免每手势重复推送）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct AppliedActive {
    profile_id: Option<String>,
    source: &'static str,
    app_target: Option<String>,
}

struct HostState {
    document: ButtonProfilesDocument,
    access: ProfileAccess,
    /// 已推送到引擎的覆盖层（`None` = 默认方案，未安装覆盖层）。
    applied_overlay: Option<ButtonMappings>,
    applied: AppliedActive,
}

/// 惰性求值结果：当前应生效的方案与来源。
struct DesiredActive {
    profile_id: Option<String>,
    source: &'static str,
    app_target: Option<String>,
}

/// dispatch 的统一信封（对齐常用语组件的 `{ok, value?, error?}` 约定）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub struct CustomButtonProfileHost {
    store: ProfileStore,
    context: Arc<dyn ButtonProfileHostContext>,
    state: RwLock<HostState>,
}

impl CustomButtonProfileHost {
    pub fn new(
        store: ProfileStore,
        context: Arc<dyn ButtonProfileHostContext>,
        document: ButtonProfilesDocument,
    ) -> Self {
        Self {
            store,
            context,
            state: RwLock::new(HostState {
                document,
                access: ProfileAccess::Allowed { valid_until: None },
                applied_overlay: None,
                applied: AppliedActive {
                    profile_id: None,
                    source: "default",
                    app_target: None,
                },
            }),
        }
    }

    fn read_state(&self) -> RwLockReadGuard<'_, HostState> {
        match self.state.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn write_state(&self) -> RwLockWriteGuard<'_, HostState> {
        match self.state.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn now_millis() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }

    /// 持久化并回写内存镜像。失败时内存保持原状（下次操作重试落盘）。
    fn persist(&self, state: &mut HostState) -> Result<(), String> {
        let document = state.document.clone();
        let saved = self.store.save(document)?;
        state.document = saved;
        Ok(())
    }

    fn snapshot_value(&self) -> serde_json::Value {
        let base = self.context.base_mappings();
        let state = self.read_state();
        let desired = evaluate_active(
            &state.document,
            self.context.foreground_executable().as_deref(),
        );
        let base_bindings = crate::composition::PROFILE_BUTTONS
            .iter()
            .map(|button| {
                let actions = base.actions.get(button).cloned().unwrap_or_default();
                BaseButtonSummary {
                    button: *button,
                    single: actions.single,
                    double: actions.double,
                    long: actions.long,
                }
            })
            .collect();
        serde_json::to_value(HostSnapshot {
            marker: crate::COMPONENT_MARKER,
            access: state.access.clone(),
            base_bindings,
            profiles: state.document.profiles.clone(),
            active_profile_id: desired.profile_id,
            active_source: desired.source,
            active_app_target: desired.app_target,
            manual_profile_id: state.document.active_manual_profile_id.clone(),
        })
        .unwrap_or(json!({"marker": crate::COMPONENT_MARKER, "error": "snapshot_serialize_failed"}))
    }

    fn envelope(
        &self,
        ok: bool,
        value: Option<serde_json::Value>,
        error: Option<String>,
    ) -> String {
        serde_json::to_string(&Envelope { ok, value, error })
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"envelope_serialize_failed\"}".to_owned())
    }

    fn snapshot_envelope(&self) -> String {
        let value = self.snapshot_value();
        self.envelope(true, Some(value), None)
    }

    fn error_envelope(&self, message: impl Into<String>) -> String {
        self.envelope(false, None, Some(message.into()))
    }

    /// 重算当前应生效的方案并按需推送覆盖层。
    ///
    /// 每次手势前（`sync_foreground`）与每次方案变更后调用：既覆盖前台自动
    /// 应用，也让「继承的基础动作」始终取基础映射现值。
    fn refresh_active_profile(&self) {
        let executable = self.context.foreground_executable();
        let base = self.context.base_mappings();
        let overlay = {
            let state = self.read_state();
            let desired = evaluate_active(&state.document, executable.as_deref());
            let overlay = desired
                .profile_id
                .as_ref()
                .and_then(|id| state.document.profile(id))
                .map(|profile| compose_overlay(profile, &base));
            (desired, overlay)
        };
        let (desired, overlay) = overlay;
        let mut state = self.write_state();
        let next = AppliedActive {
            profile_id: desired.profile_id,
            source: desired.source,
            app_target: desired.app_target,
        };
        if state.applied == next
            && overlay_equivalent(state.applied_overlay.as_ref(), overlay.as_ref())
        {
            return;
        }
        // 基础映射总开关关闭时引擎会忽略覆盖层；这里照常推送，语义由引擎统一。
        self.context.set_overlay(overlay.clone());
        state.applied = next;
        state.applied_overlay = overlay;
    }

    /// OpenApp 完成后的冷启动预置：目标路径能解出 `.exe` 文件名且命中某个
    /// 自动应用方案时，先按该方案推送覆盖层（等真实前台就绪前的窗口期）。
    /// 预设应用（target = 预设 id，非路径）不做预置，由后续手势的
    /// `sync_foreground` 惰性接管。
    fn prearm_for_target(&self, target: &str) {
        let Some(executable) = executable_from_target(target) else {
            return;
        };
        let base = self.context.base_mappings();
        let overlay = {
            let state = self.read_state();
            let profile = state
                .document
                .profiles
                .iter()
                .find(|profile| profile.auto_apply && profile.matches_executable(&executable));
            let Some(profile) = profile else {
                return;
            };
            Some(compose_overlay(profile, &base))
        };
        let mut state = self.write_state();
        if overlay_equivalent(state.applied_overlay.as_ref(), overlay.as_ref()) {
            return;
        }
        self.context.set_overlay(overlay.clone());
        state.applied = AppliedActive {
            profile_id: state
                .document
                .profiles
                .iter()
                .find(|profile| profile.auto_apply && profile.matches_executable(&executable))
                .map(|profile| profile.id.clone()),
            source: "auto",
            app_target: Some(executable.to_lowercase()),
        };
        state.applied_overlay = overlay;
    }

    fn dispatch_command(&self, command: &str, payload: Option<&str>) -> String {
        let result = match command {
            "create_profile" => self.command_create_profile(payload),
            "update_profile" => self.command_update_profile(payload),
            "set_binding" => self.command_set_binding(payload),
            "delete_profile" => self.command_delete_profile(payload),
            "activate" => self.command_activate(payload),
            "activate_default" => self.command_activate_default(),
            "export_to_file" => self.command_export_to_file(payload),
            "import_from_file" => self.command_import_from_file(payload),
            "refresh" => Ok(()),
            other => return self.error_envelope(format!("未知的键位方案命令：{other}")),
        };
        match result {
            Ok(()) => self.snapshot_envelope(),
            Err(error) => self.error_envelope(error),
        }
    }

    fn command_create_profile(&self, payload: Option<&str>) -> Result<(), String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            name: String,
        }
        let payload: Payload = parse_payload(payload)?;
        let name = payload.name.trim().to_owned();
        if name.is_empty() {
            return Err("方案名称不能为空".to_owned());
        }
        let mut state = self.write_state();
        state.document.profiles.push(ButtonProfile {
            id: new_profile_id(),
            name,
            created_at: Self::now_millis(),
            app_targets: Vec::new(),
            auto_apply: false,
            bindings: std::collections::BTreeMap::new(),
        });
        self.persist(&mut state)
    }

    fn command_update_profile(&self, payload: Option<&str>) -> Result<(), String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            id: String,
            name: Option<String>,
            #[serde(default)]
            app_targets: Option<Vec<String>>,
            auto_apply: Option<bool>,
        }
        let payload: Payload = parse_payload(payload)?;
        let mut state = self.write_state();
        // 校验先行：全部通过才触碰内存状态，失败不改变现状。
        if state.document.profile(&payload.id).is_none() {
            return Err("方案不存在".to_owned());
        }
        if let Some(name) = &payload.name {
            if name.trim().is_empty() {
                return Err("方案名称不能为空".to_owned());
            }
        }
        let next_targets: Option<Vec<String>> = payload.app_targets.as_ref().map(|targets| {
            targets
                .iter()
                .map(|target| target.trim().to_lowercase())
                .filter(|target| !target.is_empty())
                .collect()
        });
        // 一个 App 只能被一个方案关联（与 normalized 的「保留首个」语义一致，
        // 但这里显式报错，让界面能给出「App 已被其他方案关联」提示）。
        if let Some(targets) = &next_targets {
            for other in &state.document.profiles {
                if other.id == payload.id {
                    continue;
                }
                for target in &other.app_targets {
                    if targets.contains(target) {
                        return Err("App 已被其他方案关联".to_owned());
                    }
                }
            }
        }
        {
            let profile = state.document.profile_mut(&payload.id).unwrap();
            if let Some(name) = &payload.name {
                profile.name = name.trim().to_owned();
            }
            if let Some(targets) = next_targets {
                profile.app_targets = targets;
            }
            if let Some(auto_apply) = payload.auto_apply {
                profile.auto_apply = auto_apply;
            }
        }
        self.persist(&mut state)?;
        drop(state);
        self.refresh_active_profile();
        Ok(())
    }

    fn command_set_binding(&self, payload: Option<&str>) -> Result<(), String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            id: String,
            button: RemoteButton,
            trigger: ButtonTrigger,
            /// `None` = 清除该触发的绑定（恢复继承基础映射）。
            action: Option<ButtonAction>,
        }
        let payload: Payload = parse_payload(payload)?;
        if !is_profile_button(payload.button) {
            return Err("该按键不属于键位方案覆盖范围".to_owned());
        }
        if let Some(action) = &payload.action {
            validate_action(payload.button, payload.trigger, action)?;
        }
        let mut state = self.write_state();
        {
            let profile = state
                .document
                .profile_mut(&payload.id)
                .ok_or_else(|| "方案不存在".to_owned())?;
            let bindings = profile.bindings.entry(payload.button).or_default();
            match payload.action {
                Some(action) => {
                    bindings.insert(payload.trigger, action);
                }
                None => {
                    bindings.remove(&payload.trigger);
                    if bindings.is_empty() {
                        profile.bindings.remove(&payload.button);
                    }
                }
            }
        }
        self.persist(&mut state)?;
        drop(state);
        self.refresh_active_profile();
        Ok(())
    }

    fn command_delete_profile(&self, payload: Option<&str>) -> Result<(), String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            id: String,
        }
        let payload: Payload = parse_payload(payload)?;
        let mut state = self.write_state();
        let existed = state.document.profile(&payload.id).is_some();
        if !existed {
            return Err("方案不存在".to_owned());
        }
        state
            .document
            .profiles
            .retain(|profile| profile.id != payload.id);
        if state.document.active_manual_profile_id.as_deref() == Some(payload.id.as_str()) {
            state.document.active_manual_profile_id = None;
        }
        self.persist(&mut state)?;
        drop(state);
        self.refresh_active_profile();
        Ok(())
    }

    fn command_activate(&self, payload: Option<&str>) -> Result<(), String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            id: String,
        }
        let payload: Payload = parse_payload(payload)?;
        let mut state = self.write_state();
        if state.document.profile(&payload.id).is_none() {
            return Err("方案不存在".to_owned());
        }
        state.document.active_manual_profile_id = Some(payload.id);
        self.persist(&mut state)?;
        drop(state);
        self.refresh_active_profile();
        Ok(())
    }

    fn command_activate_default(&self) -> Result<(), String> {
        let mut state = self.write_state();
        state.document.active_manual_profile_id = None;
        self.persist(&mut state)?;
        drop(state);
        self.refresh_active_profile();
        Ok(())
    }

    fn command_export_to_file(&self, payload: Option<&str>) -> Result<(), String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            path: String,
        }
        let payload: Payload = parse_payload(payload)?;
        let state = self.read_state();
        self.store
            .write_export(Path::new(&payload.path), &state.document.profiles)
    }

    fn command_import_from_file(&self, payload: Option<&str>) -> Result<(), String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            path: String,
        }
        let payload: Payload = parse_payload(payload)?;
        let incoming = self.store.read_import(Path::new(&payload.path))?;
        let mut state = self.write_state();
        for profile in incoming {
            match state.document.profile_mut(&profile.id) {
                // 同 id 覆盖（备份回灌语义）。
                Some(existing) => *existing = profile,
                None => state.document.profiles.push(profile),
            }
        }
        self.persist(&mut state)?;
        drop(state);
        self.refresh_active_profile();
        Ok(())
    }
}

impl ButtonProfileHost for CustomButtonProfileHost {
    fn is_available(&self) -> bool {
        true
    }

    fn access(&self) -> ProfileAccess {
        self.read_state().access.clone()
    }

    fn set_access(&self, access: ProfileAccess) {
        if let Ok(mut state) = self.state.write() {
            state.access = access;
        }
    }

    fn snapshot_json(&self) -> Option<String> {
        Some(serde_json::to_string(&self.snapshot_value()).ok()?)
    }

    fn dispatch(&self, command: &str, payload: Option<&str>) -> Result<String, String> {
        Ok(self.dispatch_command(command, payload))
    }

    fn foreground_changed(&self, _executable: Option<&str>) {
        // 预留接口：引擎当前不调用；前台求值走 sync_foreground 的惰性路径。
    }

    fn sync_foreground(&self) {
        self.refresh_active_profile();
    }

    fn target_opened(&self, target: &str) {
        self.prearm_for_target(target);
    }
}

/// 前台自动应用求值：auto 命中（按 exe 精确匹配）> 手动激活 > 默认。
/// 前台未知（`None`）或未命中时保持手动激活，避免桌面/App 切换抖动。
fn evaluate_active(document: &ButtonProfilesDocument, executable: Option<&str>) -> DesiredActive {
    if let Some(executable) = executable {
        if let Some(profile) = document
            .profiles
            .iter()
            .find(|profile| profile.auto_apply && profile.matches_executable(executable))
        {
            return DesiredActive {
                profile_id: Some(profile.id.clone()),
                source: "auto",
                app_target: Some(executable.to_lowercase()),
            };
        }
    }
    match &document.active_manual_profile_id {
        Some(id) => DesiredActive {
            profile_id: Some(id.clone()),
            source: "manual",
            app_target: None,
        },
        None => DesiredActive {
            profile_id: None,
            source: "default",
            app_target: None,
        },
    }
}

/// OpenApp target（自定义应用路径）→ 可执行文件名；仅接受 `.exe` 形态，
/// 预设 id（非路径）返回 `None`。
fn executable_from_target(target: &str) -> Option<String> {
    let name = Path::new(target).file_name()?.to_str()?;
    let name = name.to_owned();
    if name.to_lowercase().ends_with(".exe") {
        Some(name)
    } else {
        None
    }
}

/// 覆盖层等价判定（ButtonMappings 已派生 PartialEq）。
fn overlay_equivalent(left: Option<&ButtonMappings>, right: Option<&ButtonMappings>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn parse_payload<'a, T: Deserialize<'a>>(payload: Option<&'a str>) -> Result<T, String> {
    let payload = payload.ok_or_else(|| "缺少命令参数".to_owned())?;
    serde_json::from_str(payload).map_err(|error| format!("命令参数无效：{error}"))
}

/// 用 `ButtonMappings::normalized` 的既有规则校验单条动作（chord 形状、
/// 鼠标量上限等），避免实现第二套校验。
fn validate_action(
    button: RemoteButton,
    trigger: ButtonTrigger,
    action: &ButtonAction,
) -> Result<(), String> {
    let mut actions = std::collections::BTreeMap::new();
    actions.insert(
        button,
        shengjian_windows::send_input::ButtonActions {
            single: ButtonAction::Disabled,
            double: ButtonAction::Disabled,
            long: ButtonAction::Disabled,
            hold_repeat: None,
            ok_context_click: false,
        },
    );
    if let Some(cell) = actions.get_mut(&button) {
        match trigger {
            ButtonTrigger::Single => cell.single = action.clone(),
            ButtonTrigger::Double => cell.double = action.clone(),
            ButtonTrigger::Long => cell.long = action.clone(),
        }
    }
    let mappings = ButtonMappings {
        schema_version: BUTTON_MAPPINGS_SCHEMA_VERSION,
        enabled: true,
        actions,
        applications: Vec::new(),
        focus_profiles: std::collections::BTreeMap::new(),
    };
    mappings
        .normalized()
        .map(|_| ())
        .map_err(|error| format!("动作无效：{error}"))
}
