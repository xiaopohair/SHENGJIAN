use crate::event_bus::EventPublisher;
use crate::ports::RemoteLinkPort;
use shengjian_contract::{
    ConnectionPhase, ConnectionSnapshot, CoreEvent, CoreResult, EngineError, ErrorDomain,
    RemoteCandidate, RemoteId,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct ConnectionService {
    link: Arc<dyn RemoteLinkPort>,
    publisher: Arc<EventPublisher>,
    state: Mutex<ConnectionSnapshot>,
    generation: AtomicU64,
}

impl ConnectionService {
    pub fn new(link: Arc<dyn RemoteLinkPort>, publisher: Arc<EventPublisher>) -> Self {
        let snapshot = link.snapshot();
        let generation = AtomicU64::new(snapshot.generation.value());
        Self {
            link,
            publisher,
            state: Mutex::new(snapshot),
            generation,
        }
    }

    pub fn snapshot(&self) -> ConnectionSnapshot {
        self.state.lock().expect("connection state").clone()
    }

    pub fn scan(&self) -> CoreResult<Vec<RemoteCandidate>> {
        self.set_phase(ConnectionPhase::Scanning, None);
        match self.link.scan() {
            Ok(candidates) => {
                self.set_phase(
                    if candidates.is_empty() {
                        ConnectionPhase::Idle
                    } else {
                        ConnectionPhase::Disconnected
                    },
                    None,
                );
                Ok(candidates)
            }
            Err(error) => {
                self.fail(error.clone());
                Err(error)
            }
        }
    }

    pub fn connect(&self, remote_id: &RemoteId) -> CoreResult<ConnectionSnapshot> {
        self.set_phase(ConnectionPhase::Connecting, None);
        match self.link.connect(remote_id) {
            Ok(snapshot) => Ok(self.commit(snapshot, Some(ConnectionPhase::Ready))),
            Err(error) => {
                self.fail(error.clone());
                Err(error)
            }
        }
    }

    pub fn disconnect(&self) -> CoreResult<ConnectionSnapshot> {
        self.set_phase(ConnectionPhase::Disconnecting, None);
        match self.link.disconnect() {
            Ok(mut snapshot) => {
                snapshot.remote = None;
                Ok(self.commit(snapshot, Some(ConnectionPhase::Disconnected)))
            }
            Err(error) => {
                self.fail(error.clone());
                Err(error)
            }
        }
    }

    fn commit(
        &self,
        mut snapshot: ConnectionSnapshot,
        phase: Option<ConnectionPhase>,
    ) -> ConnectionSnapshot {
        let generation = self
            .generation
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        snapshot.generation = generation.into();
        if let Some(phase) = phase {
            snapshot.phase = phase;
        }
        snapshot.last_error = None;
        *self.state.lock().expect("connection state") = snapshot.clone();
        self.publisher.publish(
            snapshot.generation,
            CoreEvent::ConnectionChanged {
                snapshot: snapshot.clone(),
            },
        );
        snapshot
    }

    fn fail(&self, error: EngineError) -> ConnectionSnapshot {
        let mut current = self.state.lock().expect("connection state").clone();
        let generation = self
            .generation
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        current.phase = ConnectionPhase::Failed;
        current.last_error = Some(error.clone());
        current.generation = generation.into();
        *self.state.lock().expect("connection state") = current.clone();
        self.publisher.publish(
            current.generation,
            CoreEvent::ConnectionChanged {
                snapshot: current.clone(),
            },
        );
        current
    }

    fn set_phase(&self, phase: ConnectionPhase, error: Option<EngineError>) {
        let mut current = self.state.lock().expect("connection state").clone();
        current.phase = phase;
        current.last_error = error;
        *self.state.lock().expect("connection state") = current.clone();
        self.publisher.publish(
            current.generation,
            CoreEvent::ConnectionChanged { snapshot: current },
        );
    }
}

pub fn unsupported_remote() -> EngineError {
    EngineError::new(ErrorDomain::Bluetooth, "unsupported-remote")
}
