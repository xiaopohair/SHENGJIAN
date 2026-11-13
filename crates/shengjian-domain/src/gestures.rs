//! Full platform-neutral button gesture policy.
//!
//! Ported semantics: single/double/long press, hold-to-repeat, lock-workstation
//! release deferral and OK context-click swallowing. The caller supplies time
//! in milliseconds; no IO, locks or platform types live here.

use shengjian_contract::{
    ButtonGesture, ButtonMappingSet, ButtonTrigger, LogicalAction, LogicalButton,
};
use std::collections::BTreeMap;

pub const DOUBLE_CLICK_WINDOW_MS: u64 = 300;
pub const LONG_PRESS_MS: u64 = 550;
pub const REPEAT_START_DELAY_MS: u64 = 350;
pub const OK_CONTEXT_CLICK_WINDOW_MS: u64 = 5000;

/// Per-button gesture configuration derived from a mapping snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GestureConfig {
    pub single_configured: bool,
    pub double_enabled: bool,
    pub long_enabled: bool,
    pub repeat_single: Option<u64>,
    pub repeat_long: Option<u64>,
    pub defer_single_until_release: bool,
}

impl GestureConfig {
    pub fn for_button(mappings: &ButtonMappingSet, button: LogicalButton) -> Option<Self> {
        let actions = mappings.actions.get(&button)?;
        if !mappings.enabled || !actions.any_configured() {
            return None;
        }
        let single_configured = !actions.single.is_disabled();
        let double_enabled = !actions.double.is_disabled();
        let long_enabled = !actions.long.is_disabled();
        let defer_single_until_release =
            !double_enabled && !long_enabled && is_lock_workstation(&actions.single);
        let interval = repeat_interval(button);
        let repeat_single = match actions.hold_repeat {
            Some(ButtonTrigger::Single)
                if !long_enabled
                    && !defer_single_until_release
                    && action_allows_repeat(&actions.single) =>
            {
                Some(interval)
            }
            _ => None,
        };
        let repeat_long = match actions.hold_repeat {
            Some(ButtonTrigger::Long) if action_allows_repeat(&actions.long) => Some(interval),
            _ => None,
        };
        Some(Self {
            single_configured,
            double_enabled,
            long_enabled,
            repeat_single,
            repeat_long,
            defer_single_until_release,
        })
    }

    fn raw_path(&self) -> bool {
        self.single_configured
            && !self.double_enabled
            && !self.long_enabled
            && !self.defer_single_until_release
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ButtonState {
    pressed: bool,
    is_second_press: bool,
    waiting_for_second: bool,
    long_fired: bool,
    hold_concluded: bool,
    long_deadline: Option<u64>,
    double_deadline: Option<u64>,
    single_conclude_deadline: Option<u64>,
    repeat_deadline: Option<u64>,
    long_repeat_deadline: Option<u64>,
    swallowing: bool,
}

#[derive(Debug, Default)]
pub struct GestureRecognizer {
    buttons: BTreeMap<LogicalButton, (GestureConfig, ButtonState)>,
}

impl GestureRecognizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configure(&mut self, mappings: &ButtonMappingSet) {
        self.buttons.clear();
        for button in LogicalButton::ALL {
            if let Some(config) = GestureConfig::for_button(mappings, button) {
                self.buttons
                    .insert(button, (config, ButtonState::default()));
            }
        }
    }

    pub fn is_configured(&self, button: LogicalButton) -> bool {
        self.buttons.contains_key(&button)
    }

    pub fn defers_single_until_release(&self, button: LogicalButton) -> bool {
        self.buttons
            .get(&button)
            .is_some_and(|(config, _)| config.defer_single_until_release)
    }

    pub fn press(&mut self, button: LogicalButton, now_ms: u64) -> Vec<ButtonGesture> {
        let Some((config, state)) = self.buttons.get_mut(&button) else {
            return Vec::new();
        };
        if state.swallowing {
            return Vec::new();
        }
        if state.waiting_for_second {
            state.waiting_for_second = false;
            state.is_second_press = true;
            state.double_deadline = None;
        }
        state.pressed = true;
        state.hold_concluded = false;
        if config.long_enabled {
            state.long_deadline = Some(now_ms.saturating_add(LONG_PRESS_MS));
        }
        if config.raw_path() {
            if config.repeat_single.is_some() {
                state.repeat_deadline = Some(now_ms.saturating_add(REPEAT_START_DELAY_MS));
            }
            return vec![gesture(button, ButtonTrigger::Single, now_ms)];
        }
        if config.double_enabled && config.repeat_single.is_some() {
            state.single_conclude_deadline = Some(now_ms.saturating_add(DOUBLE_CLICK_WINDOW_MS));
            state.repeat_deadline = Some(now_ms.saturating_add(REPEAT_START_DELAY_MS));
        }
        Vec::new()
    }

    pub fn release(&mut self, button: LogicalButton, now_ms: u64) -> Vec<ButtonGesture> {
        let Some((config, state)) = self.buttons.get_mut(&button) else {
            return Vec::new();
        };
        if state.swallowing {
            *state = ButtonState::default();
            return Vec::new();
        }
        state.pressed = false;
        state.long_deadline = None;
        state.repeat_deadline = None;
        state.single_conclude_deadline = None;
        state.long_repeat_deadline = None;
        if state.hold_concluded {
            state.hold_concluded = false;
            state.is_second_press = false;
            return Vec::new();
        }
        if state.long_fired {
            state.long_fired = false;
            state.is_second_press = false;
            return Vec::new();
        }
        if state.is_second_press {
            state.is_second_press = false;
            return vec![gesture(button, ButtonTrigger::Double, now_ms)];
        }
        if config.double_enabled {
            state.waiting_for_second = true;
            state.double_deadline = Some(now_ms.saturating_add(DOUBLE_CLICK_WINDOW_MS));
            return Vec::new();
        }
        if config.raw_path() {
            return Vec::new();
        }
        vec![gesture(button, ButtonTrigger::Single, now_ms)]
    }

    pub fn swallow_press(&mut self, button: LogicalButton) {
        if let Some((_, state)) = self.buttons.get_mut(&button) {
            *state = ButtonState::default();
            state.swallowing = true;
        }
    }

    pub fn cancel_press(&mut self, button: LogicalButton) {
        if let Some((_, state)) = self.buttons.get_mut(&button) {
            *state = ButtonState::default();
        }
    }

    pub fn advance(&mut self, now_ms: u64) -> Vec<ButtonGesture> {
        self.advance_with_ticks(now_ms).0
    }

    /// `ticks` contains only repeat pulses, not the one-shot double/long fire.
    pub fn advance_with_ticks(&mut self, now_ms: u64) -> (Vec<ButtonGesture>, Vec<ButtonGesture>) {
        let mut fired = Vec::new();
        let mut ticks = Vec::new();
        for (button, (config, state)) in &mut self.buttons {
            if state
                .double_deadline
                .is_some_and(|deadline| deadline <= now_ms)
            {
                state.double_deadline = None;
                state.waiting_for_second = false;
                fired.push(gesture(*button, ButtonTrigger::Single, now_ms));
            }
            if state
                .single_conclude_deadline
                .is_some_and(|deadline| deadline <= now_ms)
            {
                state.single_conclude_deadline = None;
                if state.pressed && config.repeat_single.is_some() && !state.hold_concluded {
                    state.hold_concluded = true;
                    state.is_second_press = false;
                    state.waiting_for_second = false;
                    fired.push(gesture(*button, ButtonTrigger::Single, now_ms));
                }
            }
            if state
                .long_deadline
                .is_some_and(|deadline| deadline <= now_ms)
            {
                state.long_deadline = None;
                if state.pressed {
                    state.long_fired = true;
                    fired.push(gesture(*button, ButtonTrigger::Long, now_ms));
                    if let Some(interval) = config.repeat_long {
                        state.long_repeat_deadline = Some(now_ms.saturating_add(interval));
                    }
                }
            }
            if let Some(interval) = config.repeat_long {
                if state
                    .long_repeat_deadline
                    .is_some_and(|deadline| deadline <= now_ms)
                {
                    if state.pressed {
                        state.long_repeat_deadline = Some(now_ms.saturating_add(interval));
                        fired.push(gesture(*button, ButtonTrigger::Long, now_ms));
                        ticks.push(gesture(*button, ButtonTrigger::Long, now_ms));
                    } else {
                        state.long_repeat_deadline = None;
                    }
                }
            }
            if let Some(interval) = config.repeat_single {
                if state
                    .repeat_deadline
                    .is_some_and(|deadline| deadline <= now_ms)
                {
                    let ready = state.pressed && (!config.double_enabled || state.hold_concluded);
                    state.repeat_deadline = if ready {
                        Some(now_ms.saturating_add(interval))
                    } else {
                        None
                    };
                    if ready {
                        fired.push(gesture(*button, ButtonTrigger::Single, now_ms));
                        ticks.push(gesture(*button, ButtonTrigger::Single, now_ms));
                    }
                }
            }
        }
        (fired, ticks)
    }

    pub fn next_deadline_ms(&self) -> Option<u64> {
        self.buttons
            .values()
            .flat_map(|(_, state)| {
                [
                    state.long_deadline,
                    state.double_deadline,
                    state.single_conclude_deadline,
                    state.repeat_deadline,
                    state.long_repeat_deadline,
                ]
            })
            .flatten()
            .min()
    }

    pub fn release_all(&mut self) {
        for (_, state) in self.buttons.values_mut() {
            *state = ButtonState::default();
        }
    }

    /// Disconnect/shutdown path: clear all in-progress state without firing.
    pub fn reset(&mut self) {
        self.release_all();
    }

    pub fn is_pressed(&self, button: LogicalButton) -> bool {
        self.buttons
            .get(&button)
            .is_some_and(|(_, state)| state.pressed)
    }
}

fn gesture(button: LogicalButton, trigger: ButtonTrigger, at_ms: u64) -> ButtonGesture {
    ButtonGesture {
        button,
        trigger,
        at_ms,
    }
}

fn repeat_interval(button: LogicalButton) -> u64 {
    if button == LogicalButton::Back {
        50
    } else {
        100
    }
}

fn action_allows_repeat(action: &LogicalAction) -> bool {
    match action {
        LogicalAction::Shortcut { keys, .. } => {
            keys.len() == 1 && !keys[0].contains('+') && !is_modifier_key(&keys[0])
        }
        // 系统动作 = 单键快捷键，连发语义必须与等价的 `Shortcut { keys: [key] }` 一致，
        // 否则绑定「音量+/媒体键」长按不会连发（音量键的常见使用方式）；
        // 空 key 防御性拒绝（执行层会解析失败，不应进入连发）。
        LogicalAction::SystemAction { key } => {
            // 组合类系统动作（返回桌面/切换任务）与多键快捷键一样不连发。
            let key = key.trim();
            !key.is_empty()
                && !key.contains('+')
                && !is_modifier_key(key)
                && !matches!(key, "show_desktop" | "task_switch")
        }
        LogicalAction::Mouse { kind, .. } => matches!(
            kind,
            shengjian_contract::MouseActionKind::Scroll | shengjian_contract::MouseActionKind::Move
        ),
        _ => false,
    }
}

fn is_modifier_key(key: &str) -> bool {
    matches!(
        normalize_key(key).as_str(),
        "ctrl"
            | "control"
            | "leftctrl"
            | "leftcontrol"
            | "rightctrl"
            | "rightcontrol"
            | "shift"
            | "leftshift"
            | "rightshift"
            | "alt"
            | "leftalt"
            | "rightalt"
            | "win"
            | "windows"
            | "meta"
            | "leftwin"
            | "leftwindows"
            | "rightwin"
            | "rightwindows"
    )
}

fn normalize_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace([' ', '_', '-'], "")
}

fn is_lock_workstation(action: &LogicalAction) -> bool {
    let LogicalAction::Shortcut { keys, .. } = action else {
        return false;
    };
    keys.len() == 2
        && keys.iter().any(|key| normalize_key(key) == "l")
        && keys.iter().any(|key| {
            matches!(
                normalize_key(key).as_str(),
                "win"
                    | "windows"
                    | "meta"
                    | "leftwin"
                    | "leftwindows"
                    | "rightwin"
                    | "rightwindows"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_contract::{ButtonActions, LogicalAction};

    fn shortcut(label: &str) -> LogicalAction {
        LogicalAction::Shortcut {
            label: label.to_owned(),
            keys: vec![label.to_owned()],
        }
    }

    fn mapping(
        button: LogicalButton,
        single: LogicalAction,
        double: LogicalAction,
        long: LogicalAction,
        hold_repeat: Option<ButtonTrigger>,
    ) -> ButtonMappingSet {
        let mut mappings = ButtonMappingSet::default();
        mappings.set_actions(
            button,
            ButtonActions {
                single,
                double,
                long,
                hold_repeat,
                ok_context_click: false,
            },
        );
        mappings
    }

    #[test]
    fn raw_single_fires_on_press_and_repeats_while_held() {
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mapping(
            LogicalButton::Up,
            shortcut("Up"),
            LogicalAction::Disabled,
            LogicalAction::Disabled,
            Some(ButtonTrigger::Single),
        ));
        let first = recognizer.press(LogicalButton::Up, 0);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].trigger, ButtonTrigger::Single);
        assert!(recognizer.advance(349).is_empty());
        assert_eq!(recognizer.advance(350)[0].trigger, ButtonTrigger::Single);
        assert!(recognizer.advance(449).is_empty());
        assert_eq!(recognizer.advance(450)[0].trigger, ButtonTrigger::Single);
        assert!(recognizer.release(LogicalButton::Up, 500).is_empty());
    }

    #[test]
    fn double_click_wins_within_window_and_single_after_timeout() {
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mapping(
            LogicalButton::Ok,
            shortcut("Enter"),
            shortcut("Ctrl+Enter"),
            LogicalAction::Disabled,
            None,
        ));
        assert!(recognizer.press(LogicalButton::Ok, 0).is_empty());
        assert!(recognizer.release(LogicalButton::Ok, 20).is_empty());
        assert!(recognizer.advance(100).is_empty());
        assert!(recognizer.press(LogicalButton::Ok, 200).is_empty());
        let released = recognizer.release(LogicalButton::Ok, 250);
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].trigger, ButtonTrigger::Double);

        assert!(recognizer.press(LogicalButton::Ok, 1000).is_empty());
        assert!(recognizer.release(LogicalButton::Ok, 1020).is_empty());
        let single = recognizer.advance(1320);
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].trigger, ButtonTrigger::Single);
    }

    #[test]
    fn long_press_fires_once_and_suppresses_single() {
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mapping(
            LogicalButton::Ok,
            shortcut("Enter"),
            LogicalAction::Disabled,
            shortcut("Focus"),
            None,
        ));
        recognizer.press(LogicalButton::Ok, 0);
        assert!(recognizer.advance(549).is_empty());
        let long = recognizer.advance(550);
        assert_eq!(long.len(), 1);
        assert_eq!(long[0].trigger, ButtonTrigger::Long);
        assert!(recognizer.release(LogicalButton::Ok, 600).is_empty());
    }

    #[test]
    fn lock_workstation_single_waits_for_release() {
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mapping(
            LogicalButton::Power,
            LogicalAction::Shortcut {
                label: "Lock".to_owned(),
                keys: vec!["Win".to_owned(), "L".to_owned()],
            },
            LogicalAction::Disabled,
            LogicalAction::Disabled,
            None,
        ));
        assert!(recognizer.defers_single_until_release(LogicalButton::Power));
        assert!(recognizer.press(LogicalButton::Power, 0).is_empty());
        assert!(recognizer.next_deadline_ms().is_none());
        let released = recognizer.release(LogicalButton::Power, 80);
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].trigger, ButtonTrigger::Single);
    }

    #[test]
    fn non_repeatable_single_never_repeats() {
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mapping(
            LogicalButton::Up,
            LogicalAction::FocusInput,
            LogicalAction::Disabled,
            LogicalAction::Disabled,
            Some(ButtonTrigger::Single),
        ));
        assert_eq!(
            recognizer.press(LogicalButton::Up, 0)[0].trigger,
            ButtonTrigger::Single
        );
        assert!(recognizer.advance(2000).is_empty());
    }

    /// 系统动作（音量/媒体/退出等单键）与等价单键 Shortcut 的连发语义一致；
    /// 修饰键与多键写法仍不连发。
    #[test]
    fn system_action_repeats_like_single_key_shortcut() {
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mapping(
            LogicalButton::Up,
            LogicalAction::SystemAction {
                key: "volume_up".to_owned(),
            },
            LogicalAction::Disabled,
            LogicalAction::Disabled,
            Some(ButtonTrigger::Single),
        ));
        assert_eq!(
            recognizer.press(LogicalButton::Up, 0)[0].trigger,
            ButtonTrigger::Single
        );
        assert_eq!(
            recognizer.advance(350).len(),
            1,
            "系统动作单键应允许长按连发"
        );
        assert!(action_allows_repeat(&LogicalAction::SystemAction {
            key: "media_play_pause".to_owned()
        }));
        assert!(!action_allows_repeat(&LogicalAction::SystemAction {
            key: "left_control".to_owned()
        }));
        assert!(!action_allows_repeat(&LogicalAction::SystemAction {
            key: "ctrl+up".to_owned()
        }));
        assert!(!action_allows_repeat(&LogicalAction::SystemAction {
            key: "   ".to_owned()
        }));
        assert!(!action_allows_repeat(&LogicalAction::SystemAction {
            key: "show_desktop".to_owned()
        }));
        assert!(!action_allows_repeat(&LogicalAction::SystemAction {
            key: "task_switch".to_owned()
        }));
    }

    #[test]
    fn long_repeat_ticks_after_initial_long() {
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mapping(
            LogicalButton::Down,
            LogicalAction::Disabled,
            LogicalAction::Disabled,
            shortcut("Down"),
            Some(ButtonTrigger::Long),
        ));
        recognizer.press(LogicalButton::Down, 0);
        assert_eq!(recognizer.advance(550).len(), 1);
        assert_eq!(recognizer.advance(650).len(), 1);
        assert_eq!(recognizer.advance(749).len(), 0);
        assert_eq!(recognizer.advance(750).len(), 1);
    }

    #[test]
    fn swallowing_suppresses_all_gestures_until_release() {
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mapping(
            LogicalButton::Ok,
            shortcut("Enter"),
            shortcut("Ctrl+Enter"),
            shortcut("Focus"),
            None,
        ));
        recognizer.swallow_press(LogicalButton::Ok);
        assert!(recognizer.press(LogicalButton::Ok, 0).is_empty());
        assert!(recognizer.advance(1000).is_empty());
        assert!(recognizer.release(LogicalButton::Ok, 1100).is_empty());
        assert!(recognizer.advance(2000).is_empty());
    }
}
