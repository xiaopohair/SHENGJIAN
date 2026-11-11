use shengjian_app::{
    ButtonProfilesStorePort, ButtonSourcePort, CoreEngine, CoreEnginePorts, DeviceCatalog,
};
use shengjian_contract::{
    ButtonActions, ButtonEdge, ButtonMappingSet, ButtonProfilesDocument, ButtonTrigger, CoreEvent,
    CoreResult, EngineError, ErrorDomain, LogicalAction, LogicalButton, RemoteId,
};
use shengjian_testkit::{
    recording_sink, FakeActionSink, FakeAudioSink, FakeForeground, FakeRemoteLink, FakeVoiceSource,
    InMemoryButtonMappingStore, InMemoryButtonProfileStore, InMemorySettingsStore,
};
use std::sync::{Arc, Mutex};

#[test]
fn core_engine_composes_ports_and_keeps_one_ordered_event_stream() {
    let (sink, trait_sink) = recording_sink();
    let builder = shengjian_contract::RemoteCandidate {
        remote_id: RemoteId::new("remote-1"),
        name: "Xiaomi Bluetooth Remote 2 Pro".to_owned(),
        model: Some("RC003".to_owned()),
        device_id: Some(shengjian_contract::DeviceId::new("xiaomi-rc003")),
        is_supported: true,
        token: Some("token-1".to_owned()),
    };
    let engine = CoreEngine::new(
        trait_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new().with_candidate(builder)),
            audio: Arc::new(FakeAudioSink::new()),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );

    let connected = engine
        .connection()
        .connect(&RemoteId::new("remote-1"))
        .expect("connect");
    assert_eq!(connected.phase, shengjian_contract::ConnectionPhase::Ready);
    engine.voice().press().expect("voice press");
    engine.voice().release().expect("voice release");
    engine.voice().finish_drain().expect("drain");

    let mut mappings = ButtonMappingSet::default();
    mappings.set_actions(
        LogicalButton::Ok,
        ButtonActions {
            single: LogicalAction::FocusInput,
            ..ButtonActions::default()
        },
    );
    engine.set_button_mappings(mappings);
    engine.buttons().press(LogicalButton::Ok, 0).expect("press");
    engine
        .buttons()
        .release(LogicalButton::Ok, 10)
        .expect("release");

    let snapshots = engine.snapshots();
    assert_eq!(
        snapshots.connection.phase,
        shengjian_contract::ConnectionPhase::Ready
    );
    assert_eq!(snapshots.voice.state, shengjian_contract::VoiceState::Idle);
    assert!(snapshots.capabilities.bluetooth);

    let events = sink.events();
    assert!(!events.is_empty());
    for pair in events.windows(2) {
        assert!(pair[0].seq < pair[1].seq, "events must be ordered");
    }
    assert!(events.iter().any(|event| matches!(
        &event.event,
        CoreEvent::ButtonGesture { gesture } if gesture.trigger == ButtonTrigger::Single
    )));
    assert!(engine.event_publisher().last_seq() >= events.len() as u64);
}

#[derive(Default)]
struct FakeButtonSource {
    sink: Mutex<Option<Arc<dyn Fn(ButtonEdge) + Send + Sync>>>,
}

impl FakeButtonSource {
    fn emit(&self, edge: ButtonEdge) {
        let sink = self.sink.lock().expect("sink").clone();
        if let Some(sink) = sink {
            sink(edge);
        }
    }
}

impl ButtonSourcePort for FakeButtonSource {
    fn subscribe_edges(&self, sink: Arc<dyn Fn(ButtonEdge) + Send + Sync>) {
        *self.sink.lock().expect("sink") = Some(sink);
    }
}

#[test]
fn core_engine_routes_live_button_edges_through_gesture_and_action() {
    let (_, trait_sink) = recording_sink();
    let actions = Arc::new(FakeActionSink::new());
    let engine = CoreEngine::new(
        trait_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new()),
            audio: Arc::new(FakeAudioSink::new()),
            actions: actions.clone(),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );
    let mut mappings = ButtonMappingSet::default();
    mappings.set_actions(
        LogicalButton::Ok,
        ButtonActions {
            single: LogicalAction::FocusInput,
            ..ButtonActions::default()
        },
    );
    engine.set_button_mappings(mappings);

    let source = Arc::new(FakeButtonSource::default());
    engine.wire_button_source(source.clone());
    source.emit(ButtonEdge {
        button: LogicalButton::Ok,
        pressed: true,
        at_ms: 100,
    });
    source.emit(ButtonEdge {
        button: LogicalButton::Ok,
        pressed: false,
        at_ms: 120,
    });

    assert_eq!(actions.actions(), vec![LogicalAction::FocusInput]);
}

#[test]
fn disconnect_releases_voice_and_button_state() {
    let (_, trait_sink) = recording_sink();
    let engine = CoreEngine::new(
        trait_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new().with_candidate(
                shengjian_contract::RemoteCandidate {
                    remote_id: RemoteId::new("remote-1"),
                    name: "Mi Remote".to_owned(),
                    model: Some("RC003".to_owned()),
                    device_id: Some(shengjian_contract::DeviceId::new("xiaomi-rc003")),
                    is_supported: true,
                    token: None,
                },
            )),
            audio: Arc::new(FakeAudioSink::new()),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );
    engine
        .connection()
        .connect(&RemoteId::new("remote-1"))
        .expect("connect");
    engine.voice().press().expect("voice");
    engine
        .buttons()
        .press(LogicalButton::Ok, 0)
        .expect("button");

    let snapshot = engine.disconnect().expect("disconnect");
    assert_eq!(
        snapshot.phase,
        shengjian_contract::ConnectionPhase::Disconnected
    );
    assert_eq!(
        engine.snapshots().voice.state,
        shengjian_contract::VoiceState::Idle
    );
}

#[test]
fn engine_applies_auto_profile_after_foreground_sync() {
    let (_, trait_sink) = recording_sink();
    let foreground = Arc::new(FakeForeground::new());
    let engine = CoreEngine::new(
        trait_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new()),
            audio: Arc::new(FakeAudioSink::new()),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: foreground.clone(),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );

    let shortcut = |key: &str| LogicalAction::Shortcut {
        label: key.to_owned(),
        keys: vec![key.to_owned()],
    };
    let mut base = ButtonMappingSet::default();
    base.set_actions(
        LogicalButton::Up,
        ButtonActions {
            single: shortcut("W"),
            ..ButtonActions::default()
        },
    );
    engine.set_button_mappings(base);

    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert(ButtonTrigger::Single, shortcut("J"));
    let mut profile_bindings = std::collections::BTreeMap::new();
    profile_bindings.insert(LogicalButton::Up, bindings);
    engine.set_button_profiles(shengjian_contract::ButtonProfilesDocument {
        profiles: vec![shengjian_contract::ButtonProfile {
            id: "music".to_owned(),
            name: "Music".to_owned(),
            created_at: 0,
            app_targets: vec!["cloudmusic.exe".to_owned()],
            auto_apply: true,
            hold_repeat: None,
            hold_repeat_by_button: std::collections::BTreeMap::new(),
            voice_input_tool: None,
            bindings: profile_bindings,
        }],
        ..shengjian_contract::ButtonProfilesDocument::default()
    });

    foreground.set_executable(Some("CloudMusic.exe"));
    assert!(engine.sync_foreground_if_changed());
    assert!(!engine.sync_foreground_if_changed());
    assert_eq!(
        engine.buttons().snapshot().actions[&LogicalButton::Up].single,
        shortcut("J")
    );

    foreground.set_executable(Some("notepad.exe"));
    assert!(engine.sync_foreground_if_changed());
    assert_eq!(
        engine.buttons().snapshot().actions[&LogicalButton::Up].single,
        shortcut("W")
    );
}

#[test]
fn shutdown_after_disconnect_is_eventless() {
    let (sink, trait_sink) = recording_sink();
    let engine = CoreEngine::new(
        trait_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new().with_candidate(
                shengjian_contract::RemoteCandidate {
                    remote_id: RemoteId::new("remote-1"),
                    name: "Mi Remote".to_owned(),
                    model: Some("RC003".to_owned()),
                    device_id: Some(shengjian_contract::DeviceId::new("xiaomi-rc003")),
                    is_supported: true,
                    token: None,
                },
            )),
            audio: Arc::new(FakeAudioSink::new()),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );
    engine
        .connection()
        .connect(&RemoteId::new("remote-1"))
        .expect("connect");
    engine.disconnect().expect("disconnect");
    let before = sink.len();
    engine.shutdown();
    assert_eq!(sink.len(), before);
}

#[test]
fn engine_reconciles_polled_voice_state() {
    let (_, trait_sink) = recording_sink();
    let voice_source = Arc::new(FakeVoiceSource::new());
    let engine = CoreEngine::new(
        trait_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new()),
            audio: Arc::new(FakeAudioSink::new()),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: Some(voice_source.clone()),
        },
        DeviceCatalog::bundled().expect("catalog"),
    );

    voice_source.set_state(shengjian_contract::VoiceState::Streaming);
    engine.sync_voice_state();
    assert_eq!(
        engine.voice().snapshot().state,
        shengjian_contract::VoiceState::Streaming
    );

    voice_source.set_state(shengjian_contract::VoiceState::Draining);
    engine.sync_voice_state();
    assert_eq!(
        engine.voice().snapshot().state,
        shengjian_contract::VoiceState::Draining
    );

    voice_source.set_state(shengjian_contract::VoiceState::Idle);
    engine.sync_voice_state();
    assert_eq!(
        engine.voice().snapshot().state,
        shengjian_contract::VoiceState::Idle
    );
}

#[test]
fn corrupt_persisted_profile_does_not_block_engine_startup() {
    struct FailingProfileStore;

    impl ButtonProfilesStorePort for FailingProfileStore {
        fn load(&self) -> CoreResult<Option<ButtonProfilesDocument>> {
            Err(EngineError::new(
                ErrorDomain::Configuration,
                "profile-store-invalid",
            ))
        }

        fn save(&self, _document: &ButtonProfilesDocument) -> CoreResult<()> {
            Ok(())
        }
    }

    let (sink, trait_sink) = recording_sink();
    let engine = CoreEngine::new(
        trait_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new()),
            audio: Arc::new(FakeAudioSink::new()),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );
    engine.wire_profile_store(Arc::new(FailingProfileStore));
    assert!(!engine.load_persisted_button_config());
    assert!(sink
        .events()
        .iter()
        .any(|event| matches!(&event.event, CoreEvent::Error { .. })));
    assert!(engine.buttons().snapshot().actions.is_empty());
    engine.shutdown();
}

#[test]
fn persisted_button_config_survives_engine_restart() {
    let (_, first_sink) = recording_sink();
    let profiles = Arc::new(InMemoryButtonProfileStore::new());
    let mappings = Arc::new(InMemoryButtonMappingStore::new());
    let first = CoreEngine::new(
        first_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new()),
            audio: Arc::new(FakeAudioSink::new()),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );
    first.wire_profile_store(profiles.clone());
    first.wire_mapping_store(mappings.clone());

    let mut base = ButtonMappingSet::default();
    base.set_actions(
        LogicalButton::Up,
        ButtonActions {
            single: LogicalAction::FocusInput,
            ..ButtonActions::default()
        },
    );
    first.set_button_mappings_transient(base.clone());
    let document = ButtonProfilesDocument::default();
    first.set_button_profiles_transient(document.clone());
    assert!(profiles.document().is_none());
    assert!(mappings.mappings().is_none());

    first.set_button_mappings(base.clone());
    first.set_button_profiles(document.clone());
    // Saving normalizes the document, which always materializes the global profile.
    let document = document.normalized();
    first.shutdown();

    assert!(profiles.document().is_some());
    assert!(mappings.mappings().is_some());

    let (_, second_sink) = recording_sink();
    let second = CoreEngine::new(
        second_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new()),
            audio: Arc::new(FakeAudioSink::new()),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );
    second.wire_profile_store(profiles);
    second.wire_mapping_store(mappings);
    assert!(second.load_persisted_button_config());
    assert_eq!(
        second.buttons().snapshot().actions[&LogicalButton::Up].single,
        LogicalAction::FocusInput
    );
    assert_eq!(second.button_profiles(), document);
    second.shutdown();
}

#[test]
fn engine_reconciles_polled_audio_state() {
    let (sink, trait_sink) = recording_sink();
    let audio = Arc::new(FakeAudioSink::new());
    let engine = CoreEngine::new(
        trait_sink,
        CoreEnginePorts {
            remote: Arc::new(FakeRemoteLink::new()),
            audio: audio.clone(),
            actions: Arc::new(FakeActionSink::new()),
            settings: Arc::new(InMemorySettingsStore::new()),
            foreground: Arc::new(FakeForeground::new()),
            voice: None,
        },
        DeviceCatalog::bundled().expect("catalog"),
    );
    engine.sync_audio_state();
    audio.set_state(shengjian_contract::AudioState::Streaming);
    engine.sync_audio_state();
    assert_eq!(
        engine.audio().snapshot().state,
        shengjian_contract::AudioState::Streaming
    );
    assert!(sink
        .events()
        .iter()
        .any(|event| matches!(&event.event, CoreEvent::AudioChanged { .. })));
}
