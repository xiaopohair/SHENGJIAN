//! 内置「推荐按键方案」只读目录（B 方案：只读推荐 → 复制为我的方案 → 使用）。
//!
//! 数据来自 `config/button-profiles.json`（构建时复制进安装包资源，内嵌副本作为
//! dev/测试/资源缺失时的兜底）。推荐方案**不进入用户的方案文档**：桌面端只读列出，
//! 「复制」生成一份普通可编辑 `ButtonProfile`（不落盘），前端插入用户文档后走现有
//! 保存流程。
//!
//! 契约要点：
//! - 每条 profile 是完整的 `ButtonProfile`（字段 + `compatibleModels`）；
//! - `id` 必须以 `preset.` 开头、不能是默认方案、不能重复；
//! - `compatibleModels` 为空 = 所有型号；否则归一化大写后必须包含当前型号；
//! - `bindings` 只允许可编辑按钮（`is_profile_button`）；引用 `run_macro` 时宏必须
//!   存在于推荐宏目录（`crate::recommended_macros`），否则拒绝该方案并记录原因；
//! - `appTargets` 归一化小写去重。

use std::collections::BTreeSet;
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};
use shengjian_contract::{is_profile_button, normalized_model, ButtonProfile, LogicalAction};

/// 推荐方案 id 前缀（与用户方案/默认方案区分）。
pub const BUTTON_PRESET_ID_PREFIX: &str = "preset.";

/// 单份推荐方案数量上限（防御被改坏的配置把内存撑爆）。
pub const MAX_BUTTON_PRESETS: usize = 200;

/// 内嵌兜底副本（与安装包资源里的 `config/button-profiles.json` 同源）。
pub const DEFAULT_BUTTON_PRESETS_JSON: &str = include_str!("../../../config/button-profiles.json");

/// 推荐方案目录文件结构。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonPresetCatalog {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub bundle_version: String,
    #[serde(default)]
    pub profiles: Vec<ButtonPreset>,
}

/// 单条推荐方案：完整 profile 字段 + 声明适配型号。
///
/// 用 `#[serde(flatten)]` 让配置文件里 `id/name/bindings/...` 与
/// `compatibleModels` 平铺在同一对象里（与 Lead 提供的配置格式一致）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonPreset {
    #[serde(flatten)]
    pub profile: ButtonProfile,
    /// 适配型号（大写）；空 = 所有型号。
    #[serde(default)]
    pub compatible_models: Vec<String>,
}

impl ButtonPreset {
    /// 该推荐方案是否对当前型号可见：`compatibleModels` 空 = 全部，否则必须包含。
    ///
    /// 型号未知（`None`）时只显示通用方案，避免把「仅 RC003 适配」的方案推给未知设备。
    pub fn applies_to_model(&self, model: Option<&str>) -> bool {
        match normalized_model(model) {
            None => self.compatible_models.is_empty(),
            Some(model) => {
                self.compatible_models.is_empty() || self.compatible_models.contains(&model)
            }
        }
    }
}

impl ButtonPresetCatalog {
    /// 当前型号可见的推荐方案（文件顺序）。
    pub fn profiles_for_model(&self, model: Option<&str>) -> Vec<&ButtonPreset> {
        self.profiles
            .iter()
            .filter(|preset| preset.applies_to_model(model))
            .collect()
    }

    /// 按 id 查推荐方案。
    pub fn profile(&self, id: &str) -> Option<&ButtonPreset> {
        self.profiles.iter().find(|preset| preset.profile.id == id)
    }
}

#[derive(Debug, Clone, Default)]
struct ButtonPresetStore {
    bundle_version: String,
    profiles: Vec<ButtonPreset>,
}

static STORE: OnceLock<RwLock<ButtonPresetStore>> = OnceLock::new();

fn store() -> &'static RwLock<ButtonPresetStore> {
    STORE.get_or_init(|| RwLock::new(embedded_store()))
}

fn embedded_store() -> ButtonPresetStore {
    parse_catalog(DEFAULT_BUTTON_PRESETS_JSON)
        .and_then(|catalog| {
            validate_catalog(&catalog, crate::recommended_macros::contains_macro)
                .map_err(|issues| issues.join("; "))?;
            Ok(ButtonPresetStore {
                bundle_version: catalog.bundle_version,
                profiles: catalog.profiles,
            })
        })
        .unwrap_or_default()
}

/// 解析 + 归一化推荐方案目录（纯函数，不触碰全局目录，测试可直接使用）。
///
/// 归一化：`deviceModel`/`compatibleModels` 大写去空白、`compatibleModels` 去重、
/// `appTargets` 小写去重、名称去首尾空白。结构/引用校验见 [`validate_catalog`]。
pub fn parse_catalog(json: &str) -> Result<ButtonPresetCatalog, String> {
    let catalog: ButtonPresetCatalog = serde_json::from_str(json)
        .map_err(|error| format!("invalid button preset json: {error}"))?;
    Ok(normalize_catalog(catalog))
}

fn normalize_catalog(mut catalog: ButtonPresetCatalog) -> ButtonPresetCatalog {
    catalog.bundle_version = catalog.bundle_version.trim().to_owned();
    for preset in &mut catalog.profiles {
        preset.profile.name = preset.profile.name.trim().to_owned();
        preset.profile.device_model = normalized_model(preset.profile.device_model.as_deref());
        // 推荐方案是只读模板：时间戳由复制命令盖章。
        preset.profile.created_at = 0;
        preset.profile.app_targets = normalized_targets(&preset.profile.app_targets);
        preset.compatible_models = normalized_models(&preset.compatible_models);
    }
    catalog
}

fn normalized_targets(targets: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for target in targets {
        let trimmed = target.trim().to_lowercase();
        if !trimmed.is_empty() && !normalized.contains(&trimmed) {
            normalized.push(trimmed);
        }
    }
    normalized
}

fn normalized_models(models: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for model in models {
        let Some(model) = normalized_model(Some(model)) else {
            continue;
        };
        if !normalized.contains(&model) {
            normalized.push(model);
        }
    }
    normalized
}

/// 校验推荐方案目录（纯函数；`macro_exists` 注入推荐宏目录查询，便于单测）。
///
/// 返回全部问题的可读原因（含方案 id）；为空表示通过。任一条不合法即整个目录被拒绝，
/// 桌面端启动时回退内嵌副本并记录原因。
pub fn validate_catalog(
    catalog: &ButtonPresetCatalog,
    macro_exists: impl Fn(&str) -> bool,
) -> Result<(), Vec<String>> {
    let mut issues = Vec::new();
    if catalog.profiles.len() > MAX_BUTTON_PRESETS {
        issues.push(format!(
            "too many button presets: {} > {MAX_BUTTON_PRESETS}",
            catalog.profiles.len()
        ));
    }
    let mut seen_ids = BTreeSet::new();
    for preset in &catalog.profiles {
        let id = preset.profile.id.as_str();
        let label = if id.trim().is_empty() {
            "<empty preset id>".to_owned()
        } else {
            id.trim().to_owned()
        };
        if !id.starts_with(BUTTON_PRESET_ID_PREFIX) || id.len() == BUTTON_PRESET_ID_PREFIX.len() {
            issues.push(format!(
                "{label}: id must start with \"{BUTTON_PRESET_ID_PREFIX}\""
            ));
        }
        if id != id.trim() {
            issues.push(format!(
                "{label}: id must not contain surrounding whitespace"
            ));
        }
        if !seen_ids.insert(id.trim().to_owned()) {
            issues.push(format!("{label}: duplicate preset id"));
        }
        if preset.profile.is_default {
            issues.push(format!(
                "{label}: recommended profile cannot be a default profile"
            ));
        }
        if preset.profile.name.trim().is_empty() {
            issues.push(format!("{label}: name must not be empty"));
        }
        if preset
            .compatible_models
            .iter()
            .any(|model| model.is_empty())
        {
            issues.push(format!("{label}: compatibleModels contains an empty model"));
        }
        if preset
            .profile
            .app_targets
            .iter()
            .any(|target| target.trim().is_empty())
        {
            issues.push(format!("{label}: appTargets contains an empty target"));
        }
        for button in preset.profile.bindings.keys() {
            if !is_profile_button(*button) {
                issues.push(format!(
                    "{label}: button {button:?} is not profile-editable"
                ));
            }
        }
        for action in preset
            .profile
            .bindings
            .values()
            .flat_map(|triggers| triggers.values())
        {
            if let LogicalAction::RunMacro { macro_id } = action {
                if !macro_exists(macro_id) {
                    issues.push(format!(
                        "{label}: run_macro references unknown recommended macro {macro_id}"
                    ));
                }
            }
        }
        for button in preset.profile.hold_repeat_by_button.keys() {
            if !is_profile_button(*button) {
                issues.push(format!(
                    "{label}: hold_repeat_by_button contains non-editable button {button:?}"
                ));
            }
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// 当前推荐方案目录（只读快照，按型号过滤）。
pub fn profiles_for_model(model: Option<&str>) -> Vec<ButtonPreset> {
    store()
        .read()
        .map(|guard| {
            guard
                .profiles
                .iter()
                .filter(|preset| preset.applies_to_model(model))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// 按 id 查推荐方案（只读副本）。
pub fn profile(id: &str) -> Option<ButtonPreset> {
    store().read().ok().and_then(|guard| {
        guard
            .profiles
            .iter()
            .find(|preset| preset.profile.id == id)
            .cloned()
    })
}

/// 推荐方案目录版本号（随安装包下发；前端用于提示更新）。
pub fn bundle_version() -> String {
    store()
        .read()
        .map(|guard| guard.bundle_version.clone())
        .unwrap_or_default()
}

/// 复制为我的方案：生成唯一 id、名称加「（我的）」、盖时间戳，其余字段（型号/应用/
/// 绑定/holdRepeat 等）原样保留。**不落盘**——前端插入文档后走现有保存流程。
pub fn copy_as_user_profile(
    preset: &ButtonPreset,
    existing_ids: &[String],
    now_ms: u64,
    created_at: u64,
) -> ButtonProfile {
    let mut profile = preset.profile.clone();
    let base_id = format!("preset-copy-{}-{now_ms}", preset.profile.id);
    let mut candidate = base_id.clone();
    let mut suffix = 1u32;
    while existing_ids.iter().any(|existing| existing == &candidate) {
        candidate = format!("{base_id}-{suffix}");
        suffix += 1;
    }
    profile.id = candidate;
    profile.name = format!("{}（我的）", preset.profile.name.trim());
    profile.created_at = created_at;
    profile.is_default = false;
    profile
}

/// 用 JSON 替换进程内推荐方案目录（桌面端启动时从安装包资源加载；非法回退内嵌）。
pub fn install_from_json(json: &str) -> Result<(), String> {
    let catalog = parse_catalog(json)?;
    validate_catalog(&catalog, crate::recommended_macros::contains_macro)
        .map_err(|issues| issues.join("; "))?;
    let mut slot = store()
        .write()
        .map_err(|_| "button preset catalog lock poisoned".to_owned())?;
    slot.bundle_version = catalog.bundle_version;
    slot.profiles = catalog.profiles;
    Ok(())
}

/// 安装内嵌兜底副本（资源缺失/非法时使用；首次访问也会懒加载它）。
pub fn install_defaults() -> Result<(), String> {
    install_from_json(DEFAULT_BUTTON_PRESETS_JSON)
}

/// 内嵌兜底 JSON（诊断/测试用）。
pub fn default_catalog_json() -> &'static str {
    DEFAULT_BUTTON_PRESETS_JSON
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_contract::{ButtonTrigger, LogicalAction, LogicalButton};
    use std::collections::BTreeMap;

    fn test_profile(id: &str, name: &str) -> ButtonProfile {
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

    fn test_preset(id: &str, name: &str) -> ButtonPreset {
        ButtonPreset {
            profile: test_profile(id, name),
            compatible_models: Vec::new(),
        }
    }

    fn catalog_with(presets: Vec<ButtonPreset>) -> ButtonPresetCatalog {
        ButtonPresetCatalog {
            schema_version: 1,
            bundle_version: "test".to_owned(),
            profiles: presets,
        }
    }

    fn no_macros(_: &str) -> bool {
        false
    }

    /// 内嵌配置（Lead 下发）合法：2 条示例、bundleVersion 正确。
    #[test]
    fn embedded_catalog_is_valid() {
        let catalog = parse_catalog(DEFAULT_BUTTON_PRESETS_JSON).expect("embedded presets");
        assert_eq!(catalog.bundle_version, "2026.11.1");
        assert_eq!(catalog.profiles.len(), 2);
        assert_eq!(catalog.profiles[0].profile.id, "preset.desktop.shortcuts");
        assert_eq!(catalog.profiles[1].profile.id, "preset.netease.controls");
        assert_eq!(
            catalog.profiles[1].compatible_models,
            vec!["RC003".to_owned()]
        );
        validate_catalog(&catalog, no_macros).expect("embedded config must validate");
    }

    /// 型号筛选：通用方案对所有型号可见；`compatibleModels` 非空时只匹配该型号；
    /// 型号未知时只列通用方案。
    #[test]
    fn filters_presets_by_model() {
        let catalog = parse_catalog(DEFAULT_BUTTON_PRESETS_JSON).expect("embedded presets");
        let ids_for = |model: Option<&str>| -> Vec<&str> {
            catalog
                .profiles_for_model(model)
                .into_iter()
                .map(|preset| preset.profile.id.as_str())
                .collect()
        };
        assert_eq!(
            ids_for(Some("rc003")),
            vec!["preset.desktop.shortcuts", "preset.netease.controls"],
            "型号归一化后 RC003 两条都可见"
        );
        assert_eq!(
            ids_for(Some("ARN9")),
            vec!["preset.desktop.shortcuts"],
            "仅 RC003 适配的方案不应出现在 ARN9 下"
        );
        assert_eq!(ids_for(None), vec!["preset.desktop.shortcuts"]);
    }

    /// 归一化：型号大写去空白、compatibleModels/appTargets 去重、名称 trim。
    #[test]
    fn normalizes_models_and_targets() {
        let json = r#"{
            "schemaVersion": 1,
            "bundleVersion": " 1.2.3 ",
            "profiles": [{
                "id": "preset.probe",
                "name": "  探针  ",
                "deviceModel": " rc003 ",
                "compatibleModels": [" rc003 ", "RC003", "", "arn9"],
                "appTargets": ["CloudMusic.EXE", "cloudmusic.exe", " "],
                "autoApply": true,
                "bindings": {}
            }]
        }"#;
        let catalog = parse_catalog(json).expect("parse");
        let preset = &catalog.profiles[0];
        assert_eq!(catalog.bundle_version, "1.2.3");
        assert_eq!(preset.profile.name, "探针");
        assert_eq!(preset.profile.device_model.as_deref(), Some("RC003"));
        assert_eq!(
            preset.compatible_models,
            vec!["RC003".to_owned(), "ARN9".to_owned()]
        );
        assert_eq!(
            preset.profile.app_targets,
            vec!["cloudmusic.exe".to_owned()]
        );
    }

    /// 结构校验：id 前缀、不可默认、id 唯一、名称非空、按钮可编辑、型号非空。
    #[test]
    fn rejects_invalid_presets() {
        let mut no_prefix = test_preset("custom.one", "无前缀");
        no_prefix.profile.name = "无前缀".to_owned();

        let mut default_profile = test_preset("preset.default", "默认");
        default_profile.profile.is_default = true;

        let mut empty_name = test_preset("preset.empty", "");
        empty_name.profile.name = "   ".to_owned();

        let mut mute = test_preset("preset.mute", "静音键");
        mute.profile.bindings.insert(
            LogicalButton::VolumeMute,
            BTreeMap::from([(
                ButtonTrigger::Single,
                LogicalAction::Shortcut {
                    label: "静音".to_owned(),
                    keys: vec!["volume_mute".to_owned()],
                },
            )]),
        );

        let duplicate = test_preset("preset.dupe", "重复");

        let catalog = catalog_with(vec![
            no_prefix,
            default_profile,
            empty_name,
            mute,
            duplicate.clone(),
            duplicate,
        ]);
        let issues = validate_catalog(&catalog, no_macros).expect_err("must reject");
        let joined = issues.join("\n");
        assert!(joined.contains("must start with"), "issues: {joined}");
        assert!(joined.contains("cannot be a default"), "issues: {joined}");
        assert!(
            joined.contains("name must not be empty"),
            "issues: {joined}"
        );
        assert!(joined.contains("not profile-editable"), "issues: {joined}");
        assert!(joined.contains("duplicate preset id"), "issues: {joined}");
    }

    /// `run_macro` 引用必须存在于推荐宏目录：缺失 → 拒绝该方案并给出原因；
    /// 存在 → 通过（`macro_exists` 可注入，测试无需真的安装宏目录）。
    #[test]
    fn rejects_run_macro_reference_missing_from_recommended_macros() {
        let mut preset = test_preset("preset.macro", "宏方案");
        preset.profile.bindings.insert(
            LogicalButton::Ok,
            BTreeMap::from([(
                ButtonTrigger::Single,
                LogicalAction::RunMacro {
                    macro_id: "macro.missing".to_owned(),
                },
            )]),
        );
        let catalog = catalog_with(vec![preset]);

        let issues = validate_catalog(&catalog, no_macros).expect_err("missing macro rejected");
        assert!(
            issues
                .join("\n")
                .contains("unknown recommended macro macro.missing"),
            "issues: {issues:?}"
        );

        validate_catalog(&catalog, |id| id == "macro.missing").expect("known macro accepted");
    }

    /// 复制字段：唯一 id（与现有方案去重）、名称加「（我的）」、保留型号/应用/绑定、
    /// 时间戳由复制盖章、非默认方案。
    #[test]
    fn copies_preset_into_editable_profile() {
        let mut source = test_preset("preset.media", "媒体控制");
        source.profile.device_model = Some("RC003".to_owned());
        source.profile.app_targets = vec!["netease_music".to_owned()];
        source.profile.auto_apply = true;
        source.profile.bindings.insert(
            LogicalButton::Up,
            BTreeMap::from([(
                ButtonTrigger::Single,
                LogicalAction::Shortcut {
                    label: "音量 +".to_owned(),
                    keys: vec!["volume_up".to_owned()],
                },
            )]),
        );

        let generated = copy_as_user_profile(&source, &[], 1_700_000_000_123, 1_700_000_000);
        assert_eq!(
            generated.id, "preset-copy-preset.media-1700000000123",
            "id 由源 id + 毫秒时间戳生成"
        );
        assert_eq!(generated.name, "媒体控制（我的）");
        assert_eq!(generated.created_at, 1_700_000_000);
        assert!(!generated.is_default);
        assert_eq!(generated.device_model.as_deref(), Some("RC003"));
        assert_eq!(generated.app_targets, vec!["netease_music".to_owned()]);
        assert_eq!(generated.bindings, source.profile.bindings);

        // 与现有方案 id 冲突时追加序号，仍保持唯一。
        let existing = vec!["preset-copy-preset.media-1700000000123".to_owned()];
        let regenerated = copy_as_user_profile(&source, &existing, 1_700_000_000_123, 0);
        assert_eq!(
            regenerated.id, "preset-copy-preset.media-1700000000123-1",
            "冲突时追加序号"
        );
    }

    /// 全局目录安装：内嵌副本可安装，查询 API 与只读快照一致。
    #[test]
    fn install_defaults_exposes_read_only_catalog() {
        install_defaults().expect("install embedded presets");
        assert_eq!(bundle_version(), "2026.11.1");
        let rc003 = profiles_for_model(Some("RC003"));
        assert_eq!(rc003.len(), 2);
        let found = profile("preset.netease.controls").expect("preset lookup");
        assert_eq!(found.profile.name, "网易云音乐按键");
        assert!(profile("preset.missing").is_none());
    }
}
