use crate::event_bus::EventPublisher;
use crate::ports::ActionSinkPort;
use shengjian_contract::{
    ButtonEdge, ButtonGesture, ButtonMappingSet, CoreEvent, CoreResult, LogicalAction,
    LogicalButton, MouseActionKind,
};
use shengjian_domain::{GestureRecognizer, OK_CONTEXT_CLICK_WINDOW_MS};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct ButtonService {
    publisher: Arc<EventPublisher>,
    actions: Arc<dyn ActionSinkPort>,
    mappings: Mutex<ButtonMappingSet>,
    recognizer: Mutex<GestureRecognizer>,
    last_remote_move_at: Mutex<Option<u64>>,
    generation: AtomicU64,
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
        }
    }

    pub fn snapshot(&self) -> ButtonMappingSet {
        self.mappings.lock().expect("button mappings").clone()
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
        if button != LogicalButton::Ok {
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
            let action = self
                .mappings
                .lock()
                .expect("button mappings")
                .action(gesture.button, gesture.trigger);
            if action.is_disabled() {
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
