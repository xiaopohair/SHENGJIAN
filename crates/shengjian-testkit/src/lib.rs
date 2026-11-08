//! Test doubles for the Shengjian core. No platform I/O.

use shengjian_app::{
    ActionSinkPort, AudioSinkPort, ButtonMappingStorePort, ButtonProfilesStorePort, EventSink,
    ForegroundPort, RemoteLinkPort, SettingsDocument, SettingsStorePort, VoiceSourcePort,
};
use shengjian_contract::{
    AudioEndpoint, AudioSnapshot, AudioState, ButtonMappingSet, ButtonProfilesDocument,
    ConnectionPhase, ConnectionSnapshot, CoreResult, EngineError, ErrorDomain, EventEnvelope,
    LogicalAction, RemoteCandidate, RemoteId, VoiceState,
};
use shengjian_domain::AppSettings;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct RecordingSink {
    events: Mutex<Vec<EventEnvelope>>,
}

impl RecordingSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<EventEnvelope> {
        self.events.lock().expect("events").clone()
    }

    pub fn len(&self) -> usize {
        self.events.lock().expect("events").len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.lock().expect("events").is_empty()
    }
}

#[derive(Default)]
pub struct FakeForeground {
    executable: Mutex<Option<String>>,
}

impl FakeForeground {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_executable(&self, executable: Option<&str>) {
        *self.executable.lock().expect("executable") = executable.map(str::to_owned);
    }
}

impl ForegroundPort for FakeForeground {
    fn executable(&self) -> Option<String> {
        self.executable.lock().expect("executable").clone()
    }
}

pub struct FakeVoiceSource {
    state: Mutex<VoiceState>,
}

impl Default for FakeVoiceSource {
    fn default() -> Self {
        Self {
            state: Mutex::new(VoiceState::Idle),
        }
    }
}

impl FakeVoiceSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_state(&self, state: VoiceState) {
        *self.state.lock().expect("voice state") = state;
    }
}

impl VoiceSourcePort for FakeVoiceSource {
    fn state(&self) -> VoiceState {
        *self.state.lock().expect("voice state")
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, envelope: EventEnvelope) {
        self.events.lock().expect("events").push(envelope);
    }
}

pub struct FakeRemoteLink {
    candidates: Mutex<Vec<RemoteCandidate>>,
    snapshot: Mutex<ConnectionSnapshot>,
    connect_error: Mutex<Option<EngineError>>,
}

impl Default for FakeRemoteLink {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeRemoteLink {
    pub fn new() -> Self {
        Self {
            candidates: Mutex::new(Vec::new()),
            snapshot: Mutex::new(ConnectionSnapshot::default()),
            connect_error: Mutex::new(None),
        }
    }

    pub fn with_candidate(self, candidate: RemoteCandidate) -> Self {
        self.candidates.lock().expect("candidates").push(candidate);
        self
    }

    pub fn fail_connect_with(&self, error: EngineError) {
        *self.connect_error.lock().expect("connect error") = Some(error);
    }
}

impl RemoteLinkPort for FakeRemoteLink {
    fn snapshot(&self) -> ConnectionSnapshot {
        self.snapshot.lock().expect("snapshot").clone()
    }

    fn scan(&self) -> CoreResult<Vec<RemoteCandidate>> {
        Ok(self.candidates.lock().expect("candidates").clone())
    }

    fn connect(&self, remote_id: &RemoteId) -> CoreResult<ConnectionSnapshot> {
        if let Some(error) = self.connect_error.lock().expect("connect error").take() {
            return Err(error);
        }
        let remote = self
            .candidates
            .lock()
            .expect("candidates")
            .iter()
            .find(|candidate| &candidate.remote_id == remote_id)
            .cloned()
            .ok_or_else(|| EngineError::new(ErrorDomain::Bluetooth, "remote-not-found"))?;
        let mut snapshot = self.snapshot.lock().expect("snapshot");
        snapshot.phase = ConnectionPhase::Ready;
        snapshot.remote = Some(remote);
        snapshot.last_error = None;
        Ok(snapshot.clone())
    }

    fn disconnect(&self) -> CoreResult<ConnectionSnapshot> {
        let mut snapshot = self.snapshot.lock().expect("snapshot");
        snapshot.phase = ConnectionPhase::Disconnected;
        snapshot.remote = None;
        Ok(snapshot.clone())
    }
}

pub struct FakeAudioSink {
    endpoints: Mutex<Vec<AudioEndpoint>>,
    snapshot: Mutex<AudioSnapshot>,
}

impl Default for FakeAudioSink {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeAudioSink {
    pub fn new() -> Self {
        Self {
            endpoints: Mutex::new(vec![
                AudioEndpoint {
                    id: "cable".to_owned(),
                    name: "CABLE Input".to_owned(),
                    recommended: true,
                },
                AudioEndpoint {
                    id: "speaker".to_owned(),
                    name: "Speakers".to_owned(),
                    recommended: false,
                },
            ]),
            snapshot: Mutex::new(AudioSnapshot::default()),
        }
    }

    pub fn set_state(&self, state: AudioState) {
        let mut snapshot = self.snapshot.lock().expect("audio snapshot");
        snapshot.state = state;
        snapshot.generation = snapshot.generation.next();
    }
}

impl AudioSinkPort for FakeAudioSink {
    fn snapshot(&self) -> AudioSnapshot {
        self.snapshot.lock().expect("audio snapshot").clone()
    }

    fn list_endpoints(&self) -> CoreResult<Vec<AudioEndpoint>> {
        Ok(self.endpoints.lock().expect("endpoints").clone())
    }

    fn select_endpoint(&self, endpoint_id: &str) -> CoreResult<AudioSnapshot> {
        let endpoint = self
            .endpoints
            .lock()
            .expect("endpoints")
            .iter()
            .find(|endpoint| endpoint.id == endpoint_id)
            .cloned()
            .ok_or_else(|| EngineError::new(ErrorDomain::Audio, "endpoint-not-found"))?;
        let mut snapshot = self.snapshot.lock().expect("audio snapshot");
        snapshot.state = AudioState::Ready;
        snapshot.endpoint = Some(endpoint);
        Ok(snapshot.clone())
    }

    fn set_gain_db(&self, gain_db: i16) -> CoreResult<AudioSnapshot> {
        let mut snapshot = self.snapshot.lock().expect("audio snapshot");
        snapshot.gain_db = gain_db;
        Ok(snapshot.clone())
    }
}

#[derive(Default)]
pub struct FakeActionSink {
    actions: Mutex<Vec<LogicalAction>>,
    fail_next: AtomicBool,
}

impl FakeActionSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn actions(&self) -> Vec<LogicalAction> {
        self.actions.lock().expect("actions").clone()
    }

    pub fn fail_next(&self) {
        self.fail_next.store(true, Ordering::SeqCst);
    }
}

impl ActionSinkPort for FakeActionSink {
    fn execute(&self, action: &LogicalAction) -> CoreResult<()> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(EngineError::new(ErrorDomain::Input, "inject-failed"));
        }
        self.actions.lock().expect("actions").push(action.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemorySettingsStore {
    document: Mutex<Option<SettingsDocument>>,
}

impl InMemorySettingsStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SettingsStorePort for InMemorySettingsStore {
    fn load(&self) -> CoreResult<AppSettings> {
        let document = self
            .document
            .lock()
            .expect("settings")
            .clone()
            .unwrap_or_default();
        let selected = document.global.selected_remote_id.clone();
        Ok(document.resolve(selected.as_deref(), "windows"))
    }

    fn save(&self, settings: &AppSettings) -> CoreResult<()> {
        let mut document = self
            .document
            .lock()
            .expect("settings")
            .clone()
            .unwrap_or_default();
        document.global = settings.clone();
        *self.document.lock().expect("settings") = Some(document.normalized());
        Ok(())
    }

    fn load_document(&self) -> CoreResult<SettingsDocument> {
        Ok(self
            .document
            .lock()
            .expect("settings")
            .clone()
            .unwrap_or_default())
    }

    fn save_document(&self, document: &SettingsDocument) -> CoreResult<()> {
        *self.document.lock().expect("settings") = Some(document.clone().normalized());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryButtonProfileStore {
    document: Mutex<Option<ButtonProfilesDocument>>,
}

impl InMemoryButtonProfileStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn document(&self) -> Option<ButtonProfilesDocument> {
        self.document.lock().expect("profiles").clone()
    }
}

impl ButtonProfilesStorePort for InMemoryButtonProfileStore {
    fn load(&self) -> CoreResult<Option<ButtonProfilesDocument>> {
        Ok(self.document())
    }

    fn save(&self, document: &ButtonProfilesDocument) -> CoreResult<()> {
        *self.document.lock().expect("profiles") = Some(document.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryButtonMappingStore {
    mappings: Mutex<Option<ButtonMappingSet>>,
}

impl InMemoryButtonMappingStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mappings(&self) -> Option<ButtonMappingSet> {
        self.mappings.lock().expect("mappings").clone()
    }
}

impl ButtonMappingStorePort for InMemoryButtonMappingStore {
    fn load(&self) -> CoreResult<Option<ButtonMappingSet>> {
        Ok(self.mappings())
    }

    fn save(&self, mappings: &ButtonMappingSet) -> CoreResult<()> {
        *self.mappings.lock().expect("mappings") = Some(mappings.clone());
        Ok(())
    }
}

pub fn recording_sink() -> (Arc<RecordingSink>, Arc<dyn EventSink>) {
    let sink = Arc::new(RecordingSink::new());
    let trait_sink: Arc<dyn EventSink> = sink.clone();
    (sink, trait_sink)
}
