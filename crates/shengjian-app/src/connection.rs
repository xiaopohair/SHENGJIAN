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
                self.set_phase(self.phase_after_scan(&candidates), None);
                Ok(candidates)
            }
            Err(error) => {
                self.fail(error.clone());
                Err(error)
            }
        }
    }

    /// 扫完之后该回到哪个阶段。
    ///
    /// 扫描只做**发现**，不拆链路，所以不能一律按「扫到了设备 = 现在未连接」
    /// 处理。2026-11-12 现场（ARN9 真机）：设备连着的时候点「扫描设备」，旧实现
    /// 无条件写 `Disconnected`，界面头部和设备卡片立刻变成「已断开 / 未连接」，
    /// 但 `device_status` 仍是 `Ready`、电量还在实时推送——链路根本没断，只是
    /// 状态被覆盖了。这里以链路端口的快照为准，把 `Ready` 原样保留。
    fn phase_after_scan(&self, candidates: &[RemoteCandidate]) -> ConnectionPhase {
        if self.link.snapshot().phase == ConnectionPhase::Ready {
            ConnectionPhase::Ready
        } else if candidates.is_empty() {
            ConnectionPhase::Idle
        } else {
            ConnectionPhase::Disconnected
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::EventSink;
    use shengjian_contract::EventEnvelope;

    struct NullSink;

    impl EventSink for NullSink {
        fn emit(&self, _envelope: EventEnvelope) {}
    }

    struct FakeLink {
        state: Mutex<ConnectionSnapshot>,
        candidates: Vec<RemoteCandidate>,
    }

    impl FakeLink {
        fn new(phase: ConnectionPhase, candidates: Vec<RemoteCandidate>) -> Self {
            Self {
                state: Mutex::new(ConnectionSnapshot {
                    phase,
                    ..ConnectionSnapshot::default()
                }),
                candidates,
            }
        }
    }

    impl RemoteLinkPort for FakeLink {
        fn snapshot(&self) -> ConnectionSnapshot {
            self.state.lock().expect("fake link").clone()
        }

        fn scan(&self) -> CoreResult<Vec<RemoteCandidate>> {
            Ok(self.candidates.clone())
        }

        fn connect(&self, _remote_id: &RemoteId) -> CoreResult<ConnectionSnapshot> {
            Err(unsupported_remote())
        }

        fn disconnect(&self) -> CoreResult<ConnectionSnapshot> {
            let mut snapshot = self.snapshot();
            snapshot.phase = ConnectionPhase::Disconnected;
            Ok(snapshot)
        }
    }

    fn service(phase: ConnectionPhase, candidates: Vec<RemoteCandidate>) -> ConnectionService {
        ConnectionService::new(
            Arc::new(FakeLink::new(phase, candidates)),
            Arc::new(EventPublisher::new(Arc::new(NullSink))),
        )
    }

    fn candidate(id: &str) -> RemoteCandidate {
        RemoteCandidate {
            remote_id: RemoteId::new(id),
            name: "remote".to_owned(),
            model: None,
            device_id: None,
            is_supported: true,
            token: None,
        }
    }

    #[test]
    fn scan_keeps_a_live_connection_ready() {
        let service = service(ConnectionPhase::Ready, vec![candidate("a")]);
        service.scan().expect("scan");
        assert_eq!(
            service.snapshot().phase,
            ConnectionPhase::Ready,
            "扫描设备不能把仍在工作的连接显示成已断开"
        );
    }

    #[test]
    fn scan_reports_disconnected_when_not_connected() {
        let service = service(ConnectionPhase::Disconnected, vec![candidate("a")]);
        service.scan().expect("scan");
        assert_eq!(service.snapshot().phase, ConnectionPhase::Disconnected);
    }

    #[test]
    fn scan_reports_idle_when_nothing_is_found() {
        let service = service(ConnectionPhase::Idle, Vec::new());
        service.scan().expect("scan");
        assert_eq!(service.snapshot().phase, ConnectionPhase::Idle);
    }
}
