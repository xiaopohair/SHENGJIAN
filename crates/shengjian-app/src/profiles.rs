use crate::ports::ForegroundPort;
use shengjian_contract::{
    compose_profile_mappings, ButtonMappingSet, ButtonProfile, ButtonProfilesDocument,
    GLOBAL_PROFILE_ID,
};
use std::sync::{Arc, Mutex};

/// Core profile selection and overlay policy.
///
/// Manual activation wins; otherwise an auto-apply profile whose executable
/// matches the current foreground app wins; otherwise the base mapping is used.
pub struct ButtonProfileService {
    foreground: Arc<dyn ForegroundPort>,
    document: Mutex<ButtonProfilesDocument>,
    base: Mutex<ButtonMappingSet>,
}

impl ButtonProfileService {
    pub fn new(foreground: Arc<dyn ForegroundPort>) -> Self {
        Self {
            foreground,
            // Normalizing the empty document materializes the global profile, so
            // a fresh install already has a working default instead of no profiles.
            document: Mutex::new(ButtonProfilesDocument::default().normalized()),
            base: Mutex::new(ButtonMappingSet::default()),
        }
    }

    pub fn document(&self) -> ButtonProfilesDocument {
        self.document.lock().expect("profiles document").clone()
    }

    pub fn set_document(&self, document: ButtonProfilesDocument) {
        *self.document.lock().expect("profiles document") = document.normalized();
    }

    pub fn base_mappings(&self) -> ButtonMappingSet {
        self.base.lock().expect("base mappings").clone()
    }

    pub fn set_base_mappings(&self, mappings: ButtonMappingSet) {
        *self.base.lock().expect("base mappings") = mappings;
    }

    pub fn activate(&self, profile_id: &str) -> bool {
        let mut document = self.document.lock().expect("profiles document");
        if document.profile(profile_id).is_none() {
            return false;
        }
        document.active_manual_profile_id = Some(profile_id.to_owned());
        true
    }

    pub fn clear_activation(&self) {
        self.document
            .lock()
            .expect("profiles document")
            .active_manual_profile_id = None;
    }

    pub fn active_profile(&self) -> Option<ButtonProfile> {
        self.selected_profile()
    }

    pub fn active_profile_id(&self) -> Option<String> {
        self.selected_profile().map(|profile| profile.id)
    }

    fn selected_profile(&self) -> Option<ButtonProfile> {
        let document = self.document.lock().expect("profiles document");
        let executable = self.foreground.executable();
        let profile = match &document.active_manual_profile_id {
            Some(id) => document.profile(id),
            None => executable.as_deref().and_then(|executable| {
                document
                    .profiles
                    .iter()
                    .find(|profile| profile.auto_apply && profile.matches_executable(executable))
            }),
        }?;
        Some(profile.clone())
    }

    /// Effective baseline: the factory mapping set with the global profile on top.
    ///
    /// Layering is `button-mappings.json` → global profile → active profile, so a
    /// button the global profile edits becomes the default for every other profile.
    fn layered_base(&self, document: &ButtonProfilesDocument) -> ButtonMappingSet {
        let factory = self.base.lock().expect("base mappings").clone();
        match document.profile(GLOBAL_PROFILE_ID) {
            Some(global) => compose_profile_mappings(global, &factory),
            None => factory,
        }
    }

    /// Compute the effective complete mapping set for the current foreground.
    pub fn sync_foreground(&self) -> ButtonMappingSet {
        let document = self.document.lock().expect("profiles document").clone();
        let base = self.layered_base(&document);
        match self.selected_profile() {
            Some(profile) => compose_profile_mappings(&profile, &base),
            // No manual/auto profile: the global profile itself is in effect.
            None => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_contract::{ButtonActions, ButtonTrigger, LogicalAction, LogicalButton};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeForeground {
        executable: Mutex<Option<String>>,
    }

    impl FakeForeground {
        fn set(&self, value: Option<&str>) {
            *self.executable.lock().expect("executable") = value.map(str::to_owned);
        }
    }

    impl ForegroundPort for FakeForeground {
        fn executable(&self) -> Option<String> {
            self.executable.lock().expect("executable").clone()
        }
    }

    fn shortcut(key: &str) -> LogicalAction {
        LogicalAction::Shortcut {
            label: key.to_owned(),
            keys: vec![key.to_owned()],
        }
    }

    fn base() -> ButtonMappingSet {
        let mut mappings = ButtonMappingSet::default();
        mappings.set_actions(
            LogicalButton::Up,
            ButtonActions {
                single: shortcut("W"),
                long: shortcut("PageUp"),
                ..ButtonActions::default()
            },
        );
        mappings
    }

    fn profile(id: &str, exe: &str) -> ButtonProfile {
        let mut bindings = std::collections::BTreeMap::new();
        bindings.insert(ButtonTrigger::Single, shortcut("J"));
        let mut bindings_by_button = std::collections::BTreeMap::new();
        bindings_by_button.insert(LogicalButton::Up, bindings);
        ButtonProfile {
            id: id.to_owned(),
            name: id.to_owned(),
            created_at: 0,
            app_targets: vec![exe.to_owned()],
            auto_apply: true,
            hold_repeat: None,
            hold_repeat_by_button: std::collections::BTreeMap::new(),
            voice_input_tool: None,
            bindings: bindings_by_button,
        }
    }

    #[test]
    fn global_profile_is_the_default_and_feeds_other_profiles() {
        let foreground = Arc::new(FakeForeground::default());
        let service = ButtonProfileService::new(foreground.clone());
        service.set_base_mappings(base());

        // Global overrides Up/single; a normal profile overrides Up/long only.
        let mut global = ButtonProfile {
            id: "global".to_owned(),
            name: "全局".to_owned(),
            created_at: 0,
            app_targets: Vec::new(),
            auto_apply: false,
            hold_repeat: None,
            hold_repeat_by_button: std::collections::BTreeMap::new(),
            voice_input_tool: None,
            bindings: std::collections::BTreeMap::new(),
        };
        let mut global_up = std::collections::BTreeMap::new();
        global_up.insert(ButtonTrigger::Single, shortcut("G"));
        global.bindings.insert(LogicalButton::Up, global_up);

        let mut scoped = profile("p2", "cloudmusic.exe");
        let mut scoped_up = std::collections::BTreeMap::new();
        scoped_up.insert(ButtonTrigger::Long, shortcut("L"));
        scoped.bindings.insert(LogicalButton::Up, scoped_up);

        service.set_document(ButtonProfilesDocument {
            profiles: vec![global, scoped],
            ..ButtonProfilesDocument::default()
        });

        // No manual profile → the global profile is in effect.
        let effective = service.sync_foreground();
        assert_eq!(effective.actions[&LogicalButton::Up].single, shortcut("G"));
        assert_eq!(
            effective.actions[&LogicalButton::Up].long,
            shortcut("PageUp"),
            "unbound trigger still inherits the factory base"
        );

        // A normal profile inherits the global single and overrides long.
        foreground.set(Some("cloudmusic.exe"));
        let effective = service.sync_foreground();
        assert_eq!(
            effective.actions[&LogicalButton::Up].single,
            shortcut("G"),
            "inherits the global profile, not the factory value"
        );
        assert_eq!(effective.actions[&LogicalButton::Up].long, shortcut("L"));
    }

    #[test]
    fn global_profile_survives_document_replacement() {
        let foreground = Arc::new(FakeForeground::default());
        let service = ButtonProfileService::new(foreground);
        // A document that dropped the global profile gets it back via normalization.
        service.set_document(ButtonProfilesDocument {
            profiles: vec![profile("p1", "a.exe")],
            ..ButtonProfilesDocument::default()
        });
        let document = service.document();
        assert_eq!(document.profiles[0].id, "global");
        assert_eq!(document.profiles.len(), 2);
    }

    #[test]
    fn auto_profile_applies_on_matching_foreground_and_clears_afterward() {
        let foreground = Arc::new(FakeForeground::default());
        let service = ButtonProfileService::new(foreground.clone());
        service.set_base_mappings(base());
        service.set_document(ButtonProfilesDocument {
            profiles: vec![profile("p1", "cloudmusic.exe")],
            ..ButtonProfilesDocument::default()
        });

        foreground.set(Some("CloudMusic.exe"));
        let effective = service.sync_foreground();
        assert_eq!(effective.actions[&LogicalButton::Up].single, shortcut("J"));
        assert_eq!(
            effective.actions[&LogicalButton::Up].long,
            shortcut("PageUp")
        );

        foreground.set(Some("notepad.exe"));
        assert_eq!(
            service.sync_foreground().actions[&LogicalButton::Up].single,
            shortcut("W")
        );
    }

    #[test]
    fn manual_activation_wins_and_clear_returns_to_base() {
        let foreground = Arc::new(FakeForeground::default());
        let service = ButtonProfileService::new(foreground.clone());
        service.set_base_mappings(base());
        service.set_document(ButtonProfilesDocument {
            profiles: vec![profile("p1", "cloudmusic.exe")],
            ..ButtonProfilesDocument::default()
        });
        foreground.set(Some("notepad.exe"));
        assert!(service.activate("p1"));
        assert_eq!(
            service.sync_foreground().actions[&LogicalButton::Up].single,
            shortcut("J")
        );
        service.clear_activation();
        assert_eq!(
            service.sync_foreground().actions[&LogicalButton::Up].single,
            shortcut("W")
        );
    }
}
