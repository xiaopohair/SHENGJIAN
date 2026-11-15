//! 常用语：内置的快捷文本面板。
//!
//! 用户在设置里维护一个列表；按下绑定键后，用 ↑/↓ 选择条目、OK 插入、返回关闭。
//! 面板打开期间这些按键由本模块在**手势识别之前**认领，命中即插入文本并结束，
//! 不进入单击/双击/长按判定，也不写成持久绑定。
//!
//! 常用语完全内置，不依赖外部组件；数据保存在
//! `%LOCALAPPDATA%\Shengjian\common-phrases.json`。

use crate::raw_input::RemoteButton;
use std::sync::{Arc, Mutex};

/// 面板按键认领的来源标识（Windows 只有一条遥控器来源；保留参数以便未来多来源）。
pub const PHRASE_SOURCE_DEFAULT: &str = "remote";

/// 常用语是否可用。内置宿主恒为 [`CommonPhraseAccess::Allowed`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonPhraseAccess {
    /// 未安装宿主（空实现）。
    Unavailable,
    /// 可用。
    Allowed,
}

/// 面板浮层窗口（由宿主实现）：创建**不抢焦点**的浮层并加载可选组件的 UI 资源。
///
/// 约定：`show` 不得抢走前台应用的焦点；`hide` 必须幂等且不得改变用户按键映射。
pub trait PhrasePanelWindow: Send + Sync {
    fn show(&self);
    fn hide(&self);
}

/// 面板上某个位置的数据（只含短显示，不含完整句）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonPhrasePanelEntry {
    pub button: RemoteButton,
    /// 该位置的短显示；未分配时为空字符串。
    pub label: String,
}

/// 可选常用语组件适配接口。所有方法都有默认实现（不可用 + 不认领），
/// 因此未安装宿主时引擎行为与改动前完全一致。
pub trait CommonPhraseHost: Send + Sync {
    /// 可用状态（界面据此显示入口）。
    fn access(&self) -> CommonPhraseAccess {
        CommonPhraseAccess::Unavailable
    }

    /// 兼容入口：更新可用状态（内置宿主忽略）。
    fn set_access(&self, _access: CommonPhraseAccess) {}

    /// 是否可用（`Allowed` 的便捷判据）。
    fn can_use(&self) -> bool {
        matches!(self.access(), CommonPhraseAccess::Allowed)
    }

    /// 打开面板（返回是否真的打开）。未安装宿主时返回 `false`。
    fn open(&self, source: &str) -> bool {
        let _ = source;
        false
    }

    /// 该按键此刻是否归面板所有（只读预判；供门控遮蔽与界面状态使用）。
    fn claims(&self, button: RemoteButton, source: &str) -> bool {
        let _ = (button, source);
        false
    }

    /// 处理一次按键边沿；返回 `true` 表示已被面板消费（不得进入手势识别，也不执行原映射）。
    ///
    /// 配对约定（公开侧自持，2026-10-15 评审 M2）：引擎自己记录**被认领的按下**；
    /// 该键随后的释放沿由引擎直接消费，**不依赖本方法在释放沿的返回值**——即使
    /// 那一刻面板已关闭、本方法返回 `false`，释放沿也不会漏进手势识别（否则会对
    /// 一个从未开始的按压做释放判定，触发幽灵单击/双击）。实现方仍会收到这段
    /// 释放沿（`is_pressed=false`），可据此清理内部在按状态。
    /// 反之，若释放沿被本方法认领、而引擎没有该键的认领记录（按下沿未被认领），
    /// 引擎会先清掉该键的手势状态再消费，避免按压残留被错判成后续手势。
    fn handle(&self, button: RemoteButton, is_pressed: bool, source: &str) -> bool {
        let _ = (button, is_pressed, source);
        false
    }

    /// 面板当前是否打开。
    fn is_panel_open(&self) -> bool {
        false
    }

    /// 来源失效（遥控器断连、输入重置）：关闭面板并清掉认领状态。
    fn reset(&self, reason: &str) {
        let _ = reason;
    }

    /// 关闭面板（返回键之外的路径：应用退出、前台变化等）。
    fn close(&self, reason: &str) {
        let _ = reason;
    }

    /// 应用退出时的收尾（进程结束前的最后一次调用；幂等）。
    ///
    /// 实现方必须**等待**在途的插入事务完成（含剪贴板恢复窗口）再让内部
    /// 工作线程退出。为什么必须显式：退出路径是 `std::process::exit`，不执行
    /// 析构——只靠 `Drop` 会让「已写脏剪贴板」的事务在恢复前被掐断，把用户
    /// 剪贴板永久留在常用语文本上。未接入组件时空操作。
    fn shutdown(&self) {}

    /// 面板当前列表（兼容接口；只含短显示）。
    fn panel_entries(&self) -> Vec<CommonPhrasePanelEntry> {
        Vec::new()
    }

    /// 取走面板当前要显示的一次性提示（失败反馈等）。
    ///
    /// 返回失败原因的稳定 key（如 `input_unavailable`），无提示时返回 `None`。
    /// **读取即消费**：同一条提示只下发一次（面板每次显示都会重读快照）。
    /// 用户文案由界面层按 key 分叉，不在这里拼。
    fn take_panel_notice(&self) -> Option<String> {
        None
    }

    /// 鼠标点选面板某个位置（与遥控器按下等价）；返回是否已受理。
    fn select(&self, button: RemoteButton) -> bool {
        let _ = button;
        false
    }

    /// 面板是否处于「常用语示例」（只读预览）模式。
    fn panel_preview_mode(&self) -> bool {
        false
    }

    /// 下行命令通道：编辑器（"调整常用语"）等扩展能力走这里，公开侧用一条
    /// Tauri 命令转发。`command` 是稳定命令 ID，`payload` 与返回值都是 JSON。
    /// 未安装宿主或未知命令一律返回 `None`——调用方按"不支持"处理，
    /// 不得静默当成功（外部调用成功判据）。
    fn dispatch(&self, command: &str, payload: &str) -> Option<String> {
        let _ = (command, payload);
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonPhraseSlot {
    /// 面板位置：ok / up / down / left / right。
    pub button: String,
    /// 面板上显示的短标题。
    pub label: String,
    /// 真正插入的文本。
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonPhrasesDocument {
    pub slots: Vec<CommonPhraseSlot>,
}

impl Default for CommonPhrasesDocument {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

fn normalize_document(mut document: CommonPhrasesDocument) -> CommonPhrasesDocument {
    let mut seen = std::collections::HashSet::new();
    for (index, slot) in document.slots.iter_mut().enumerate() {
        slot.label = slot.label.trim().to_owned();
        if slot.button.trim().is_empty() || !seen.insert(slot.button.clone()) {
            slot.button = format!("phrase-{index}");
            seen.insert(slot.button.clone());
        }
    }
    document
}

fn document_path() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Shengjian")
        .join("common-phrases.json")
}

pub fn load_document() -> CommonPhrasesDocument {
    let Ok(raw) = std::fs::read_to_string(document_path()) else {
        return CommonPhrasesDocument::default();
    };
    serde_json::from_str::<CommonPhrasesDocument>(&raw)
        .map(normalize_document)
        .unwrap_or_default()
}

pub fn save_document(document: &CommonPhrasesDocument) -> Result<(), String> {
    let path = document_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let normalized = normalize_document(document.clone());
    let json = serde_json::to_string_pretty(&normalized).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, json).map_err(|error| error.to_string())?;
    // Windows 上 rename 不覆盖已存在目标。
    let _ = std::fs::remove_file(&path);
    std::fs::rename(&temporary, &path).map_err(|error| error.to_string())
}

fn button_from_id(id: &str) -> Option<RemoteButton> {
    match id {
        "ok" => Some(RemoteButton::Ok),
        "up" => Some(RemoteButton::Up),
        "down" => Some(RemoteButton::Down),
        "left" => Some(RemoteButton::Left),
        "right" => Some(RemoteButton::Right),
        _ => None,
    }
}

fn is_panel_button(button: RemoteButton) -> bool {
    matches!(
        button,
        RemoteButton::Ok
            | RemoteButton::Up
            | RemoteButton::Down
            | RemoteButton::Left
            | RemoteButton::Right
            | RemoteButton::Back
            | RemoteButton::Menu
    )
}

/// 内置常用语宿主。
pub struct BuiltinCommonPhraseHost {
    slots: Arc<std::sync::RwLock<Vec<CommonPhraseSlot>>>,
    panel_open: std::sync::atomic::AtomicBool,
    notice: Mutex<Option<String>>,
}

impl BuiltinCommonPhraseHost {
    pub fn new() -> Self {
        Self {
            slots: Arc::new(std::sync::RwLock::new(load_document().slots)),
            panel_open: std::sync::atomic::AtomicBool::new(false),
            notice: Mutex::new(None),
        }
    }

    fn close_panel(&self, _reason: &str) {
        self.panel_open
            .store(false, std::sync::atomic::Ordering::SeqCst);
        crate::key_gate::set_phrase_panel_mask(0);
        #[cfg(windows)]
        crate::common_phrases_panel::hide_panel();
    }

    fn insert_selected(&self) -> Result<bool, String> {
        #[cfg(windows)]
        let text = crate::common_phrases_panel::selected_text().unwrap_or_default();
        #[cfg(not(windows))]
        let text = String::new();
        if text.trim().is_empty() {
            if let Ok(mut notice) = self.notice.lock() {
                *notice = Some("empty_slot".to_owned());
            }
            return Ok(false);
        }
        #[cfg(windows)]
        {
            crate::common_phrases_panel::insert_text(&text)?;
            Ok(true)
        }
        #[cfg(not(windows))]
        {
            Err("unsupported".to_owned())
        }
    }
}

impl Default for BuiltinCommonPhraseHost {
    fn default() -> Self {
        Self::new()
    }
}

impl CommonPhraseHost for BuiltinCommonPhraseHost {
    fn access(&self) -> CommonPhraseAccess {
        CommonPhraseAccess::Allowed
    }

    fn open(&self, _source: &str) -> bool {
        let document = load_document();
        if let Ok(mut slots) = self.slots.write() {
            *slots = document.slots.clone();
        }
        if let Ok(mut notice) = self.notice.lock() {
            *notice = None;
        }
        #[cfg(windows)]
        {
            if !crate::common_phrases_panel::show_panel(document.slots.clone()) {
                if let Ok(mut notice) = self.notice.lock() {
                    *notice = Some("panel_unavailable".to_owned());
                }
                return false;
            }
        }
        #[cfg(not(windows))]
        {
            return false;
        }
        self.panel_open
            .store(true, std::sync::atomic::Ordering::SeqCst);
        crate::key_gate::set_phrase_panel_mask(crate::key_gate::phrase_panel_mask());
        true
    }

    fn claims(&self, button: RemoteButton, _source: &str) -> bool {
        self.panel_open.load(std::sync::atomic::Ordering::SeqCst) && is_panel_button(button)
    }

    fn handle(&self, button: RemoteButton, is_pressed: bool, _source: &str) -> bool {
        if !self.panel_open.load(std::sync::atomic::Ordering::SeqCst) || !is_panel_button(button) {
            return false;
        }
        if !is_pressed {
            return true;
        }
        match button {
            RemoteButton::Back | RemoteButton::Menu => {
                self.close_panel("back");
                true
            }
            RemoteButton::Up => {
                #[cfg(windows)]
                crate::common_phrases_panel::move_selection(-1);
                true
            }
            RemoteButton::Down => {
                #[cfg(windows)]
                crate::common_phrases_panel::move_selection(1);
                true
            }
            RemoteButton::Ok => match self.insert_selected() {
                Ok(true) => {
                    self.close_panel("inserted");
                    true
                }
                Ok(false) => true,
                Err(error) => {
                    if let Ok(mut notice) = self.notice.lock() {
                        *notice = Some(error);
                    }
                    true
                }
            },
            // 左右键在面板里不做事，但仍由面板认领，避免漏到底层应用。
            _ => true,
        }
    }

    fn is_panel_open(&self) -> bool {
        self.panel_open.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn reset(&self, reason: &str) {
        self.close_panel(reason);
    }

    fn close(&self, reason: &str) {
        self.close_panel(reason);
    }

    fn shutdown(&self) {
        self.close_panel("shutdown");
    }

    fn panel_entries(&self) -> Vec<CommonPhrasePanelEntry> {
        let Ok(slots) = self.slots.read() else {
            return Vec::new();
        };
        slots
            .iter()
            .filter_map(|slot| {
                let button = button_from_id(&slot.button)?;
                Some(CommonPhrasePanelEntry {
                    button,
                    label: slot.label.clone(),
                })
            })
            .collect()
    }

    fn take_panel_notice(&self) -> Option<String> {
        self.notice.lock().ok().and_then(|mut guard| guard.take())
    }
}

static BUILTIN_HOST: std::sync::OnceLock<Arc<BuiltinCommonPhraseHost>> = std::sync::OnceLock::new();

/// 面板窗口被销毁（点击浮层 / 系统关窗）时由 `common_phrases_panel` 回调。
///
/// 点击浮层走的是 `WM_LBUTTONUP → WM_CLOSE → DestroyWindow`，不经过宿主的
/// `close_panel`。不在这条回路上复位的话，`panel_open` 会一直是 true，`key_gate`
/// 的「面板遮蔽掩码」也一直挂着——低层键盘钩子于是继续吞方向键，表现就是
/// 「面板消失后实体键盘方向键失效，直到应用退出」。
pub(crate) fn notify_panel_destroyed() {
    if let Some(host) = BUILTIN_HOST.get() {
        host.close_panel("window_destroyed");
    }
}

pub fn builtin_host() -> Arc<BuiltinCommonPhraseHost> {
    Arc::clone(BUILTIN_HOST.get_or_init(|| Arc::new(BuiltinCommonPhraseHost::new())))
}

/// 安装内置常用语宿主（幂等）。
pub fn install_builtin_host() {
    set_common_phrase_host(builtin_host());
}

/// 未安装宿主时的空实现：不可用、不认领、不影响任何按键行为。
#[derive(Debug, Default)]
pub struct DisabledCommonPhraseHost;

impl CommonPhraseHost for DisabledCommonPhraseHost {}

/// 进程级安装点：宿主启动时把常用语实现安装在这里。
///
/// 为什么用全局槽而不是往引擎里层层传递：引擎的注入器接口（`MappingInjector`）
/// 已经是唯一的按键分发入口，`common_phrase_host()` 的默认实现直接读这里即可，
/// 无需改动 `Platform` / `ButtonMappingRuntime` 的构造与全部测试调用点。
/// 本应用是单实例桌面应用（一个引擎、一个遥控器来源），因此单一槽位成立。
static COMMON_PHRASE_HOST: std::sync::OnceLock<std::sync::RwLock<Arc<dyn CommonPhraseHost>>> =
    std::sync::OnceLock::new();

fn common_phrase_slot() -> &'static std::sync::RwLock<Arc<dyn CommonPhraseHost>> {
    COMMON_PHRASE_HOST.get_or_init(|| std::sync::RwLock::new(Arc::new(DisabledCommonPhraseHost)))
}

/// 安装/替换常用语组件（宿主启动时调用一次；重复调用以后写入者为准）。
pub fn set_common_phrase_host(host: Arc<dyn CommonPhraseHost>) {
    let slot = common_phrase_slot();
    match slot.write() {
        Ok(mut guard) => *guard = host,
        Err(poisoned) => *poisoned.into_inner() = host,
    }
}

/// 当前已安装的常用语组件（未安装时是空实现）。
pub fn current_common_phrase_host() -> Arc<dyn CommonPhraseHost> {
    match common_phrase_slot().read() {
        Ok(guard) => Arc::clone(&guard),
        Err(poisoned) => Arc::clone(&poisoned.into_inner()),
    }
}

/// 兼容入口：更新当前宿主的可用状态（内置宿主忽略外部门禁）。
pub fn set_common_phrase_access(access: CommonPhraseAccess) {
    current_common_phrase_host().set_access(access);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_host_is_unavailable_and_never_claims_a_button() {
        let host = DisabledCommonPhraseHost;
        assert_eq!(host.access(), CommonPhraseAccess::Unavailable);
        assert!(!host.can_use());
        assert!(!host.open(PHRASE_SOURCE_DEFAULT));
        assert!(!host.is_panel_open());
        for button in [
            RemoteButton::Ok,
            RemoteButton::Left,
            RemoteButton::Up,
            RemoteButton::Right,
            RemoteButton::Down,
            RemoteButton::Back,
            RemoteButton::VolumeUp,
        ] {
            assert!(!host.claims(button, PHRASE_SOURCE_DEFAULT));
            assert!(!host.handle(button, true, PHRASE_SOURCE_DEFAULT));
            assert!(!host.handle(button, false, PHRASE_SOURCE_DEFAULT));
        }
        // 收尾路径必须可调用且无副作用。
        host.reset("test");
        host.close("test");
        assert!(host.panel_entries().is_empty());
        assert!(!host.select(RemoteButton::Ok));
    }

    #[test]
    fn default_document_has_no_fixed_slots() {
        let document = CommonPhrasesDocument::default();
        assert!(document.slots.is_empty());
    }

    #[test]
    fn normalize_keeps_arbitrary_rows_in_order() {
        let document = normalize_document(CommonPhrasesDocument {
            slots: vec![
                CommonPhraseSlot {
                    button: "custom-a".to_owned(),
                    label: " 地址 ".to_owned(),
                    text: "A".to_owned(),
                },
                CommonPhraseSlot {
                    button: "custom-b".to_owned(),
                    label: "邮箱".to_owned(),
                    text: "B".to_owned(),
                },
            ],
        });
        assert_eq!(document.slots.len(), 2);
        assert_eq!(document.slots[0].label, "地址");
        assert_eq!(document.slots[0].button, "custom-a");
        assert_eq!(document.slots[1].button, "custom-b");
    }

    #[test]
    fn builtin_host_is_available() {
        let host = BuiltinCommonPhraseHost::new();
        assert_eq!(host.access(), CommonPhraseAccess::Allowed);
        assert!(!host.is_panel_open());
    }

    #[test]
    fn panel_button_mask_covers_panel_keys_and_menu_fallback() {
        let mask = crate::key_gate::phrase_panel_mask();
        let expected = [
            RemoteButton::Ok,
            RemoteButton::Left,
            RemoteButton::Up,
            RemoteButton::Right,
            RemoteButton::Down,
            RemoteButton::Back,
            RemoteButton::Menu,
        ];
        for button in expected {
            assert_eq!(
                (mask >> button.ordinal()) & 1,
                1,
                "{button:?} 必须在遮蔽掩码里"
            );
        }
        for button in [RemoteButton::Home, RemoteButton::Tv, RemoteButton::VolumeUp] {
            assert_eq!((mask >> button.ordinal()) & 1, 0, "{button:?} 不得被遮蔽");
        }
    }
}
