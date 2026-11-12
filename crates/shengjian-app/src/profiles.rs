use crate::ports::ForegroundPort;
use shengjian_contract::{
    compose_profile_mappings, normalized_model, ButtonMappingSet, ButtonProfile,
    ButtonProfilesDocument,
};
use std::sync::{Arc, Mutex};

/// Core profile selection and overlay policy.
///
/// 方案按**设备型号**隔离（`ButtonProfile::device_model`）：
/// 手动激活 > 匹配前台的自动方案 > 该型号默认方案；型号未知时只有通用方案可见。
/// 分层是 `button-mappings.json` → 该型号默认方案 → 当前方案。
pub struct ButtonProfileService {
    foreground: Arc<dyn ForegroundPort>,
    document: Mutex<ButtonProfilesDocument>,
    base: Mutex<ButtonMappingSet>,
    current_model: Mutex<Option<String>>,
}

impl ButtonProfileService {
    pub fn new(foreground: Arc<dyn ForegroundPort>) -> Self {
        Self {
            foreground,
            // Normalizing the empty document materializes the generic default profile,
            // so a fresh install already has a working baseline instead of no profiles.
            document: Mutex::new(ButtonProfilesDocument::default().normalized()),
            base: Mutex::new(ButtonMappingSet::default()),
            current_model: Mutex::new(None),
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

    pub fn current_device_model(&self) -> Option<String> {
        self.current_model.lock().expect("current model").clone()
    }

    /// 切换当前设备型号；返回文档是否发生变化（调用方据此决定是否落盘）。
    ///
    /// 该型号的默认方案不存在时补建一份：界面需要它作为可编辑的基线，其他方案也
    /// 继承它，否则「刚换到新型号」会看到一份空列表。
    pub fn set_current_device_model(&self, model: Option<&str>) -> bool {
        let model = normalized_model(model);
        {
            let mut current = self.current_model.lock().expect("current model");
            if *current == model {
                return false;
            }
            *current = model.clone();
        }
        let mut document = self.document.lock().expect("profiles document");
        if document.default_profile_for(model.as_deref()).is_some() {
            return false;
        }
        let snapshot = document.clone();
        let profiles = snapshot
            .profiles
            .iter()
            .cloned()
            .chain(std::iter::once(ButtonProfile::default_for(
                model.as_deref(),
            )))
            .collect();
        *document = ButtonProfilesDocument {
            profiles,
            ..snapshot
        }
        .normalized();
        true
    }

    pub fn activate(&self, profile_id: &str) -> bool {
        let model = self.current_device_model();
        let mut document = self.document.lock().expect("profiles document");
        match document.profile(profile_id) {
            // 别的型号的方案不能激活；默认方案等同于「不激活」。
            Some(profile) if profile.applies_to_model(model.as_deref()) && !profile.is_default => {
                document.active_manual_profile_id = Some(profile_id.to_owned());
                true
            }
            _ => false,
        }
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
        let model = self.current_device_model();
        let document = self.document.lock().expect("profiles document");
        let executable = self.foreground.executable();
        let usable = |profile: &ButtonProfile| {
            !profile.is_default && profile.applies_to_model(model.as_deref())
        };
        let profile = match &document.active_manual_profile_id {
            Some(id) => document.profile(id).filter(|profile| usable(profile)),
            None => executable.as_deref().and_then(|executable| {
                document.profiles.iter().find(|profile| {
                    usable(profile) && profile.auto_apply && profile.matches_executable(executable)
                })
            }),
        }?;
        Some(profile.clone())
    }

    /// Effective baseline: the factory mapping set with the current model's default
    /// profile on top.
    ///
    /// Layering is `button-mappings.json` → 该型号默认方案 → 当前方案，所以默认方案里
    /// 编辑过的按键会成为该型号下所有方案的默认值。型号没有专属默认方案时退回通用方案。
    fn layered_base(&self, document: &ButtonProfilesDocument) -> ButtonMappingSet {
        let factory = self.base.lock().expect("base mappings").clone();
        let model = self.current_device_model();
        let default = document
            .default_profile_for(model.as_deref())
            .or_else(|| document.default_profile_for(None));
        match default {
            Some(profile) => compose_profile_mappings(profile, &factory),
            None => factory,
        }
    }

    /// Compute the effective complete mapping set for the current foreground.
    pub fn sync_foreground(&self) -> ButtonMappingSet {
        let document = self.document.lock().expect("profiles document").clone();
        let base = self.layered_base(&document);
        match self.selected_profile() {
            Some(profile) => compose_profile_mappings(&profile, &base),
            // 没有手动/自动方案：该型号的默认方案本身就在生效。
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
            device_model: None,
            is_default: false,
            bindings: bindings_by_button,
        }
    }

    fn model_profile(id: &str, exe: &str, model: &str) -> ButtonProfile {
        ButtonProfile {
            device_model: Some(model.to_owned()),
            ..profile(id, exe)
        }
    }

    #[test]
    fn model_default_profile_is_the_baseline_and_feeds_other_profiles() {
        let foreground = Arc::new(FakeForeground::default());
        let service = ButtonProfileService::new(foreground.clone());
        service.set_base_mappings(base());
        // 换到 RC003：服务应自动补一份该型号的默认方案。
        assert!(service.set_current_device_model(Some("RC003")));

        // 默认方案改 Up/single；普通方案只改 Up/long。
        let mut document = service.document();
        let default = document
            .default_profile_for(Some("RC003"))
            .expect("RC003 default")
            .clone();
        assert!(default.is_default);
        let default_index = document
            .profiles
            .iter()
            .position(|profile| profile.id == default.id)
            .expect("index");
        let mut default_up = std::collections::BTreeMap::new();
        default_up.insert(ButtonTrigger::Single, shortcut("G"));
        document.profiles[default_index]
            .bindings
            .insert(LogicalButton::Up, default_up);
        let mut scoped = model_profile("p2", "cloudmusic.exe", "RC003");
        let mut scoped_up = std::collections::BTreeMap::new();
        scoped_up.insert(ButtonTrigger::Long, shortcut("L"));
        scoped.bindings.insert(LogicalButton::Up, scoped_up);
        document.profiles.push(scoped);
        service.set_document(document);

        // 没有手动方案 → 该型号默认方案生效。
        let effective = service.sync_foreground();
        assert_eq!(effective.actions[&LogicalButton::Up].single, shortcut("G"));
        assert_eq!(
            effective.actions[&LogicalButton::Up].long,
            shortcut("PageUp"),
            "未绑定的触发仍继承工厂基线"
        );

        // 普通方案继承默认方案的 single，自己覆盖 long。
        foreground.set(Some("cloudmusic.exe"));
        let effective = service.sync_foreground();
        assert_eq!(
            effective.actions[&LogicalButton::Up].single,
            shortcut("G"),
            "应继承该型号默认方案，而不是工厂值"
        );
        assert_eq!(effective.actions[&LogicalButton::Up].long, shortcut("L"));
    }

    #[test]
    fn default_profile_survives_document_replacement() {
        let foreground = Arc::new(FakeForeground::default());
        let service = ButtonProfileService::new(foreground);
        // 丢掉了默认方案的文档在归一化后会重建成通用默认。
        service.set_document(ButtonProfilesDocument {
            profiles: vec![profile("p1", "a.exe")],
            ..ButtonProfilesDocument::default()
        });
        let document = service.document();
        assert!(document.default_profile_for(None).is_some());
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

    #[test]
    fn model_profiles_do_not_leak_across_models() {
        let foreground = Arc::new(FakeForeground::default());
        let service = ButtonProfileService::new(foreground);
        service.set_base_mappings(base());

        let mut rc003 = model_profile("rc003-p", "cloudmusic.exe", "RC003");
        rc003.auto_apply = false;
        let mut arn9 = model_profile("arn9-p", "cloudmusic.exe", "ARN9");
        arn9.auto_apply = false;
        service.set_document(ButtonProfilesDocument {
            profiles: vec![rc003, arn9],
            ..ButtonProfilesDocument::default()
        });

        // RC003 手动激活后，切到 ARN9 不能继续沿用 RC003 的方案。
        service.set_current_device_model(Some("RC003"));
        assert!(service.activate("rc003-p"));
        assert_eq!(service.active_profile_id().as_deref(), Some("rc003-p"));

        service.set_current_device_model(Some("ARN9"));
        assert_eq!(
            service.active_profile_id(),
            None,
            "切到 ARN9 后 RC003 的方案必须失效"
        );
        // 反过来也不能激活别的型号的方案。
        assert!(!service.activate("rc003-p"));
        assert_eq!(
            service.sync_foreground().actions[&LogicalButton::Up].single,
            shortcut("W")
        );
    }

    #[test]
    fn switching_model_creates_its_default_profile() {
        let foreground = Arc::new(FakeForeground::default());
        let service = ButtonProfileService::new(foreground);
        service.set_current_device_model(Some("RC003"));
        service.set_current_device_model(Some("ARN9"));
        let document = service.document();
        assert!(document.default_profile_for(Some("RC003")).is_some());
        assert!(document.default_profile_for(Some("ARN9")).is_some());
        // 未知型号仍有通用兜底。
        assert!(document.default_profile_for(None).is_some());
        // 型号没变时不重复补建。
        assert!(!service.set_current_device_model(Some("arn9")));
    }
}
