//! 方案绑定 → 引擎覆盖层的合成。
//!
//! 引擎的覆盖层是**按键粒度整体替换**（`merged_mappings`：覆盖层里出现的键
//! 整条 `ButtonActions` 生效），而产品语义是 **(按键, 触发) 独立继承**（macOS
//! `feature/button-mapping/PRODUCT_SPEC.md`）。两者之间的桥就是这里的合成：
//! 宿主在激活方案时用 `base_mappings()` 把继承的触发位填充成基础动作，
//! 引擎零改动。
//!
//! 已知边界（一期接受）：被方案覆盖的按键，其「继承得到」的 Disabled 触发位
//! 按覆盖层 Disabled 语义执行（吞键、不回注原生键）；基础映射未配置该触发时，
//! 原生按键在方案激活期间不再透传。产品上等价于「方案接管了这个键」。

use std::collections::BTreeMap;

use shengjian_windows::raw_input::RemoteButton;
use shengjian_windows::send_input::{
    ButtonAction, ButtonActions, ButtonMappings, ButtonTrigger, BUTTON_MAPPINGS_SCHEMA_VERSION,
};

use crate::model::ButtonProfile;

/// 方案覆盖的按键白名单：11 键边界（TV 纳入），
/// 菜单 / 静音仍由基础层持有。
pub const PROFILE_BUTTONS: [RemoteButton; 11] = [
    RemoteButton::Back,
    RemoteButton::Ok,
    RemoteButton::Home,
    RemoteButton::Tv,
    RemoteButton::Left,
    RemoteButton::Right,
    RemoteButton::Down,
    RemoteButton::Up,
    RemoteButton::Power,
    RemoteButton::VolumeUp,
    RemoteButton::VolumeDown,
];

pub fn is_profile_button(button: RemoteButton) -> bool {
    PROFILE_BUTTONS.contains(&button)
}

/// 合成覆盖层：只包含至少有一个显式绑定的按键；继承位从基础映射现值填充。
///
/// 引擎的 `merged_mappings` 会以 `base.clone()` 为底，`applications`
/// （打开应用的自定义路径）恒由基础层带出；覆盖层里的 `focus_profiles`
/// 为空时也不影响基础聚焦档案。
pub fn compose_overlay(profile: &ButtonProfile, base: &ButtonMappings) -> ButtonMappings {
    let mut actions = BTreeMap::new();
    for button in PROFILE_BUTTONS {
        let Some(triggers) = profile.bindings.get(&button) else {
            continue;
        };
        // 空绑定表（所有触发都已清除回继承）＝ 该按键行为与基础层完全一致，
        // 不进覆盖层，保持基础层的原生透传语义。
        if triggers.is_empty() {
            continue;
        }
        let base_actions = base.actions.get(&button).cloned().unwrap_or_default();
        let resolve = |trigger: ButtonTrigger| -> ButtonAction {
            match triggers.get(&trigger) {
                Some(action) => action.clone(),
                None => base_actions.trigger(trigger).clone(),
            }
        };
        actions.insert(
            button,
            ButtonActions {
                single: resolve(ButtonTrigger::Single),
                double: resolve(ButtonTrigger::Double),
                long: resolve(ButtonTrigger::Long),
                // 一期：按住连续触发继承基础层，不在方案中编辑。
                hold_repeat: base_actions.hold_repeat,
                ok_context_click: base_actions.ok_context_click,
            },
        );
    }
    ButtonMappings {
        schema_version: BUTTON_MAPPINGS_SCHEMA_VERSION,
        enabled: true,
        actions,
        applications: Vec::new(),
        focus_profiles: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TriggerBindings;
    use shengjian_windows::send_input::{KeyChord, KeyCode};

    fn chord_action(key: KeyCode) -> ButtonAction {
        ButtonAction::Shortcut {
            chord: KeyChord { keys: vec![key] },
        }
    }

    fn base_mappings() -> ButtonMappings {
        let mut actions = BTreeMap::new();
        actions.insert(
            RemoteButton::Up,
            ButtonActions {
                single: chord_action(KeyCode::W),
                double: ButtonAction::Disabled,
                long: chord_action(KeyCode::PageUp),
                hold_repeat: None,
                ok_context_click: false,
            },
        );
        ButtonMappings {
            schema_version: BUTTON_MAPPINGS_SCHEMA_VERSION,
            enabled: true,
            actions,
            applications: Vec::new(),
            focus_profiles: BTreeMap::new(),
        }
    }

    fn profile_with(bindings: BTreeMap<RemoteButton, TriggerBindings>) -> ButtonProfile {
        ButtonProfile {
            id: "p".to_owned(),
            name: "P".to_owned(),
            created_at: 0,
            app_targets: Vec::new(),
            auto_apply: false,
            bindings,
        }
    }

    #[test]
    fn explicit_triggers_override_and_missing_triggers_inherit() {
        let mut single_only = TriggerBindings::new();
        single_only.insert(ButtonTrigger::Single, chord_action(KeyCode::J));
        let profile = profile_with(BTreeMap::from([(RemoteButton::Up, single_only)]));
        let overlay = compose_overlay(&profile, &base_mappings());
        let up = &overlay.actions[&RemoteButton::Up];
        assert_eq!(up.single, chord_action(KeyCode::J));
        assert_eq!(up.double, ButtonAction::Disabled);
        assert_eq!(up.long, chord_action(KeyCode::PageUp));
    }

    #[test]
    fn explicit_disabled_is_a_real_override() {
        let mut long_disabled = TriggerBindings::new();
        long_disabled.insert(ButtonTrigger::Long, ButtonAction::Disabled);
        let profile = profile_with(BTreeMap::from([(RemoteButton::Up, long_disabled)]));
        let overlay = compose_overlay(&profile, &base_mappings());
        assert_eq!(
            overlay.actions[&RemoteButton::Up].long,
            ButtonAction::Disabled
        );
        assert_eq!(
            overlay.actions[&RemoteButton::Up].single,
            chord_action(KeyCode::W)
        );
    }

    #[test]
    fn buttons_without_bindings_are_absent_from_the_overlay() {
        let mut single_only = TriggerBindings::new();
        single_only.insert(ButtonTrigger::Single, chord_action(KeyCode::J));
        let profile = profile_with(BTreeMap::from([
            (RemoteButton::Up, single_only),
            (RemoteButton::VolumeMute, BTreeMap::new()), // 非白名单键应被忽略
        ]));
        let overlay = compose_overlay(&profile, &base_mappings());
        assert_eq!(overlay.actions.len(), 1);
        assert!(!overlay.actions.contains_key(&RemoteButton::VolumeMute));
    }

    #[test]
    fn tv_binding_is_composed() {
        let mut single_only = TriggerBindings::new();
        single_only.insert(ButtonTrigger::Single, chord_action(KeyCode::J));
        let profile = profile_with(BTreeMap::from([(RemoteButton::Tv, single_only)]));
        let overlay = compose_overlay(&profile, &base_mappings());
        assert!(overlay.actions.contains_key(&RemoteButton::Tv));
    }

    #[test]
    fn non_profile_buttons_are_never_included() {
        assert!(is_profile_button(RemoteButton::VolumeUp));
        assert!(is_profile_button(RemoteButton::Tv));
        assert!(!is_profile_button(RemoteButton::VolumeMute));
        assert!(!is_profile_button(RemoteButton::Menu));
    }

    #[test]
    fn hold_repeat_and_ok_context_click_inherit_from_base() {
        let mut base = base_mappings();
        let up = base.actions.get_mut(&RemoteButton::Up).unwrap();
        up.hold_repeat = Some(ButtonTrigger::Long);
        up.ok_context_click = false;
        let mut single_only = TriggerBindings::new();
        single_only.insert(ButtonTrigger::Single, chord_action(KeyCode::J));
        let profile = profile_with(BTreeMap::from([(RemoteButton::Up, single_only)]));
        let overlay = compose_overlay(&profile, &base);
        let up = &overlay.actions[&RemoteButton::Up];
        assert_eq!(up.hold_repeat, Some(ButtonTrigger::Long));
    }

    #[test]
    fn empty_profile_composes_an_empty_overlay() {
        let profile = profile_with(BTreeMap::new());
        let overlay = compose_overlay(&profile, &base_mappings());
        assert!(overlay.actions.is_empty());
    }
}
