//! Windows 侧「输入工具行为」枚举（2026-11-13 配置化）。
//!
//! 工具列表、显示名与默认热键来自可编辑配置 `config/voice-tools.json`
//! （`shengjian_domain::voice_tools`）；本枚举只保留**必须写在代码里的 Windows
//! 行为**，由配置里的 `kind` 字段选择：
//!
//! * `wechat` → 会话级切进微信输入法（`ime` 的 WeType 配方）、微信开麦观测与
//!   休眠重放（`spawn_wetype_check`）；
//! * `doubao` → 会话级切进豆包输入法、豆包开麦观测、报告层右 Alt 松手保险；
//! * `vokie` → 桌面程序：不切输入法，只做 `Vokie.exe` 开麦观测；
//! * `custom` → 其他工具：只有用户自定义热键，**不做任何输入法切换/观测/救回**
//!   （不借用微信或豆包的行为）。
//!
//! 新增工具只要在 JSON 里把 `kind` 指到已支持的四者之一即可复用行为；未知
//! `kind` 一律按 [`VoiceToolKind::Custom`] 处理——配置里的新值不会因为代码没
//! 跟上就悄悄借用微信/豆包的输入法逻辑。

/// 语音输入工具在 Windows 上需要执行的**行为**类别。
///
/// 与配置里的 `kind` 一一对应；它不携带 id 或显示名（那些只在配置目录里）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceToolKind {
    /// 微信输入法（WeType）。
    Wechat,
    /// 豆包输入法。
    Doubao,
    /// Vokie 桌面版（不是输入法）。
    Vokie,
    /// 其他工具：只有自定义热键，不做输入法修复。
    #[default]
    Custom,
}

impl VoiceToolKind {
    /// 从配置的 `kind` 字段解析行为类别。
    ///
    /// 认不出的值（配置更新而代码未更新、或用户手写错）按 `custom` 处理：
    /// 宁可少做（只注入热键），也不能误切用户的输入法——2026-09-29 真机曾把
    /// 豆包路径误切成微信。
    pub fn from_kind(kind: &str) -> Self {
        match kind.trim() {
            "wechat" => Self::Wechat,
            "doubao" => Self::Doubao,
            "vokie" => Self::Vokie,
            _ => Self::Custom,
        }
    }

    /// 稳定 token（日志与配置同词表）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wechat => "wechat",
            Self::Doubao => "doubao",
            Self::Vokie => "vokie",
            Self::Custom => "custom",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VoiceToolKind;

    #[test]
    fn catalog_kinds_round_trip_and_unknown_falls_back_to_custom() {
        for kind in [
            VoiceToolKind::Wechat,
            VoiceToolKind::Doubao,
            VoiceToolKind::Vokie,
            VoiceToolKind::Custom,
        ] {
            assert_eq!(VoiceToolKind::from_kind(kind.as_str()), kind);
        }

        // 未知 / 空白 / 历史拼写：一律 custom，不借用微信/豆包行为。
        for unknown in ["", "  ", "other", "wechat2", "WeChat"] {
            assert_eq!(
                VoiceToolKind::from_kind(unknown),
                VoiceToolKind::Custom,
                "{unknown:?}"
            );
        }

        // 配置值允许首尾空白（JSON 手写场景）。
        assert_eq!(VoiceToolKind::from_kind(" doubao "), VoiceToolKind::Doubao);
        assert_eq!(VoiceToolKind::default(), VoiceToolKind::Custom);
    }
}
