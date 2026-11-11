//! One-way migration from the legacy Windows button models to core contract
//! types. The legacy files are read by the host; this module only translates
//! already-parsed values.

use serde::{Deserialize, Serialize};
use shengjian_windows::raw_input::RemoteButton;
use shengjian_windows::send_input::{
    ButtonAction as LegacyAction, ButtonActions as LegacyActions, ButtonMappings as LegacyMappings,
    ButtonTrigger as LegacyTrigger, KeyCode, MoveDirection, ScrollDirection,
};
use std::collections::BTreeMap;

use shengjian_contract::{
    ButtonActions as ContractActions, ButtonMappingSet, ButtonProfilesDocument, ButtonTrigger,
    FocusStrategy, LogicalAction, LogicalButton, MouseActionKind, TriggerBindings,
    BUTTON_PROFILES_SCHEMA_VERSION,
};
// The legacy profile model used to live in a separate crate. Only three types are
// needed for this one-way migration, so they are declared here and the crate is gone.
pub type LegacyTriggerBindings = BTreeMap<LegacyTrigger, LegacyAction>;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub app_targets: Vec<String>,
    #[serde(default)]
    pub auto_apply: bool,
    #[serde(default)]
    pub bindings: BTreeMap<RemoteButton, LegacyTriggerBindings>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyProfiles {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub profiles: Vec<LegacyProfile>,
    #[serde(default)]
    pub active_manual_profile_id: Option<String>,
}
pub fn migrate_button_mappings(legacy: &LegacyMappings) -> ButtonMappingSet {
    let actions = legacy
        .actions
        .iter()
        .map(|(button, actions)| (map_button(*button), migrate_actions(actions)))
        .collect::<BTreeMap<_, _>>();
    ButtonMappingSet {
        enabled: legacy.enabled,
        actions,
    }
}

pub fn migrate_button_profiles_document(legacy: &LegacyProfiles) -> ButtonProfilesDocument {
    ButtonProfilesDocument {
        schema_version: BUTTON_PROFILES_SCHEMA_VERSION,
        profiles: legacy
            .profiles
            .iter()
            .map(migrate_profile)
            .collect::<Vec<_>>(),
        active_manual_profile_id: legacy.active_manual_profile_id.clone(),
    }
    .normalized()
}

fn migrate_profile(profile: &LegacyProfile) -> shengjian_contract::ButtonProfile {
    shengjian_contract::ButtonProfile {
        id: profile.id.clone(),
        name: profile.name.clone(),
        created_at: profile.created_at,
        app_targets: profile.app_targets.clone(),
        auto_apply: profile.auto_apply,
        hold_repeat: None,
        hold_repeat_by_button: BTreeMap::new(),
        voice_input_tool: None,
        bindings: profile
            .bindings
            .iter()
            .map(|(button, bindings)| (map_button(*button), migrate_bindings(bindings)))
            .collect::<BTreeMap<_, _>>(),
    }
}

fn migrate_bindings(bindings: &LegacyTriggerBindings) -> TriggerBindings {
    bindings
        .iter()
        .map(|(trigger, action)| (map_trigger(*trigger), migrate_action(action)))
        .collect()
}

fn migrate_actions(actions: &LegacyActions) -> ContractActions {
    ContractActions {
        single: migrate_action(&actions.single),
        double: migrate_action(&actions.double),
        long: migrate_action(&actions.long),
        hold_repeat: actions.hold_repeat.map(map_trigger),
        ok_context_click: actions.ok_context_click,
    }
}

fn migrate_action(action: &LegacyAction) -> LogicalAction {
    match action {
        LegacyAction::Disabled => LogicalAction::Disabled,
        LegacyAction::Shortcut { chord } => LogicalAction::Shortcut {
            label: chord
                .keys
                .iter()
                .map(|key| key_name(*key))
                .collect::<Vec<_>>()
                .join("+"),
            keys: chord.keys.iter().map(|key| key_name(*key)).collect(),
        },
        LegacyAction::OpenApp { target } => LogicalAction::OpenApp {
            app: target.clone(),
            focus: FocusStrategy::Direct,
        },
        LegacyAction::FocusInput => LogicalAction::FocusInput,
        LegacyAction::Scroll { direction, steps } => LogicalAction::Mouse {
            kind: MouseActionKind::Scroll,
            direction: Some(match direction {
                ScrollDirection::Up => shengjian_contract::Direction::Up,
                ScrollDirection::Down => shengjian_contract::Direction::Down,
            }),
            amount: *steps,
        },
        LegacyAction::MouseClick { .. } => {
            // The core contract currently has one click action (left click).
            LogicalAction::Mouse {
                kind: MouseActionKind::Click,
                direction: None,
                amount: 0,
            }
        }
        LegacyAction::MouseMove {
            direction,
            distance,
        } => LogicalAction::Mouse {
            kind: MouseActionKind::Move,
            direction: Some(match direction {
                MoveDirection::Up => shengjian_contract::Direction::Up,
                MoveDirection::Down => shengjian_contract::Direction::Down,
                MoveDirection::Left => shengjian_contract::Direction::Left,
                MoveDirection::Right => shengjian_contract::Direction::Right,
            }),
            amount: *distance,
        },
        LegacyAction::RunMacro { macro_id, .. } => LogicalAction::RunMacro {
            macro_id: macro_id.clone(),
        },
        LegacyAction::OpenCommonPhrases => LogicalAction::CommonPhrase,
    }
}

fn map_trigger(trigger: LegacyTrigger) -> ButtonTrigger {
    match trigger {
        LegacyTrigger::Single => ButtonTrigger::Single,
        LegacyTrigger::Double => ButtonTrigger::Double,
        LegacyTrigger::Long => ButtonTrigger::Long,
    }
}

fn map_button(button: RemoteButton) -> LogicalButton {
    crate::map_button(button)
}

fn key_name(key: KeyCode) -> String {
    serde_json::to_string(&key)
        .ok()
        .and_then(|value| {
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("{key:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_windows::send_input::{KeyChord, KeyCode};

    fn shortcut(key: KeyCode) -> LegacyAction {
        LegacyAction::Shortcut {
            chord: KeyChord { keys: vec![key] },
        }
    }

    fn actions(single: LegacyAction, long: LegacyAction) -> LegacyActions {
        LegacyActions {
            single,
            double: LegacyAction::Disabled,
            long,
            hold_repeat: Some(LegacyTrigger::Single),
            ok_context_click: true,
        }
    }

    #[test]
    fn migrates_legacy_button_mappings() {
        let mut legacy = LegacyMappings::default();
        legacy.enabled = true;
        legacy.actions.insert(
            RemoteButton::Up,
            actions(shortcut(KeyCode::W), shortcut(KeyCode::PageUp)),
        );
        let migrated = migrate_button_mappings(&legacy);
        let up = &migrated.actions[&LogicalButton::Up];
        assert_eq!(
            up.single,
            LogicalAction::Shortcut {
                label: "w".to_owned(),
                keys: vec!["w".to_owned()],
            }
        );
        assert_eq!(up.hold_repeat, Some(ButtonTrigger::Single));
        assert!(up.ok_context_click);
        assert_eq!(
            up.long,
            LogicalAction::Shortcut {
                label: "page_up".to_owned(),
                keys: vec!["page_up".to_owned()],
            }
        );
    }

    #[test]
    fn migrates_legacy_button_profile_document() {
        let mut bindings = BTreeMap::new();
        bindings.insert(LegacyTrigger::Single, shortcut(KeyCode::J));
        let mut profile = LegacyProfile {
            id: "p1".to_owned(),
            name: "Music".to_owned(),
            created_at: 1,
            app_targets: vec!["cloudmusic.exe".to_owned()],
            auto_apply: true,
            bindings: BTreeMap::new(),
        };
        profile.bindings.insert(RemoteButton::Up, bindings);
        let legacy = LegacyProfiles {
            schema_version: 1,
            profiles: vec![profile],
            active_manual_profile_id: Some("p1".to_owned()),
        };
        let migrated = migrate_button_profiles_document(&legacy);
        // Normalization always adds the global profile at index 0.
        assert_eq!(migrated.profiles.len(), 2);
        assert_eq!(migrated.profiles[0].id, "global");
        assert_eq!(migrated.profiles[1].id, "p1");
        assert_eq!(migrated.profiles[1].auto_apply, true);
        assert_eq!(
            migrated.profiles[1].bindings[&LogicalButton::Up][&ButtonTrigger::Single],
            LogicalAction::Shortcut {
                label: "j".to_owned(),
                keys: vec!["j".to_owned()],
            }
        );
        assert_eq!(migrated.active_manual_profile_id.as_deref(), Some("p1"));
    }
}
