use crate::event_bus::EventPublisher;
use crate::ports::AudioSinkPort;
use shengjian_contract::{
    AudioEndpoint, AudioSnapshot, CoreEvent, CoreResult, EngineError, ErrorDomain,
};
use std::sync::{Arc, Mutex};

pub struct AudioService {
    sink: Arc<dyn AudioSinkPort>,
    publisher: Arc<EventPublisher>,
    last_snapshot: Mutex<AudioSnapshot>,
}

impl AudioService {
    pub fn new(sink: Arc<dyn AudioSinkPort>, publisher: Arc<EventPublisher>) -> Self {
        let last_snapshot = sink.snapshot();
        Self {
            sink,
            publisher,
            last_snapshot: Mutex::new(last_snapshot),
        }
    }

    pub fn snapshot(&self) -> AudioSnapshot {
        self.sink.snapshot()
    }

    /// Poll the platform snapshot and publish only on change. The engine ticker
    /// calls this for streaming/draining/device-invalidation transitions that
    /// have no command callback.
    pub fn sync(&self) -> bool {
        let snapshot = self.sink.snapshot();
        let changed = {
            let mut last = self.last_snapshot.lock().expect("audio snapshot");
            if *last == snapshot {
                false
            } else {
                *last = snapshot.clone();
                true
            }
        };
        if changed {
            self.publisher.publish(
                snapshot.generation,
                CoreEvent::AudioChanged {
                    snapshot: snapshot.clone(),
                },
            );
        }
        changed
    }

    pub fn list_endpoints(&self) -> CoreResult<Vec<AudioEndpoint>> {
        self.sink.list_endpoints()
    }

    pub fn select_endpoint(&self, endpoint_id: &str) -> CoreResult<AudioSnapshot> {
        let snapshot = self.sink.select_endpoint(endpoint_id)?;
        self.publish(&snapshot);
        Ok(snapshot)
    }

    pub fn set_gain_db(&self, gain_db: i16) -> CoreResult<AudioSnapshot> {
        if !(0..=24).contains(&gain_db) {
            return Err(EngineError::new(
                ErrorDomain::Configuration,
                "gain-out-of-range",
            ));
        }
        let snapshot = self.sink.set_gain_db(gain_db)?;
        self.publish(&snapshot);
        Ok(snapshot)
    }

    fn publish(&self, snapshot: &AudioSnapshot) {
        *self.last_snapshot.lock().expect("audio snapshot") = snapshot.clone();
        self.publisher.publish(
            snapshot.generation,
            CoreEvent::AudioChanged {
                snapshot: snapshot.clone(),
            },
        );
    }
}
