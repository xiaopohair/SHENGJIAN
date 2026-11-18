//! Button profile document and per-trigger overlay composition.
//!
//! A profile is sparse: a missing `(button, trigger)` inherits from the base
//! mapping; an explicit entry (including `Disabled`) overrides. TV and Menu
//! are profile-editable (12-key boundary); Mute stays base-owned.

use crate::model::{ButtonActions, ButtonMappingSet, ButtonTrigger, LogicalAction, LogicalButton};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const BUTTON_PROFILES_SCHEMA_VERSION: u32 = 5;

/// 默认方案的显示名（固定，不随用户改名变化）。
///
/// 「默认方案」是**每个设备型号各一份**的基线：同型号的其他方案未显式绑定的
/// `(按键, 触发)` 继承它的动作，它自身未绑定的位置再继承基础映射
/// （`button-mappings.json`）。`device_model == None` 的那一份是「通用」兜底，
/// 对型号未知的情况生效。
pub const DEFAULT_PROFILE_NAME: &str = "默认";

/// 默认方案的稳定 id。型号未知时用 `generic` 段。
///
/// id 只用于持久化与前端定位，语义完全由 `is_default` 决定——不再有「固定全局 id」
/// 这种双轨标记。
pub fn default_profile_id(model: Option<&str>) -> String {
    match normalized_model(model) {
        Some(model) => format!("default-{}", model.to_lowercase()),
        None => "default-generic".to_owned(),
    }
}

/// 型号字符串统一为**大写去空白**；空串视作「通用」。
pub fn normalized_model(model: Option<&str>) -> Option<String> {
    let trimmed = model?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_uppercase())
}

/// 某型号的按键布局。
///
/// RC003（小米蓝牙语音遥控器 2 Pro）有 TV 键；ARN9（普通款）真机确认**没有 TV 键**
/// （见 `docs/device-identity.md` 与 2026-11-12 真机探测记录）。型号未知时给全量布局，
/// 保证用户仍能编辑所有可能的位置。
pub fn profile_buttons_for_model(model: Option<&str>) -> &'static [LogicalButton] {
    match normalized_model(model).as_deref() {
        Some("ARN9") => &ARN9_BUTTONS,
        Some("CHROMECAST") => &CHROMECAST_BUTTONS,
        _ => &PROFILE_BUTTONS,
    }
}

pub type TriggerBindings = BTreeMap<ButtonTrigger, LogicalAction>;

/// Buttons a profile may override (12 keys; TV and Menu included, Mute base-owned).
pub const PROFILE_BUTTONS: [LogicalButton; 12] = [
    LogicalButton::Back,
    LogicalButton::Ok,
    LogicalButton::Home,
    LogicalButton::Tv,
    LogicalButton::Menu,
    LogicalButton::Left,
    LogicalButton::Right,
    LogicalButton::Down,
    LogicalButton::Up,
    LogicalButton::Power,
    LogicalButton::VolumeUp,
    LogicalButton::VolumeDown,
];

pub fn is_profile_button(button: LogicalButton) -> bool {
    PROFILE_BUTTONS.contains(&button) || CHROMECAST_BUTTONS.contains(&button)
}

/// ARN9（小米蓝牙语音遥控器普通款）的布局：与 RC003 相同，但**没有 TV 键**。
pub const ARN9_BUTTONS: [LogicalButton; 11] = [
    LogicalButton::Back,
    LogicalButton::Ok,
    LogicalButton::Home,
    LogicalButton::Menu,
    LogicalButton::Left,
    LogicalButton::Right,
    LogicalButton::Down,
    LogicalButton::Up,
    LogicalButton::Power,
    LogicalButton::VolumeUp,
    LogicalButton::VolumeDown,
];

/// Chromecast Remote 的布局：**没有 TV / Menu**，多了 YouTube / Netflix / 信源。
///
/// 14 键：这台设备的静音键**要在方案里绑定**才能生效（基础映射里没有它），
/// 所以它和其它键一样进方案可编辑集合（2026-11-18 用户要求「静音键要真正静音」）。
pub const CHROMECAST_BUTTONS: [LogicalButton; 14] = [
    LogicalButton::Back,
    LogicalButton::Ok,
    LogicalButton::Home,
    LogicalButton::Left,
    LogicalButton::Right,
    LogicalButton::Down,
    LogicalButton::Up,
    LogicalButton::Power,
    LogicalButton::VolumeUp,
    LogicalButton::VolumeDown,
    LogicalButton::Youtube,
    LogicalButton::Netflix,
    LogicalButton::InputSource,
    LogicalButton::VolumeMute,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub app_targets: Vec<String>,
    #[serde(default)]
    pub auto_apply: bool,
    #[serde(default)]
    pub hold_repeat: Option<ButtonTrigger>,
    /// Per-button repeat override. A missing entry inherits `hold_repeat`;
    /// an explicit `null` disables repeat for that button.
    #[serde(default)]
    pub hold_repeat_by_button: BTreeMap<LogicalButton, Option<ButtonTrigger>>,
    #[serde(default)]
    pub voice_input_tool: Option<String>,
    /// 归属设备型号（全大写，如 `RC003`）；`None` = 通用方案，对所有型号可见。
    #[serde(default)]
    pub device_model: Option<String>,
    /// 是否为该型号的默认方案：不可删除、不可改名、不关联应用、不自动应用。
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub bindings: BTreeMap<LogicalButton, TriggerBindings>,
}

impl ButtonProfile {
    /// 是否为不可删除的默认方案。
    pub fn is_default_profile(&self) -> bool {
        self.is_default
    }

    /// 新建某型号的默认方案（恒存在、无应用绑定、不自动应用）。
    pub fn default_for(model: Option<&str>) -> Self {
        Self {
            id: default_profile_id(model),
            name: DEFAULT_PROFILE_NAME.to_owned(),
            created_at: 0,
            app_targets: Vec::new(),
            auto_apply: false,
            hold_repeat: None,
            hold_repeat_by_button: BTreeMap::new(),
            voice_input_tool: None,
            device_model: normalized_model(model),
            is_default: true,
            bindings: model_default_bindings(model),
        }
    }

    /// 该方案是否对给定型号可见（通用方案对所有型号可见）。
    pub fn applies_to_model(&self, model: Option<&str>) -> bool {
        match (&self.device_model, normalized_model(model)) {
            (None, _) => true,
            (Some(own), Some(current)) => *own == current,
            (Some(_), None) => false,
        }
    }

    pub fn bound_button_count(&self) -> usize {
        self.bindings.len()
    }

    pub fn matches_executable(&self, executable: &str) -> bool {
        let executable = executable.trim().to_lowercase();
        self.app_targets
            .iter()
            .any(|target| target.trim().to_lowercase() == executable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonProfilesDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub profiles: Vec<ButtonProfile>,
    #[serde(default)]
    pub active_manual_profile_id: Option<String>,
}

impl Default for ButtonProfilesDocument {
    fn default() -> Self {
        Self {
            schema_version: BUTTON_PROFILES_SCHEMA_VERSION,
            profiles: Vec::new(),
            active_manual_profile_id: None,
        }
    }
}

impl ButtonProfilesDocument {
    pub fn normalized(mut self) -> Self {
        let mut seen_ids = BTreeSet::new();
        self.profiles
            .retain(|profile| seen_ids.insert(profile.id.clone()));

        for profile in &mut self.profiles {
            profile.device_model = normalized_model(profile.device_model.as_deref());
            profile.name = profile.name.trim().to_owned();
            let mut targets = Vec::new();
            for target in &profile.app_targets {
                let trimmed = target.trim().to_lowercase();
                if !trimmed.is_empty() && !targets.contains(&trimmed) {
                    targets.push(trimmed);
                }
            }
            profile.app_targets = targets;
        }

        // 型号首次出现顺序决定列表分组顺序。
        let mut models: Vec<Option<String>> = Vec::new();
        for profile in &self.profiles {
            if !models.contains(&profile.device_model) {
                models.push(profile.device_model.clone());
            }
        }

        // 每个型号有且仅有一个默认方案：多出来的降级成普通方案。
        let mut seen_default: BTreeSet<Option<String>> = BTreeSet::new();
        for profile in &mut self.profiles {
            if profile.is_default && !seen_default.insert(profile.device_model.clone()) {
                profile.is_default = false;
            }
        }
        for model in &models {
            if !seen_default.contains(model) {
                self.profiles
                    .push(ButtonProfile::default_for(model.as_deref()));
                seen_default.insert(model.clone());
            }
        }
        // 通用默认方案恒存在：型号完全未知时列表也不会空。
        if !seen_default.contains(&None) {
            self.profiles.push(ButtonProfile::default_for(None));
            if !models.contains(&None) {
                models.push(None);
            }
        }

        // 默认方案的形状固定：名称「默认」、不关联应用、不自动应用。
        for profile in &mut self.profiles {
            if profile.is_default {
                profile.name = DEFAULT_PROFILE_NAME.to_owned();
                profile.app_targets.clear();
                profile.auto_apply = false;
            }
        }

        // 同一型号内 app_targets 不重复；跨型号允许（不同型号各自的同应用方案）。
        for model in &models {
            let mut claimed = BTreeSet::new();
            for profile in &mut self.profiles {
                if profile.is_default || &profile.device_model != model {
                    continue;
                }
                profile
                    .app_targets
                    .retain(|target| claimed.insert(target.clone()));
            }
        }

        // 分组排序：按型号首次出现顺序，每组的默认方案排最前。
        let mut ordered = Vec::with_capacity(self.profiles.len());
        for model in &models {
            if let Some(index) = self
                .profiles
                .iter()
                .position(|profile| &profile.device_model == model && profile.is_default)
            {
                ordered.push(self.profiles.remove(index));
            }
            let mut index = 0;
            while index < self.profiles.len() {
                if &self.profiles[index].device_model == model {
                    ordered.push(self.profiles.remove(index));
                } else {
                    index += 1;
                }
            }
        }
        // 兜底：型号分组没覆盖到的方案原样追加，绝不静默丢数据。
        ordered.extend(std::mem::take(&mut self.profiles));
        self.profiles = ordered;

        if let Some(active) = &self.active_manual_profile_id {
            if !self.profiles.iter().any(|profile| &profile.id == active) {
                self.active_manual_profile_id = None;
            }
        }
        self.schema_version = BUTTON_PROFILES_SCHEMA_VERSION;
        self
    }

    /// 某型号的默认方案（不存在时返回 None，调用方可用 `default_for` 补建）。
    pub fn default_profile_for(&self, model: Option<&str>) -> Option<&ButtonProfile> {
        let model = normalized_model(model);
        self.profiles
            .iter()
            .find(|profile| profile.is_default && profile.device_model == model)
    }

    /// 该型号可见的方案（通用方案对所有型号可见）。
    ///
    /// 例外：型号已有自己的默认方案时**不再列出通用默认方案**——它只是型号未知时的
    /// 兜底，列出来会让界面出现两个「默认」。
    pub fn profiles_for_model(&self, model: Option<&str>) -> Vec<&ButtonProfile> {
        let model = normalized_model(model);
        let has_own_default =
            model.is_some() && self.default_profile_for(model.as_deref()).is_some();
        self.profiles
            .iter()
            .filter(|profile| {
                let generic_default = profile.is_default && profile.device_model.is_none();
                if has_own_default && generic_default {
                    return false;
                }
                profile.applies_to_model(model.as_deref())
            })
            .collect()
    }

    pub fn profile(&self, id: &str) -> Option<&ButtonProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn profile_mut(&mut self, id: &str) -> Option<&mut ButtonProfile> {
        self.profiles.iter_mut().find(|profile| profile.id == id)
    }
}

/// Apply a profile to the base mapping with per-trigger inheritance.
///
/// The result is a complete mapping set ready for `ButtonService`: unbound
/// buttons and unbound triggers keep their base action.
pub fn compose_profile_mappings(
    profile: &ButtonProfile,
    base: &ButtonMappingSet,
) -> ButtonMappingSet {
    let mut result = base.clone();
    // 按**该型号的布局**遍历，而不是固定的小米 12 键：Chromecast 的静音不在
    // `PROFILE_BUTTONS` 里，用固定表会让它的默认方案绑定静默失效（2026-11-18）。
    for &button in profile_buttons_for_model(profile.device_model.as_deref()) {
        let Some(triggers) = profile.bindings.get(&button) else {
            continue;
        };
        if triggers.is_empty() {
            continue;
        }
        let base_actions = base.actions.get(&button).cloned().unwrap_or_default();
        let hold_repeat = profile
            .hold_repeat_by_button
            .get(&button)
            .copied()
            .unwrap_or(profile.hold_repeat.or(base_actions.hold_repeat));
        let resolve = |trigger: ButtonTrigger| -> LogicalAction {
            triggers
                .get(&trigger)
                .cloned()
                .unwrap_or_else(|| base_actions.action(trigger).clone())
        };
        result.actions.insert(
            button,
            ButtonActions {
                single: resolve(ButtonTrigger::Single),
                double: resolve(ButtonTrigger::Double),
                long: resolve(ButtonTrigger::Long),
                hold_repeat,
                ok_context_click: base_actions.ok_context_click,
            },
        );
    }
    if let Some(hold_repeat) = profile.hold_repeat {
        // 按**该型号的布局**遍历，而不是固定的小米 12 键：Chromecast 的静音不在
    // `PROFILE_BUTTONS` 里，用固定表会让它的默认方案绑定静默失效（2026-11-18）。
    for &button in profile_buttons_for_model(profile.device_model.as_deref()) {
            if let Some(actions) = result.actions.get_mut(&button) {
                actions.hold_repeat = Some(hold_repeat);
            }
        }
    }
    for (button, hold_repeat) in &profile.hold_repeat_by_button {
        if !is_profile_button(*button) {
            continue;
        }
        if let Some(actions) = result.actions.get_mut(button) {
            actions.hold_repeat = *hold_repeat;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, name: &str) -> ButtonProfile {
        ButtonProfile {
            id: id.to_owned(),
            name: name.to_owned(),
            created_at: 0,
            app_targets: Vec::new(),
            auto_apply: false,
            hold_repeat: None,
            hold_repeat_by_button: BTreeMap::new(),
            voice_input_tool: None,
            device_model: None,
            is_default: false,
            bindings: BTreeMap::new(),
        }
    }

    fn model_profile(id: &str, name: &str, model: &str) -> ButtonProfile {
        ButtonProfile {
            device_model: Some(model.to_owned()),
            ..profile(id, name)
        }
    }

    fn shortcut(key: &str) -> LogicalAction {
        LogicalAction::Shortcut {
            label: key.to_owned(),
            keys: vec![key.to_owned()],
        }
    }

    fn base_with_up() -> ButtonMappingSet {
        let mut mappings = ButtonMappingSet::default();
        mappings.set_actions(
            LogicalButton::Up,
            ButtonActions {
                single: shortcut("W"),
                double: LogicalAction::Disabled,
                long: shortcut("PageUp"),
                hold_repeat: None,
                ok_context_click: false,
            },
        );
        mappings
    }

    #[test]
    fn normalization_dedupes_ids_and_app_targets() {
        let mut first = profile("a", " 第一 ");
        first.app_targets = vec!["CloudMusic.EXE".to_owned(), "cloudmusic.exe".to_owned()];
        let document = ButtonProfilesDocument {
            schema_version: 99,
            profiles: vec![first, profile("a", "重复"), profile("b", "第二")],
            active_manual_profile_id: Some("missing".to_owned()),
        }
        .normalized();
        assert_eq!(document.schema_version, BUTTON_PROFILES_SCHEMA_VERSION);
        // The generic default profile is always present and first.
        assert_eq!(document.profiles.len(), 3);
        assert_eq!(document.profiles[0].id, default_profile_id(None));
        assert!(document.profiles[0].is_default);
        assert_eq!(document.profiles[1].name, "第一");
        assert_eq!(
            document.profiles[1].app_targets,
            vec!["cloudmusic.exe".to_owned()]
        );
        assert_eq!(document.active_manual_profile_id, None);
    }

    #[test]
    fn explicit_triggers_override_and_missing_triggers_inherit() {
        let mut single = TriggerBindings::new();
        single.insert(ButtonTrigger::Single, shortcut("J"));
        let mut profile = profile("p", "P");
        profile.bindings.insert(LogicalButton::Up, single);
        let effective = compose_profile_mappings(&profile, &base_with_up());
        let up = &effective.actions[&LogicalButton::Up];
        assert_eq!(up.single, shortcut("J"));
        assert_eq!(up.double, LogicalAction::Disabled);
        assert_eq!(up.long, shortcut("PageUp"));
    }

    #[test]
        fn tv_and_menu_are_profile_editable_and_mute_is_model_scoped() {
        assert!(is_profile_button(LogicalButton::Tv));
        assert!(is_profile_button(LogicalButton::Menu));
        // 静音只对 Chromecast 可编辑（它在 CHROMECAST_BUTTONS 里），这里断言全局函数也認它。
        assert!(is_profile_button(LogicalButton::VolumeMute));
    }

    #[test]
    fn profile_hold_repeat_overrides_base_hold_repeat() {
        let mut base = base_with_up();
        base.actions
            .get_mut(&LogicalButton::Up)
            .expect("up")
            .hold_repeat = Some(ButtonTrigger::Long);
        let mut profile = profile("p", "P");
        profile.hold_repeat = Some(ButtonTrigger::Single);
        let effective = compose_profile_mappings(&profile, &base);
        assert_eq!(
            effective.actions[&LogicalButton::Up].hold_repeat,
            Some(ButtonTrigger::Single)
        );
    }

    #[test]
    fn per_button_hold_repeat_overrides_global_and_base() {
        let mut base = base_with_up();
        base.actions
            .get_mut(&LogicalButton::Up)
            .expect("up")
            .hold_repeat = Some(ButtonTrigger::Long);
        let mut profile = profile("p", "P");
        profile.hold_repeat = Some(ButtonTrigger::Long);
        profile
            .hold_repeat_by_button
            .insert(LogicalButton::Up, Some(ButtonTrigger::Single));
        let effective = compose_profile_mappings(&profile, &base);
        assert_eq!(
            effective.actions[&LogicalButton::Up].hold_repeat,
            Some(ButtonTrigger::Single)
        );
    }

    #[test]
    fn per_button_null_disables_inherited_hold_repeat() {
        let mut base = base_with_up();
        base.actions
            .get_mut(&LogicalButton::Up)
            .expect("up")
            .hold_repeat = Some(ButtonTrigger::Long);
        let mut profile = profile("p", "P");
        profile
            .hold_repeat_by_button
            .insert(LogicalButton::Up, None);
        let effective = compose_profile_mappings(&profile, &base);
        assert_eq!(effective.actions[&LogicalButton::Up].hold_repeat, None);
    }

    #[test]
    fn explicit_disabled_is_a_real_override() {
        let mut long = TriggerBindings::new();
        long.insert(ButtonTrigger::Long, LogicalAction::Disabled);
        let mut profile = profile("p", "P");
        profile.bindings.insert(LogicalButton::Up, long);
        let effective = compose_profile_mappings(&profile, &base_with_up());
        assert_eq!(
            effective.actions[&LogicalButton::Up].long,
            LogicalAction::Disabled
        );
        assert_eq!(effective.actions[&LogicalButton::Up].single, shortcut("W"));
    }

    #[test]
    fn non_profile_buttons_are_not_overridden() {
        let mut bindings = TriggerBindings::new();
        bindings.insert(ButtonTrigger::Single, shortcut("J"));
        let mut profile = profile("p", "P");
        profile.bindings.insert(LogicalButton::VolumeMute, bindings);
        let effective = compose_profile_mappings(&profile, &base_with_up());
        assert!(!effective.actions.contains_key(&LogicalButton::VolumeMute));
    }

    #[test]
    fn hold_repeat_and_context_click_inherit_from_base() {
        let mut base = base_with_up();
        let up = base.actions.get_mut(&LogicalButton::Up).expect("up");
        up.hold_repeat = Some(ButtonTrigger::Long);
        up.ok_context_click = true;
        let mut single = TriggerBindings::new();
        single.insert(ButtonTrigger::Single, shortcut("J"));
        let mut profile = profile("p", "P");
        profile.bindings.insert(LogicalButton::Up, single);
        let effective = compose_profile_mappings(&profile, &base);
        let up = &effective.actions[&LogicalButton::Up];
        assert_eq!(up.hold_repeat, Some(ButtonTrigger::Long));
        assert!(up.ok_context_click);
    }

    #[test]
    fn default_profile_created_per_model() {
        let document = ButtonProfilesDocument {
            schema_version: 4,
            profiles: vec![
                model_profile("rc003-a", "A", "RC003"),
                model_profile("arn9-a", "B", "ARN9"),
            ],
            active_manual_profile_id: None,
        }
        .normalized();

        // RC003 一份默认、ARN9 一份默认、外加通用兜底。
        let defaults: Vec<_> = document
            .profiles
            .iter()
            .filter(|profile| profile.is_default)
            .collect();
        assert_eq!(defaults.len(), 3);
        assert_eq!(
            document
                .default_profile_for(Some("RC003"))
                .map(|profile| profile.name.clone()),
            Some(DEFAULT_PROFILE_NAME.to_owned())
        );
        assert_eq!(
            document.default_profile_for(Some("arn9")),
            document.default_profile_for(Some("ARN9")),
            "型号比较必须大小写无关"
        );
        assert!(document.default_profile_for(None).is_some());
    }

    #[test]
    fn normalized_enforces_default_shape() {
        let mut broken = model_profile("default-rc003", "被改过的名字", "rc003");
        broken.is_default = true;
        broken.app_targets = vec!["cloudmusic.exe".to_owned()];
        broken.auto_apply = true;
        let document = ButtonProfilesDocument {
            schema_version: 4,
            profiles: vec![broken],
            active_manual_profile_id: None,
        }
        .normalized();

        let default = document
            .default_profile_for(Some("RC003"))
            .expect("RC003 default");
        assert_eq!(default.name, DEFAULT_PROFILE_NAME);
        assert_eq!(default.device_model.as_deref(), Some("RC003"));
        assert!(default.app_targets.is_empty());
        assert!(!default.auto_apply);
    }

    #[test]
    fn duplicate_default_for_same_model_is_demoted() {
        let mut first = model_profile("d1", "一", "RC003");
        first.is_default = true;
        let mut second = model_profile("d2", "二", "RC003");
        second.is_default = true;
        let document = ButtonProfilesDocument {
            schema_version: 4,
            profiles: vec![first, second],
            active_manual_profile_id: None,
        }
        .normalized();
        let count = document
            .profiles
            .iter()
            .filter(|profile| {
                profile.is_default && profile.device_model.as_deref() == Some("RC003")
            })
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn model_profiles_are_filtered_by_model_and_generic_is_shared() {
        let document = ButtonProfilesDocument {
            schema_version: 4,
            profiles: vec![
                model_profile("rc003-music", "音乐", "RC003"),
                model_profile("arn9-music", "音乐", "ARN9"),
                profile("generic-shared", "通用方案"),
            ],
            active_manual_profile_id: None,
        }
        .normalized();

        let rc003 = document.profiles_for_model(Some("RC003"));
        let ids: Vec<_> = rc003.iter().map(|profile| profile.id.as_str()).collect();
        assert!(ids.contains(&"rc003-music"));
        assert!(!ids.contains(&"arn9-music"), "ARN9 方案不能出现在 RC003 下");
        assert!(ids.contains(&"generic-shared"), "通用方案对所有型号可见");
        assert!(
            !ids.contains(&default_profile_id(None).as_str()),
            "型号有自己的默认方案时不再列通用默认，避免出现两个「默认」"
        );

        // 型号完全未知时：只有通用方案可见。
        let unknown = document.profiles_for_model(None);
        assert!(unknown.iter().all(|profile| profile.device_model.is_none()));
    }

    #[test]
    fn arn9_layout_has_no_tv_key_but_rc003_does() {
        assert!(profile_buttons_for_model(Some("RC003")).contains(&LogicalButton::Tv));
        assert!(!profile_buttons_for_model(Some("ARN9")).contains(&LogicalButton::Tv));
        // 型号未知时给全量布局，保证所有位置都可编辑。
        assert!(profile_buttons_for_model(None).contains(&LogicalButton::Tv));
        assert_eq!(profile_buttons_for_model(Some("ARN9")).len(), 11);
        assert_eq!(profile_buttons_for_model(Some("RC003")).len(), 12);
        // RC001 实机确认有 TV 键（2026-11-15），布局与 RC003 一致，不走 ARN9 的 11 键特例。
        assert!(profile_buttons_for_model(Some("RC001")).contains(&LogicalButton::Tv));
        assert_eq!(profile_buttons_for_model(Some("RC001")).len(), 12);
    }
}
#[cfg(test)]
mod chromecast_layout_tests {
    use super::*;

    #[test]
    fn chromecast_layout_has_the_three_new_keys_and_no_tv_or_menu() {
        let layout = profile_buttons_for_model(Some("Chromecast"));
        assert_eq!(layout.len(), 14);
        assert!(layout.contains(&LogicalButton::Youtube));
        assert!(layout.contains(&LogicalButton::Netflix));
        assert!(layout.contains(&LogicalButton::InputSource));
        assert!(!layout.contains(&LogicalButton::Tv));
        assert!(!layout.contains(&LogicalButton::Menu));
        // 静音现在也在集合里：它必须能在方案里绑定才生效。
        assert!(layout.contains(&LogicalButton::VolumeMute));
        assert!(is_profile_button(LogicalButton::InputSource));
        // 小米三款布局不变。
        assert_eq!(profile_buttons_for_model(Some("ARN9")).len(), 11);
        assert_eq!(profile_buttons_for_model(Some("RC003")).len(), 12);
    }
}

/// 单个按键的「单击」绑定构造器（仅用于型号默认方案）。
fn single_trigger(action: LogicalAction) -> TriggerBindings {
    let mut triggers = BTreeMap::new();
    triggers.insert(ButtonTrigger::Single, action);
    triggers
}

fn shortcut(label: &str, keys: &[&str]) -> TriggerBindings {
    single_trigger(LogicalAction::Shortcut {
        label: label.to_owned(),
        keys: keys.iter().map(|key| (*key).to_owned()).collect(),
    })
}

fn system_action(key: &str) -> TriggerBindings {
    single_trigger(LogicalAction::SystemAction {
        key: key.to_owned(),
    })
}

/// 型号默认方案的初始绑定（「连上即用」的基线）。
///
/// 分层是 `button-mappings.json` → **型号默认方案** → 当前方案，所以写在这里的动作
/// 不需要用户手动复制任何推荐方案就会生效。目前只有 Chromecast Remote 有基线：
/// 它的 14 个键里，方向/确定/返回/主页/音量/静音/电源有通用语义，
/// 静音按既有约定归**基础映射**（不进型号布局），因此这里不绑定；
/// YouTube / Netflix / 信源没有通用的 Windows 动作，保持未绑定。
pub fn model_default_bindings(model: Option<&str>) -> BTreeMap<LogicalButton, TriggerBindings> {
    let mut bindings: BTreeMap<LogicalButton, TriggerBindings> = BTreeMap::new();
    // 小米三款（RC001 / RC003 / ARN9）：方向 / OK / 返回 / 音量±。
    //
    // 语义上没有歧义，全部映射到等价键盘输入：遥控器当方向+确认用。
    // Home / 菜单 / TV / 电源保持未绑定——它们没有通用的 Windows 语义，
    // 交给用户按需配置（2026-11-19 用户要求补上前六个键）。
    if matches!(
        normalized_model(model).as_deref(),
        Some("RC001") | Some("RC003") | Some("ARN9")
    ) {
        bindings.insert(LogicalButton::Up, shortcut("上", &["up"]));
        bindings.insert(LogicalButton::Down, shortcut("下", &["down"]));
        bindings.insert(LogicalButton::Left, shortcut("左", &["left"]));
        bindings.insert(LogicalButton::Right, shortcut("右", &["right"]));
        bindings.insert(LogicalButton::Ok, shortcut("确定 / 回车", &["enter"]));
        bindings.insert(LogicalButton::Back, shortcut("返回 / Esc", &["escape"]));
        bindings.insert(LogicalButton::VolumeUp, system_action("volume_up"));
        bindings.insert(LogicalButton::VolumeDown, system_action("volume_down"));
        return bindings;
    }
    if normalized_model(model).as_deref() != Some("CHROMECAST") {
        return bindings;
    }
    bindings.insert(LogicalButton::Ok, shortcut("确定 / 回车", &["enter"]));
    bindings.insert(LogicalButton::Back, shortcut("返回 / Esc", &["escape"]));
    bindings.insert(LogicalButton::Up, shortcut("上", &["up"]));
    bindings.insert(LogicalButton::Down, shortcut("下", &["down"]));
    bindings.insert(LogicalButton::Left, shortcut("左", &["left"]));
    bindings.insert(LogicalButton::Right, shortcut("右", &["right"]));
    bindings.insert(
        LogicalButton::Home,
        shortcut("新标签页", &["left_control", "t"]),
    );
    bindings.insert(LogicalButton::VolumeUp, system_action("volume_up"));
    bindings.insert(LogicalButton::VolumeDown, system_action("volume_down"));
    bindings.insert(LogicalButton::Power, shortcut("全屏", &["f11"]));
    bindings.insert(LogicalButton::VolumeMute, system_action("volume_mute"));
    bindings
}

#[cfg(test)]
mod model_default_bindings_tests {
    use super::*;

    #[test]
    fn chromecast_default_profile_ships_with_keyboard_mappings() {
        let profile = ButtonProfile::default_for(Some("Chromecast"));
        assert_eq!(profile.bindings.len(), 11);
        assert_eq!(
            profile.bindings[&LogicalButton::Ok][&ButtonTrigger::Single],
            LogicalAction::Shortcut {
                label: "确定 / 回车".to_owned(),
                keys: vec!["enter".to_owned()],
            }
        );
        assert_eq!(
            profile.bindings[&LogicalButton::VolumeUp][&ButtonTrigger::Single],
            LogicalAction::SystemAction {
                key: "volume_up".to_owned(),
            }
        );
        // YouTube / Netflix / 信源 没有通用动作，保持未绑定。
        assert!(!profile.bindings.contains_key(&LogicalButton::Youtube));
        assert!(!profile.bindings.contains_key(&LogicalButton::Netflix));
        assert!(!profile.bindings.contains_key(&LogicalButton::InputSource));
    }

    #[test]
    fn xiaomi_models_default_to_navigation_and_volume() {
        for model in [Some("RC003"), Some("ARN9"), Some("RC001")] {
            let profile = ButtonProfile::default_for(model);
            assert_eq!(profile.bindings.len(), 8, "model={model:?}");

            // 四个方向 + OK + 返回：等价键盘输入。
            let expected = [
                (LogicalButton::Up, "up"),
                (LogicalButton::Down, "down"),
                (LogicalButton::Left, "left"),
                (LogicalButton::Right, "right"),
                (LogicalButton::Ok, "enter"),
                (LogicalButton::Back, "escape"),
            ];
            for (button, key) in expected {
                let action = &profile.bindings[&button][&ButtonTrigger::Single];
                let LogicalAction::Shortcut { keys, .. } = action else {
                    panic!("{button:?} 应为快捷键，实际 {action:?}");
                };
                assert_eq!(keys, &vec![key.to_owned()], "model={model:?} button={button:?}");
            }

            // 音量± 仍是系统动作。
            assert_eq!(
                profile.bindings[&LogicalButton::VolumeUp][&ButtonTrigger::Single],
                LogicalAction::SystemAction {
                    key: "volume_up".to_owned(),
                }
            );
            assert_eq!(
                profile.bindings[&LogicalButton::VolumeDown][&ButtonTrigger::Single],
                LogicalAction::SystemAction {
                    key: "volume_down".to_owned(),
                }
            );

            // Home / 菜单 / TV / 电源保持未绑定。
            for button in [
                LogicalButton::Home,
                LogicalButton::Menu,
                LogicalButton::Tv,
                LogicalButton::Power,
            ] {
                assert!(
                    !profile.bindings.contains_key(&button),
                    "model={model:?} button={button:?}"
                );
            }
        }
        // 未知型号仍为空默认方案（通用兜底）。
        assert!(ButtonProfile::default_for(None).bindings.is_empty());
    }
}
#[cfg(test)]
mod compose_model_layout_tests {
    use super::*;

    #[test]
    fn chromecast_default_bindings_survive_composition_including_mute() {
        let base = ButtonMappingSet::default();
        let profile = ButtonProfile::default_for(Some("CHROMECAST"));
        let effective = compose_profile_mappings(&profile, &base);
        // 确定 → 回车
        assert_eq!(
            effective.action(LogicalButton::Ok, ButtonTrigger::Single),
            LogicalAction::Shortcut {
                label: "确定 / 回车".to_owned(),
                keys: vec!["enter".to_owned()],
            }
        );
        // 音量 + → 系统动作
        assert_eq!(
            effective.action(LogicalButton::VolumeUp, ButtonTrigger::Single),
            LogicalAction::SystemAction {
                key: "volume_up".to_owned(),
            }
        );
        // 静音现在也在型号布局里，因此能合成为真正的系统静音。
        assert_eq!(
            effective.action(LogicalButton::VolumeMute, ButtonTrigger::Single),
            LogicalAction::SystemAction {
                key: "volume_mute".to_owned(),
            }
        );
    }
}