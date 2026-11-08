use crate::audio::AudioService;
use crate::buttons::ButtonService;
use crate::catalog::DeviceCatalog;
use crate::connection::ConnectionService;
use crate::event_bus::EventPublisher;
use crate::ports::{
    ActionSinkPort, AudioSinkPort, ButtonMappingStorePort, ButtonProfilesStorePort,
    ButtonSourcePort, EventSink, ForegroundPort, RemoteLinkPort, SettingsStorePort,
    VoiceSourcePort,
};
use crate::profiles::ButtonProfileService;
use crate::settings::SettingsService;
use crate::voice::VoiceService;
use shengjian_contract::{
    AudioSnapshot, AudioState, ButtonMappingSet, ButtonProfilesDocument, ConnectionPhase,
    ConnectionSnapshot, CoreEvent, CoreResult, EngineCapabilities, VoiceSnapshot, VoiceState,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Platform ports required to assemble a core engine.
pub struct CoreEnginePorts {
    pub remote: Arc<dyn RemoteLinkPort>,
    pub audio: Arc<dyn AudioSinkPort>,
    pub actions: Arc<dyn ActionSinkPort>,
    pub settings: Arc<dyn SettingsStorePort>,
    pub foreground: Arc<dyn ForegroundPort>,
    pub voice: Option<Arc<dyn VoiceSourcePort>>,
}

/// In-process facade over the core services.
///
/// The platform adapters are injected as ports, so this stays free of Windows,
/// Tauri and UI dependencies. A later headless/IPC layer can expose the same
/// facade without changing service code.
pub struct CoreEngine {
    publisher: Arc<EventPublisher>,
    connection: ConnectionService,
    voice: Arc<VoiceService>,
    voice_source: Arc<Mutex<Option<Arc<dyn VoiceSourcePort>>>>,
    buttons: Arc<ButtonService>,
    audio: Arc<AudioService>,
    settings: SettingsService,
    profiles: ButtonProfileService,
    active_profile_id: Mutex<Option<String>>,
    profile_store: Mutex<Option<Arc<dyn ButtonProfilesStorePort>>>,
    mapping_store: Mutex<Option<Arc<dyn ButtonMappingStorePort>>>,
    catalog: DeviceCatalog,
    ticker: EngineTicker,
}

/// Drives gesture deadlines and reconciles polled platform voice state while a
/// live engine is running. Tests and headless callers may also call `advance`
/// explicitly.
struct EngineTicker {
    stop: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl EngineTicker {
    fn start(
        buttons: Arc<ButtonService>,
        audio: Arc<AudioService>,
        voice: Arc<VoiceService>,
        voice_source: Arc<Mutex<Option<Arc<dyn VoiceSourcePort>>>>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name("shengjian-engine-ticker".to_owned())
            .spawn(move || {
                while !thread_stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(10));
                    let _ = buttons.advance(now_ms());
                    audio.sync();
                    let audio_state = audio.snapshot().state;
                    let source = voice_source.lock().expect("voice source").clone();
                    if let Some(source) = source {
                        let current = voice.snapshot().state;
                        let target = reconcile_voice_state(source.state(), audio_state, current);
                        let _ = voice.sync_state(target);
                    }
                }
            })
            .expect("engine ticker thread");
        Self {
            stop,
            handle: Mutex::new(Some(handle)),
        }
    }

    fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.lock().expect("ticker handle").take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EngineTicker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Platform voice state can become idle before the audio queue finishes
/// draining. Keep the observable voice state in `draining` until audio leaves
/// `draining`, so `streaming -> draining -> idle` is not coalesced away.
fn reconcile_voice_state(source: VoiceState, audio: AudioState, current: VoiceState) -> VoiceState {
    if source == VoiceState::Idle && audio == AudioState::Draining && current != VoiceState::Idle {
        return VoiceState::Draining;
    }
    source
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSnapshot {
    pub connection: ConnectionSnapshot,
    pub voice: VoiceSnapshot,
    pub audio: AudioSnapshot,
    pub capabilities: EngineCapabilities,
}

impl CoreEngine {
    pub fn new(sink: Arc<dyn EventSink>, ports: CoreEnginePorts, catalog: DeviceCatalog) -> Self {
        let publisher = Arc::new(EventPublisher::new(sink));
        let connection = ConnectionService::new(ports.remote, Arc::clone(&publisher));
        let voice = Arc::new(VoiceService::new(Arc::clone(&publisher)));
        let voice_source = Arc::new(Mutex::new(ports.voice));
        let buttons = Arc::new(ButtonService::new(Arc::clone(&publisher), ports.actions));
        let audio = Arc::new(AudioService::new(ports.audio, Arc::clone(&publisher)));
        let ticker = EngineTicker::start(
            Arc::clone(&buttons),
            Arc::clone(&audio),
            Arc::clone(&voice),
            Arc::clone(&voice_source),
        );
        let settings = SettingsService::new(ports.settings);
        let profiles = ButtonProfileService::new(ports.foreground);
        Self {
            publisher,
            connection,
            voice,
            voice_source,
            buttons,
            audio,
            settings,
            profiles,
            active_profile_id: Mutex::new(None),
            profile_store: Mutex::new(None),
            mapping_store: Mutex::new(None),
            catalog,
            ticker,
        }
    }

    pub fn connection(&self) -> &ConnectionService {
        &self.connection
    }

    pub fn voice(&self) -> &VoiceService {
        &self.voice
    }

    pub fn wire_voice_source(&self, source: Arc<dyn VoiceSourcePort>) {
        *self.voice_source.lock().expect("voice source") = Some(source);
    }

    pub fn wire_profile_store(&self, store: Arc<dyn ButtonProfilesStorePort>) {
        *self.profile_store.lock().expect("profile store") = Some(store);
    }

    pub fn wire_mapping_store(&self, store: Arc<dyn ButtonMappingStorePort>) {
        *self.mapping_store.lock().expect("mapping store") = Some(store);
    }

    /// Load persisted base mappings and profile document. Missing files are not
    /// an error; corrupt files surface as an `engine.error` event.
    pub fn load_persisted_button_config(&self) -> bool {
        let mut loaded = false;
        let mapping_store = self.mapping_store.lock().expect("mapping store").clone();
        if let Some(store) = mapping_store {
            match store.load() {
                Ok(Some(mappings)) => {
                    self.profiles.set_base_mappings(mappings);
                    loaded = true;
                }
                Ok(None) => {}
                Err(error) => self.publish_persistence_error(error),
            }
        }
        let profile_store = self.profile_store.lock().expect("profile store").clone();
        if let Some(store) = profile_store {
            match store.load() {
                Ok(Some(document)) => {
                    self.profiles.set_document(document);
                    loaded = true;
                }
                Ok(None) => {}
                Err(error) => self.publish_persistence_error(error),
            }
        }
        if loaded {
            self.apply_effective_mappings();
        }
        loaded
    }

    fn persist_profile_document(&self) {
        let store = self.profile_store.lock().expect("profile store").clone();
        if let Some(store) = store {
            if let Err(error) = store.save(&self.profiles.document()) {
                self.publish_persistence_error(error);
            }
        }
    }

    fn persist_base_mappings(&self) {
        let store = self.mapping_store.lock().expect("mapping store").clone();
        if let Some(store) = store {
            let mappings = self.profiles.base_mappings();
            if let Err(error) = store.save(&mappings) {
                self.publish_persistence_error(error);
            }
        }
    }

    fn publish_persistence_error(&self, error: shengjian_contract::EngineError) {
        self.publisher.publish(
            self.connection.snapshot().generation,
            CoreEvent::Error { error },
        );
    }

    /// Reconcile polled platform voice state immediately (ticker also does this).
    pub fn sync_voice_state(&self) {
        let source = self.voice_source.lock().expect("voice source").clone();
        if let Some(source) = source {
            let _ = self.voice.sync_state(source.state());
        }
    }

    /// Reconcile polled platform audio state immediately (ticker also does this).
    pub fn sync_audio_state(&self) {
        self.audio.sync();
    }

    pub fn buttons(&self) -> &ButtonService {
        &self.buttons
    }

    pub fn audio(&self) -> &AudioService {
        &self.audio
    }

    pub fn settings(&self) -> &SettingsService {
        &self.settings
    }

    pub fn catalog(&self) -> &DeviceCatalog {
        &self.catalog
    }

    pub fn event_publisher(&self) -> &Arc<EventPublisher> {
        &self.publisher
    }

    pub fn snapshots(&self) -> EngineSnapshot {
        EngineSnapshot {
            connection: self.connection.snapshot(),
            voice: self.voice.snapshot(),
            audio: self.audio.snapshot(),
            capabilities: self.catalog.engine_capabilities(),
        }
    }

    pub fn set_button_mappings(&self, mappings: ButtonMappingSet) {
        self.set_button_mappings_transient(mappings);
        self.persist_base_mappings();
    }

    /// Apply mappings for this process only; unlike `set_button_mappings`, this
    /// does not write the configured mapping store.
    pub fn set_button_mappings_transient(&self, mappings: ButtonMappingSet) {
        self.profiles.set_base_mappings(mappings);
        self.apply_effective_mappings();
    }

    pub fn set_button_profiles(&self, document: ButtonProfilesDocument) {
        self.set_button_profiles_transient(document);
        self.persist_profile_document();
    }

    /// Apply profiles for this process only; useful for imports/tests that must
    /// not overwrite the user's persisted configuration.
    pub fn set_button_profiles_transient(&self, document: ButtonProfilesDocument) {
        self.profiles.set_document(document);
        self.apply_effective_mappings();
    }

    pub fn button_profiles(&self) -> ButtonProfilesDocument {
        self.profiles.document()
    }

    pub fn active_button_profile_id(&self) -> Option<String> {
        self.profiles.active_profile_id()
    }

    /// Profile-level voice input tool override, if the active profile sets one.
    pub fn active_profile_voice_input_tool(&self) -> Option<String> {
        self.profiles
            .active_profile()
            .and_then(|profile| profile.voice_input_tool)
    }

    pub fn activate_button_profile(&self, profile_id: &str) -> bool {
        let activated = self.profiles.activate(profile_id);
        if activated {
            self.apply_effective_mappings();
            self.persist_profile_document();
        }
        activated
    }

    pub fn clear_button_profile_activation(&self) {
        self.profiles.clear_activation();
        self.apply_effective_mappings();
        self.persist_profile_document();
    }

    /// Recompute the profile overlay after the host observes a foreground change.
    pub fn sync_foreground(&self) {
        self.apply_effective_mappings();
    }

    /// Poll-friendly foreground sync: only reapplies mappings when the active
    /// profile id changes, so gesture state is not reset every tick.
    pub fn sync_foreground_if_changed(&self) -> bool {
        let active = self.profiles.active_profile_id();
        let mut cached = self.active_profile_id.lock().expect("active profile id");
        if *cached == active {
            return false;
        }
        *cached = active;
        drop(cached);
        self.buttons.set_mappings(self.profiles.sync_foreground());
        true
    }

    fn apply_effective_mappings(&self) {
        let active = self.profiles.active_profile_id();
        let mappings = self.profiles.sync_foreground();
        *self.active_profile_id.lock().expect("active profile id") = active;
        self.buttons.set_mappings(mappings);
    }

    /// Disconnect while releasing voice/button state owned by the core.
    /// The platform adapter's error is returned after local state is reconciled.
    /// Already-disconnected/idle calls are local-only and idempotent.
    pub fn disconnect(&self) -> CoreResult<ConnectionSnapshot> {
        let current = self.connection.snapshot();
        if matches!(
            current.phase,
            ConnectionPhase::Idle | ConnectionPhase::Disconnected
        ) {
            self.voice.force_idle();
            self.buttons.reset();
            return Ok(current);
        }
        let result = self.connection.disconnect();
        self.voice.force_idle();
        self.buttons.reset();
        result
    }

    /// Idempotent process-exit path. Platform worker shutdown remains the
    /// adapter/host's responsibility; this reconciles core state and stops the
    /// gesture deadline ticker.
    pub fn shutdown(&self) {
        let _ = self.disconnect();
        self.ticker.stop();
    }

    /// Connect a live/injected button source to the engine's button service.
    ///
    /// The callback remains owned by the platform for the process lifetime;
    /// current Windows Raw Input has no unsubscribe path.
    pub fn wire_button_source(&self, source: Arc<dyn ButtonSourcePort>) {
        let buttons = Arc::clone(&self.buttons);
        source.subscribe_edges(Arc::new(move |edge| {
            let _ = buttons.handle_edge(edge);
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::reconcile_voice_state;
    use shengjian_contract::{AudioState, VoiceState};

    #[test]
    fn keeps_voice_draining_while_audio_drains() {
        assert_eq!(
            reconcile_voice_state(
                VoiceState::Idle,
                AudioState::Draining,
                VoiceState::Streaming,
            ),
            VoiceState::Draining
        );
        assert_eq!(
            reconcile_voice_state(VoiceState::Idle, AudioState::Draining, VoiceState::Draining,),
            VoiceState::Draining
        );
    }

    #[test]
    fn idle_when_audio_finished_or_voice_already_idle() {
        assert_eq!(
            reconcile_voice_state(VoiceState::Idle, AudioState::Ready, VoiceState::Draining),
            VoiceState::Idle
        );
        assert_eq!(
            reconcile_voice_state(VoiceState::Idle, AudioState::Draining, VoiceState::Idle),
            VoiceState::Idle
        );
    }
}
