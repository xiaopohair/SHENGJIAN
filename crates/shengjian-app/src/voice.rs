use crate::event_bus::EventPublisher;
use shengjian_contract::{
    CoreEvent, CoreResult, EngineError, ErrorDomain, VoiceSnapshot, VoiceState,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct VoiceService {
    publisher: Arc<EventPublisher>,
    state: Mutex<VoiceSnapshot>,
    generation: AtomicU64,
}

impl VoiceService {
    pub fn new(publisher: Arc<EventPublisher>) -> Self {
        Self {
            publisher,
            state: Mutex::new(VoiceSnapshot::default()),
            generation: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> VoiceSnapshot {
        self.state.lock().expect("voice state").clone()
    }

    pub fn set_input_tool(&self, tool: Option<String>) -> VoiceSnapshot {
        let mut current = self.state.lock().expect("voice state").clone();
        current.input_tool = tool;
        self.commit(current)
    }

    pub fn set_hold_hotkey(&self, keys: Vec<String>) -> VoiceSnapshot {
        let mut current = self.state.lock().expect("voice state").clone();
        current.hold_hotkey = keys;
        self.commit(current)
    }

    pub fn press(&self) -> CoreResult<VoiceSnapshot> {
        let current = self.state.lock().expect("voice state").clone();
        if current.state != VoiceState::Idle {
            return Err(EngineError::new(ErrorDomain::Input, "voice-not-idle"));
        }
        let mut next = current;
        next.state = VoiceState::Streaming;
        next.last_error = None;
        Ok(self.commit(next))
    }

    pub fn release(&self) -> CoreResult<VoiceSnapshot> {
        let current = self.state.lock().expect("voice state").clone();
        if current.state != VoiceState::Streaming {
            return Err(EngineError::new(ErrorDomain::Input, "voice-not-streaming"));
        }
        let mut next = current;
        next.state = VoiceState::Draining;
        next.last_error = None;
        Ok(self.commit(next))
    }

    pub fn finish_drain(&self) -> CoreResult<VoiceSnapshot> {
        let current = self.state.lock().expect("voice state").clone();
        if current.state != VoiceState::Draining {
            return Err(EngineError::new(ErrorDomain::Input, "voice-not-draining"));
        }
        let mut next = current;
        next.state = VoiceState::Idle;
        Ok(self.commit(next))
    }

    /// Disconnect/exit path: releases any active session pairing.
    pub fn force_idle(&self) -> VoiceSnapshot {
        let current = self.state.lock().expect("voice state").clone();
        if current.state == VoiceState::Idle {
            return current;
        }
        let mut next = current;
        next.state = VoiceState::Idle;
        self.commit(next)
    }

    /// Reconcile a platform-observed state into the core lifecycle.
    ///
    /// This is the bridge for adapters that can only poll the platform
    /// snapshot; duplicate states are no-ops.
    pub fn sync_state(&self, target: VoiceState) -> CoreResult<()> {
        let current = self.state.lock().expect("voice state").state;
        if current == target {
            return Ok(());
        }
        match target {
            VoiceState::Idle => {
                self.force_idle();
                Ok(())
            }
            VoiceState::Streaming => {
                if current != VoiceState::Idle {
                    self.force_idle();
                }
                self.press().map(|_| ())
            }
            VoiceState::Draining => {
                if current == VoiceState::Streaming {
                    self.release().map(|_| ())
                } else {
                    if current != VoiceState::Idle {
                        self.force_idle();
                    }
                    Ok(())
                }
            }
            VoiceState::Failed => {
                self.force_idle();
                Ok(())
            }
        }
    }

    fn commit(&self, mut snapshot: VoiceSnapshot) -> VoiceSnapshot {
        let generation = self
            .generation
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        snapshot.generation = generation.into();
        *self.state.lock().expect("voice state") = snapshot.clone();
        self.publisher.publish(
            snapshot.generation,
            CoreEvent::VoiceChanged {
                snapshot: snapshot.clone(),
            },
        );
        snapshot
    }
}
