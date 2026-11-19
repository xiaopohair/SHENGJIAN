use crate::event_bus::EventPublisher;
use crate::ports::ActionSinkPort;
use shengjian_contract::{
    ButtonEdge, ButtonGesture, ButtonMappingSet, CoreEvent, CoreResult, LogicalAction,
    LogicalButton, MouseActionKind,
};
use shengjian_domain::{GestureRecognizer, OK_CONTEXT_CLICK_WINDOW_MS};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct ButtonService {
    publisher: Arc<EventPublisher>,
    actions: Arc<dyn ActionSinkPort>,
    mappings: Mutex<ButtonMappingSet>,
    recognizer: Mutex<GestureRecognizer>,
    last_remote_move_at: Mutex<Option<u64>>,
    generation: AtomicU64,
    /// 边沿是否由**平台注入层**执行动作。
    ///
    /// Chromecast 这类旁路设备的边沿同时会进平台注入器，若这里也执行就是双发
    /// （静音按一下即取消、回车打两个）。小米走 Raw Input 时平台没有映射表，
    /// 必须由这里执行——所以按型号二选一（2026-11-19 真机回归）。
    platform_executes: AtomicBool,
    /// 设备测试/首次配置按键测试：仍观察边沿与手势，但不执行方案动作。
    suspended: AtomicBool,
}

impl ButtonService {
    pub fn new(publisher: Arc<EventPublisher>, actions: Arc<dyn ActionSinkPort>) -> Self {
        let mappings = ButtonMappingSet::default();
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mappings);
        Self {
            publisher,
            actions,
            mappings: Mutex::new(mappings),
            recognizer: Mutex::new(recognizer),
            last_remote_move_at: Mutex::new(None),
            generation: AtomicU64::new(0),
            platform_executes: AtomicBool::new(false),
            suspended: AtomicBool::new(false),
        }
    }

    /// 声明动作由平台注入层执行（Chromecast 旁路）。
    pub fn set_platform_executes(&self, enabled: bool) {
        self.platform_executes.store(enabled, Ordering::Relaxed);
    }

    /// 设备测试/首次配置按键测试：只观察边沿与手势，不执行动作。
    ///
    /// 与平台注入层的 `set_mapping_suspension` 配套使用，覆盖 Raw Input（core
    /// 执行）与 helper 旁路（平台执行）两条路径；只影响内存态，不写设置。
    pub fn set_suspended(&self, suspended: bool) {
        if self.suspended.swap(suspended, Ordering::Relaxed) != suspended {
            // 进入/退出测试时丢掉按住状态，避免测试里按住的键在退出后才触发。
            self.reset();
        }
    }

    pub fn snapshot(&self) -> ButtonMappingSet {
        self.mappings.lock().expect("button mappings").clone()
    }

    /// 把映射同步给平台注入层。
    ///
    /// **只给 helper 旁路的设备用**（目前是 Chromecast）：它的边沿走
    /// `EngineMessage::GateEdge` → 平台注入器，那边有独立的一张表。
    /// Raw Input 设备的边沿走 `ButtonService` 自己执行，再同步过去会**双发**。
    pub fn forward_mappings_to_platform(&self, mappings: &ButtonMappingSet) {
        self.actions.set_button_mappings(mappings);
    }

    pub fn set_mappings(&self, mappings: ButtonMappingSet) {
        *self.mappings.lock().expect("button mappings") = mappings.clone();
        self.recognizer
            .lock()
            .expect("gesture recognizer")
            .configure(&mappings);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn press(&self, button: LogicalButton, now_ms: u64) -> CoreResult<()> {
        self.handle_edge(ButtonEdge {
            button,
            pressed: true,
            at_ms: now_ms,
        })
    }

    pub fn release(&self, button: LogicalButton, now_ms: u64) -> CoreResult<()> {
        self.handle_edge(ButtonEdge {
            button,
            pressed: false,
            at_ms: now_ms,
        })
    }

    /// Feed one platform edge. Used by a live `ButtonSourcePort`; the service
    /// owns gesture recognition and action dispatch.
    pub fn handle_edge(&self, edge: ButtonEdge) -> CoreResult<()> {
        self.publish_edge(edge.clone());
        if edge.pressed && self.try_context_click(edge.button, edge.at_ms) {
            self.recognizer
                .lock()
                .expect("gesture recognizer")
                .swallow_press(edge.button);
            return Ok(());
        }
        let gestures = {
            let mut recognizer = self.recognizer.lock().expect("gesture recognizer");
            if edge.pressed {
                recognizer.press(edge.button, edge.at_ms)
            } else {
                recognizer.release(edge.button, edge.at_ms)
            }
        };
        self.dispatch(gestures)
    }

    pub fn advance(&self, now_ms: u64) -> CoreResult<()> {
        let gestures = self
            .recognizer
            .lock()
            .expect("gesture recognizer")
            .advance(now_ms);
        self.dispatch(gestures)
    }

    /// Drop in-progress press/repeat state without emitting gestures. Used on
    /// disconnect/shutdown so a stale held button cannot fire after recovery.
    pub fn reset(&self) {
        self.recognizer.lock().expect("gesture recognizer").reset();
        *self.last_remote_move_at.lock().expect("last move") = None;
    }

    /// OK context click: after a recent cursor move, pressing OK clicks the
    /// cursor and swallows the normal gesture. Injection failure falls back to
    /// the regular path (the user still gets the configured action).
    fn try_context_click(&self, button: LogicalButton, now_ms: u64) -> bool {
        if button != LogicalButton::Ok || self.suspended.load(Ordering::Relaxed) {
            return false;
        }
        let (enabled, context_click) = {
            let mappings = self.mappings.lock().expect("button mappings");
            let Some(actions) = mappings.actions.get(&LogicalButton::Ok) else {
                return false;
            };
            (mappings.enabled, actions.ok_context_click)
        };
        if !enabled || !context_click {
            return false;
        }
        let recent_move = self
            .last_remote_move_at
            .lock()
            .expect("last move")
            .is_some_and(|at| now_ms.saturating_sub(at) <= OK_CONTEXT_CLICK_WINDOW_MS);
        if !recent_move {
            return false;
        }
        let click = LogicalAction::Mouse {
            kind: MouseActionKind::Click,
            direction: None,
            amount: 0,
        };
        match self.actions.execute(&click) {
            Ok(()) => true,
            Err(error) => {
                self.publisher
                    .publish(self.current_generation(), CoreEvent::Error { error });
                false
            }
        }
    }

    fn publish_edge(&self, edge: ButtonEdge) {
        let generation = self.current_generation();
        self.publisher
            .publish(generation, CoreEvent::ButtonEdge { edge });
    }

    fn dispatch(&self, gestures: Vec<ButtonGesture>) -> CoreResult<()> {
        for gesture in gestures {
            let generation = self.current_generation();
            self.publisher.publish(
                generation,
                CoreEvent::ButtonGesture {
                    gesture: gesture.clone(),
                },
            );
            // 测试模式：手势照常上报（清单/高亮仍可用），动作不执行。
            if self.suspended.load(Ordering::Relaxed) {
                continue;
            }
            let action = self
                .mappings
                .lock()
                .expect("button mappings")
                .action(gesture.button, gesture.trigger);
            if action.is_disabled() {
                continue;
            }
            if self.platform_executes.load(Ordering::Relaxed) {
                // 平台注入层已经执行过这条动作，这里只发事件。
                continue;
            }
            if let Err(error) = self.actions.execute(&action) {
                self.publisher.publish(
                    self.current_generation(),
                    CoreEvent::Error {
                        error: error.clone(),
                    },
                );
                return Err(error);
            }
            if matches!(
                &action,
                LogicalAction::Mouse {
                    kind: MouseActionKind::Scroll | MouseActionKind::Move,
                    ..
                }
            ) {
                *self.last_remote_move_at.lock().expect("last move") = Some(gesture.at_ms);
            }
        }
        Ok(())
    }

    fn current_generation(&self) -> shengjian_contract::Generation {
        self.generation.load(Ordering::SeqCst).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::EventSink;
    use shengjian_contract::{ButtonActions, EventEnvelope, LogicalAction, LogicalButton};
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingActions {
        executed: Mutex<Vec<LogicalAction>>,
    }

    impl ActionSinkPort for RecordingActions {
        fn execute(&self, action: &LogicalAction) -> CoreResult<()> {
            self.executed
                .lock()
                .expect("recorded actions")
                .push(action.clone());
            Ok(())
        }
    }

    struct NoopSink;

    impl EventSink for NoopSink {
        fn emit(&self, _envelope: EventEnvelope) {}
    }

    fn service_with_up_action(actions: Arc<RecordingActions>) -> ButtonService {
        let service =
            ButtonService::new(Arc::new(EventPublisher::new(Arc::new(NoopSink))), actions);
        let mut mappings = ButtonMappingSet::default();
        mappings.set_actions(
            LogicalButton::Up,
            ButtonActions {
                single: LogicalAction::Shortcut {
                    label: "W".to_owned(),
                    keys: vec!["W".to_owned()],
                },
                ..ButtonActions::default()
            },
        );
        service.set_mappings(mappings);
        service
    }

    #[test]
    fn suspended_service_observes_gestures_without_executing_actions() {
        let actions = Arc::new(RecordingActions::default());
        let service = service_with_up_action(Arc::clone(&actions));

        service.set_suspended(true);
        service.press(LogicalButton::Up, 100).unwrap();
        service.release(LogicalButton::Up, 200).unwrap();
        assert!(actions.executed.lock().unwrap().is_empty());

        service.set_suspended(false);
        service.press(LogicalButton::Up, 300).unwrap();
        service.release(LogicalButton::Up, 400).unwrap();
        assert_eq!(actions.executed.lock().unwrap().len(), 1);
    }
}
