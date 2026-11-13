//! 内置「推荐宏」只读目录（P2）。
//!
//! 数据来自 `config/recommended-macros.json`（构建时复制进安装包资源，
//! 内嵌副本作为 dev/测试/资源缺失时的兜底）。本模块只负责**加载、校验、查询**：
//! 推荐宏尚未接入组合动作运行时——推荐按键方案里引用 `run_macro` 的按钮在执行前
//! 还需要把推荐宏注册进宏执行器（后续任务）。同 id 时用户宏优先由调用方保证
//! （本目录只提供只读查询，不参与用户宏库合并/落盘）。

use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::macros::{
    find_cycle, validate_definition, MacroDefinition, MacroIssue, MAX_MACRO_DEFINITIONS,
};

/// 内嵌兜底副本（与安装包资源里的 `config/recommended-macros.json` 同源）。
pub const DEFAULT_RECOMMENDED_MACROS_JSON: &str =
    include_str!("../../../config/recommended-macros.json");

/// 推荐宏目录文件结构。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedMacroCatalog {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub bundle_version: String,
    #[serde(default)]
    pub macros: Vec<MacroDefinition>,
}

#[derive(Debug, Clone, Default)]
struct RecommendedMacroStore {
    bundle_version: String,
    macros: Vec<MacroDefinition>,
}

static STORE: OnceLock<RwLock<RecommendedMacroStore>> = OnceLock::new();

fn store() -> &'static RwLock<RecommendedMacroStore> {
    STORE.get_or_init(|| RwLock::new(embedded_store()))
}

fn embedded_store() -> RecommendedMacroStore {
    parse_catalog(DEFAULT_RECOMMENDED_MACROS_JSON)
        .map(|catalog| RecommendedMacroStore {
            bundle_version: catalog.bundle_version,
            macros: catalog.macros,
        })
        .unwrap_or_default()
}

/// 解析 + 校验推荐宏目录（纯函数，不触碰全局目录，测试可直接使用）。
///
/// 校验：条数上限、每条定义合法（`macros::validate_definition`）、`macro_id` 唯一、
/// 推荐宏之间不存在 `run_macro` 环。失败返回聚合原因（含宏 id）。
pub fn parse_catalog(json: &str) -> Result<RecommendedMacroCatalog, String> {
    let mut catalog: RecommendedMacroCatalog = serde_json::from_str(json)
        .map_err(|error| format!("invalid recommended macro json: {error}"))?;
    catalog.bundle_version = catalog.bundle_version.trim().to_owned();
    validate_macros(&catalog.macros)?;
    Ok(catalog)
}

fn validate_macros(macros: &[MacroDefinition]) -> Result<(), String> {
    if macros.len() > MAX_MACRO_DEFINITIONS {
        return Err(format!(
            "too many recommended macros: {} > {MAX_MACRO_DEFINITIONS}",
            macros.len()
        ));
    }
    let mut issues: Vec<String> = Vec::new();
    let mut seen_ids = std::collections::BTreeSet::new();
    for definition in macros {
        let label = if definition.macro_id.is_empty() {
            "<empty macro id>".to_owned()
        } else {
            definition.macro_id.clone()
        };
        if !seen_ids.insert(definition.macro_id.clone()) {
            issues.push(format!("{label}: duplicate recommended macro id"));
            continue;
        }
        if let Err(definition_issues) = validate_definition(definition) {
            let codes: Vec<String> = definition_issues
                .iter()
                .map(|issue: &MacroIssue| issue.code())
                .collect();
            issues.push(format!("{label}: {}", codes.join(",")));
        }
    }
    if find_cycle(macros).is_some() {
        issues.push("recommended macros contain a run_macro cycle".to_owned());
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues.join("; "))
    }
}

/// 当前推荐宏目录（只读快照，文件顺序）。
pub fn macros() -> Vec<MacroDefinition> {
    store()
        .read()
        .map(|guard| guard.macros.clone())
        .unwrap_or_default()
}

/// 按 `macro_id` 查推荐宏（不存在返回 None）。
pub fn macro_by_id(id: &str) -> Option<MacroDefinition> {
    store().read().ok().and_then(|guard| {
        guard
            .macros
            .iter()
            .find(|item| item.macro_id == id)
            .cloned()
    })
}

/// 推荐宏目录里是否存在该 `macro_id`（推荐方案引用校验用）。
pub fn contains_macro(id: &str) -> bool {
    store()
        .read()
        .map(|guard| guard.macros.iter().any(|item| item.macro_id == id))
        .unwrap_or(false)
}

/// 推荐宏目录版本号（随安装包下发；用于前端提示更新）。
pub fn bundle_version() -> String {
    store()
        .read()
        .map(|guard| guard.bundle_version.clone())
        .unwrap_or_default()
}

/// 用 JSON 替换进程内推荐宏目录（桌面端启动时从安装包资源加载）。
pub fn install_from_json(json: &str) -> Result<(), String> {
    let catalog = parse_catalog(json)?;
    let mut slot = store()
        .write()
        .map_err(|_| "recommended macro catalog lock poisoned".to_owned())?;
    slot.bundle_version = catalog.bundle_version;
    slot.macros = catalog.macros;
    Ok(())
}

/// 安装内嵌兜底副本（资源缺失/非法时使用；首次访问也会懒加载它）。
pub fn install_defaults() -> Result<(), String> {
    install_from_json(DEFAULT_RECOMMENDED_MACROS_JSON)
}

/// 内嵌兜底 JSON（诊断/测试用）。
pub fn default_catalog_json() -> &'static str {
    DEFAULT_RECOMMENDED_MACROS_JSON
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_macro_json(macro_id: &str) -> String {
        format!(
            r#"{{
                "schemaVersion": "0.1",
                "macroId": "{macro_id}",
                "version": "1.0.0",
                "name": "推荐动作",
                "steps": [
                    {{
                        "stepId": "step.1",
                        "action": "open_application",
                        "parameters": {{ "target": "notepad" }}
                    }}
                ]
            }}"#
        )
    }

    fn catalog_json(bundle_version: &str, macros: &[String]) -> String {
        format!(
            r#"{{ "schemaVersion": 1, "bundleVersion": "{bundle_version}", "macros": [{}] }}"#,
            macros.join(",")
        )
    }

    /// 内嵌空目录可用：空列表合法，bundleVersion 正常读出。
    #[test]
    fn embedded_catalog_is_valid() {
        let catalog = parse_catalog(DEFAULT_RECOMMENDED_MACROS_JSON).expect("embedded macros");
        assert!(catalog.macros.is_empty());
        assert_eq!(catalog.bundle_version, "2026.11.1");
    }

    /// 合法推荐宏可加载；`contains_macro`/查询 API 与快照一致。
    #[test]
    fn parses_and_exposes_valid_macro() {
        let catalog = parse_catalog(&catalog_json(
            "2026.11.2",
            &[valid_macro_json("macro.abcdef")],
        ))
        .expect("valid catalog");
        assert_eq!(catalog.macros.len(), 1);
        assert_eq!(catalog.macros[0].macro_id, "macro.abcdef");
    }

    /// 重复 macro_id（即使版本不同）拒绝：推荐目录按“动作”唯一。
    #[test]
    fn rejects_duplicate_macro_ids() {
        let json = catalog_json(
            "2026.11.2",
            &[
                valid_macro_json("macro.abcdef"),
                valid_macro_json("macro.abcdef"),
            ],
        );
        let error = parse_catalog(&json).expect_err("duplicate ids must fail");
        assert!(error.contains("duplicate"), "unexpected error: {error}");
    }

    /// 非法宏定义（缺少步骤/字段错误）拒绝并带出宏 id。
    #[test]
    fn rejects_invalid_macro_definition() {
        let broken = r#"{
            "schemaVersion": "0.1",
            "macroId": "macro.abcdef",
            "version": "1.0.0",
            "name": "",
            "steps": []
        }"#;
        let error = parse_catalog(&catalog_json("2026.11.2", &[broken.to_owned()]))
            .expect_err("invalid macro must fail");
        assert!(error.contains("macro.abcdef"), "unexpected error: {error}");
        assert!(
            error.contains("name_empty") && error.contains("too_few_steps"),
            "unexpected error: {error}"
        );
    }

    /// 推荐宏之间互相引用成环 → 拒绝（避免后续接运行时后无限递归）。
    #[test]
    fn rejects_run_macro_cycles() {
        let first = r#"{
            "schemaVersion": "0.1",
            "macroId": "macro.aaaaaa",
            "version": "1.0.0",
            "name": "A",
            "steps": [{
                "stepId": "step.1",
                "action": "run_macro",
                "parameters": { "nestedMacroId": "macro.bbbbbb", "nestedMacroName": "B" }
            }]
        }"#;
        let second = r#"{
            "schemaVersion": "0.1",
            "macroId": "macro.bbbbbb",
            "version": "1.0.0",
            "name": "B",
            "steps": [{
                "stepId": "step.1",
                "action": "run_macro",
                "parameters": { "nestedMacroId": "macro.aaaaaa", "nestedMacroName": "A" }
            }]
        }"#;
        let error = parse_catalog(&catalog_json(
            "2026.11.2",
            &[first.to_owned(), second.to_owned()],
        ))
        .expect_err("cycle must fail");
        assert!(error.contains("cycle"), "unexpected error: {error}");
    }

    /// 内嵌兜底安装：返回成功，且进程内目录保持只读可用。
    #[test]
    fn install_defaults_keeps_catalog_readable() {
        install_defaults().expect("install embedded recommended macros");
        assert_eq!(bundle_version(), "2026.11.1");
        assert!(macros().is_empty());
        assert!(!contains_macro("macro.abcdef"));
    }
}
