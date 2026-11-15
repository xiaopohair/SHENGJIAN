//! 常用语内置组件入口。
//!
//! 本项目自带常用语实现：安装后返回内置宿主，
//! 面板与文本插入由 `shengjian-windows` 提供。

use std::path::PathBuf;
use std::sync::Arc;

use shengjian_windows::common_phrases::{
    builtin_host, install_builtin_host, CommonPhraseHost, PhrasePanelWindow,
};

pub const COMPONENT_MARKER: &str = "shengjian-common-phrases/builtin";

/// 安装内置常用语宿主（幂等）。
pub fn install(
    _state_dir: PathBuf,
    _window: Option<Arc<dyn PhrasePanelWindow>>,
) -> Option<Arc<dyn CommonPhraseHost>> {
    install_builtin_host();
    Some(builtin_host())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_the_builtin_host() {
        let installed = install(PathBuf::from("."), None);
        assert!(installed.is_some(), "内置常用语必须可用");
        assert_eq!(COMPONENT_MARKER, "shengjian-common-phrases/builtin");
    }
}
