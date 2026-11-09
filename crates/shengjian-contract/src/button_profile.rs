//! Button profile document and per-trigger overlay composition.
//!
//! A profile is sparse: a missing `(button, trigger)` inherits from the base
//! mapping; an explicit entry (including `Disabled`) overrides. TV and Menu
//! are profile-editable (12-key boundary); Mute stays base-owned.

use crate::model::{ButtonActions, ButtonMappingSet, ButtonTrigger, LogicalAction, LogicalButton};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const BUTTON_PROFILES_SCHEMA_VERSION: u32 = 4;

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
    PROFILE_BUTTONS.contains(&button)
}

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
    #[serde(default)]
    pub bindings: BTreeMap<LogicalButton, TriggerBindings>,
}

impl ButtonProfile {
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
        let mut claimed = BTreeSet::new();
        for profile in &mut self.profiles {
            profile
                .app_targets
                .retain(|target| claimed.insert(target.clone()));
        }
        if let Some(active) = &self.active_manual_profile_id {
            if !self.profiles.iter().any(|profile| &profile.id == active) {
                self.active_manual_profile_id = None;
            }
        }
        self.schema_version = BUTTON_PROFILES_SCHEMA_VERSION;
        self
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
    for button in PROFILE_BUTTONS {
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
        for button in PROFILE_BUTTONS {
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
            bindings: BTreeMap::new(),
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
        assert_eq!(document.profiles.len(), 2);
        assert_eq!(document.profiles[0].name, "第一");
        assert_eq!(
            document.profiles[0].app_targets,
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
    fn tv_and_menu_are_profile_editable_but_mute_is_not() {
        assert!(is_profile_button(LogicalButton::Tv));
        assert!(is_profile_button(LogicalButton::Menu));
        assert!(!is_profile_button(LogicalButton::VolumeMute));
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
}
