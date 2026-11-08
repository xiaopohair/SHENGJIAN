//! 键位方案数据模型（`button-profiles.json` 的存储形状）。
//!
//! 绑定是**稀疏**的：`(按键, 触发)` 缺条目 = 继承基础映射；显式条目（含
//! [`ButtonAction::Disabled`]）= 覆盖。覆盖层合成见 [`crate::composition`]。

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use shengjian_windows::raw_input::RemoteButton;
use shengjian_windows::send_input::{ButtonAction, ButtonTrigger};

/// 存储结构版本。1 = fork 自研实现的初始版本。
pub const BUTTON_PROFILES_SCHEMA_VERSION: u32 = 1;

/// 导出/导入交换格式的版本（`formatVersion` 包装字段）。
pub const BUTTON_PROFILES_EXPORT_VERSION: u32 = 1;

/// 单个按键的显式绑定表：触发方式 → 动作（含 Disabled = 「不执行」）。
pub type TriggerBindings = BTreeMap<ButtonTrigger, ButtonAction>;

/// 一套键位方案。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonProfile {
    pub id: String,
    pub name: String,
    /// 创建时间（Unix 毫秒）；仅用于展示与排序，不参与语义。
    pub created_at: u64,
    /// 关联的前台应用（可执行文件名，如 `cloudmusic.exe`）。自动应用按
    /// 大小写不敏感的精确匹配；同一 exe 同一时刻只允许被一个方案关联。
    ///
    /// 序列化**总是输出**（只有 `default` 兼容旧文件读取，不用 skip）：
    /// 空数组省略字段会让前端拿到 `undefined`，`.length` 直接崩渲染
    /// （2026-11-01 清除关联白屏根因）。
    #[serde(default)]
    pub app_targets: Vec<String>,
    /// 前台命中 `app_targets` 时自动切换到本方案。
    #[serde(default)]
    pub auto_apply: bool,
    /// 11 个普通键的稀疏绑定；缺条目继承基础映射。序列化总是输出（同上）。
    #[serde(default)]
    pub bindings: BTreeMap<RemoteButton, TriggerBindings>,
}

impl ButtonProfile {
    /// 有显式绑定的按键数（UI 摘要用）。
    pub fn bound_button_count(&self) -> usize {
        self.bindings.len()
    }

    /// 是否关联某个前台可执行文件（大小写不敏感）。
    pub fn matches_executable(&self, executable: &str) -> bool {
        let executable = executable.to_lowercase();
        self.app_targets
            .iter()
            .any(|target| target.to_lowercase() == executable)
    }
}

/// `button-profiles.json` 文档。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonProfilesDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub profiles: Vec<ButtonProfile>,
    /// 手动激活的方案；`None` = 默认方案。序列化总是输出（形状稳定）。
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
    /// 规范化并盖章为当前版本。保存前调用，磁盘上永远只存在规范形：
    /// - id 去重（保留首个）；
    /// - 名称去首尾空白；App 关联去空白、小写、去重；
    /// - 同一 exe 只保留列表中最早方案（一个 App 只被一个方案关联）；
    /// - 激活状态指向不存在的方案时清除。
    pub fn normalized(mut self) -> Self {
        let mut seen_ids = BTreeSet::new();
        self.profiles
            .retain(|profile| seen_ids.insert(profile.id.clone()));
        for profile in &mut self.profiles {
            profile.name = profile.name.trim().to_owned();
            let mut targets: Vec<String> = Vec::new();
            for target in &profile.app_targets {
                let trimmed = target.trim().to_lowercase();
                if !trimmed.is_empty() && !targets.contains(&trimmed) {
                    targets.push(trimmed);
                }
            }
            profile.app_targets = targets;
        }
        let mut claimed_targets = BTreeSet::new();
        for profile in &mut self.profiles {
            profile
                .app_targets
                .retain(|target| claimed_targets.insert(target.clone()));
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

/// 生成方案 ID：毫秒时间戳 + 进程内计数，无第三方依赖。
pub fn new_profile_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("bp-{millis:013x}-{counter:04x}")
}

/// 导出/导入的交换格式（不含激活状态：导入方自行决定是否激活）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonProfilesExport {
    pub format_version: u32,
    pub profiles: Vec<ButtonProfile>,
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
            bindings: BTreeMap::new(),
        }
    }

    #[test]
    fn normalized_dedupes_ids_and_clears_dangling_activation() {
        let mut first = profile("a", " 第一 ");
        first.app_targets = vec!["CloudMusic.EXE", "cloudmusic.exe", " "]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let document = ButtonProfilesDocument {
            schema_version: 99,
            profiles: vec![first, profile("a", "重复"), profile("b", "第二")],
            active_manual_profile_id: Some("missing".to_owned()),
        }
        .normalized();
        assert_eq!(document.schema_version, BUTTON_PROFILES_SCHEMA_VERSION);
        assert_eq!(document.profiles.len(), 2);
        assert_eq!(document.profiles[0].name, "第一");
        assert_eq!(document.profiles[0].app_targets, vec!["cloudmusic.exe"]);
        assert_eq!(document.profiles[1].name, "第二");
        assert_eq!(document.active_manual_profile_id, None);
    }

    #[test]
    fn normalized_keeps_only_the_first_claim_on_a_shared_app_target() {
        let mut first = profile("a", "A");
        first.auto_apply = true;
        first.app_targets = vec!["same.exe".to_owned()];
        let mut second = profile("b", "B");
        second.auto_apply = true;
        second.app_targets = vec!["same.exe".to_owned()];
        let document = ButtonProfilesDocument {
            schema_version: BUTTON_PROFILES_SCHEMA_VERSION,
            profiles: vec![first, second],
            active_manual_profile_id: None,
        }
        .normalized();
        assert_eq!(document.profiles[0].app_targets, vec!["same.exe"]);
        assert!(document.profiles[1].app_targets.is_empty());
    }

    #[test]
    fn serde_contract_uses_camel_case_and_sparse_bindings() {
        let mut bindings = TriggerBindings::new();
        bindings.insert(ButtonTrigger::Long, ButtonAction::Disabled);
        let document = ButtonProfilesDocument {
            schema_version: BUTTON_PROFILES_SCHEMA_VERSION,
            profiles: vec![ButtonProfile {
                id: "p1".to_owned(),
                name: "音乐".to_owned(),
                created_at: 1,
                app_targets: vec!["cloudmusic.exe".to_owned()],
                auto_apply: true,
                bindings: BTreeMap::from([(RemoteButton::VolumeUp, bindings)]),
            }],
            active_manual_profile_id: Some("p1".to_owned()),
        };
        let value = serde_json::to_value(&document).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["activeManualProfileId"], "p1");
        let profile = &value["profiles"][0];
        assert_eq!(profile["createdAt"], 1);
        assert_eq!(profile["appTargets"][0], "cloudmusic.exe");
        assert_eq!(profile["bindings"]["volume_up"]["long"]["type"], "disabled");
        // 缺失的触发不出现（稀疏），单动作 JSON 与 ButtonAction 兼容。
        assert!(profile["bindings"]["volume_up"].get("single").is_none());
        let round_trip: ButtonProfilesDocument =
            serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();
        assert_eq!(round_trip, document);
    }

    #[test]
    fn matches_executable_is_case_insensitive() {
        let mut candidate = profile("a", "A");
        candidate.app_targets = vec!["CloudMusic.exe".to_owned()];
        assert!(candidate.matches_executable("cloudmusic.EXE"));
        assert!(!candidate.matches_executable("spotify.exe"));
    }
}
