use shengjian_app::{
    AudioService, ButtonService, ConnectionService, DeviceCatalog, EventPublisher, SettingsService,
    VoiceService,
};
use shengjian_contract::{
    ButtonActions, ButtonMappingSet, ButtonTrigger, CoreEvent, DeviceId, Direction, FocusStrategy,
    LogicalAction, LogicalButton, MouseActionKind, RemoteId, VoiceState,
};
use shengjian_domain::AppSettings;
use shengjian_testkit::{
    recording_sink, FakeActionSink, FakeAudioSink, FakeRemoteLink, InMemorySettingsStore,
};
use std::sync::Arc;

fn candidate() -> shengjian_contract::RemoteCandidate {
    shengjian_contract::RemoteCandidate {
        remote_id: RemoteId::new("remote-1"),
        name: "Xiaomi Bluetooth Remote 2 Pro".to_owned(),
        model: Some("RC003".to_owned()),
        device_id: Some(DeviceId::new("xiaomi-rc003")),
        is_supported: true,
        token: Some("rc003-token".to_owned()),
    }
}

#[test]
fn connection_scan_connect_and_disconnect_emit_ordered_events() {
    let (sink, trait_sink) = recording_sink();
    let publisher = Arc::new(EventPublisher::new(trait_sink));
    let link = Arc::new(FakeRemoteLink::new().with_candidate(candidate()));
    let service = ConnectionService::new(link, publisher);

    let candidates = service.scan().expect("scan");
    assert_eq!(candidates.len(), 1);
    let connected = service
        .connect(&RemoteId::new("remote-1"))
        .expect("connect");
    assert_eq!(connected.phase, shengjian_contract::ConnectionPhase::Ready);
    assert_eq!(connected.generation.value(), 1);
    let disconnected = service.disconnect().expect("disconnect");
    assert!(disconnected.remote.is_none());
    assert_eq!(disconnected.generation.value(), 2);

    let events = sink.events();
    assert!(events.len() >= 5);
    assert_eq!(events[0].seq, 1);
    assert!(events.iter().any(|event| matches!(
        &event.event,
        CoreEvent::ConnectionChanged { snapshot }
            if snapshot.phase == shengjian_contract::ConnectionPhase::Ready
    )));
}

#[test]
fn voice_press_release_and_force_idle_are_paired() {
    let (sink, trait_sink) = recording_sink();
    let publisher = Arc::new(EventPublisher::new(trait_sink));
    let service = VoiceService::new(publisher);

    let streaming = service.press().expect("press");
    assert_eq!(streaming.state, VoiceState::Streaming);
    let draining = service.release().expect("release");
    assert_eq!(draining.state, VoiceState::Draining);
    assert!(service.press().is_err(), "must not restart while draining");
    assert_eq!(
        service.finish_drain().expect("drain").state,
        VoiceState::Idle
    );
    service.press().expect("press again");
    assert_eq!(service.force_idle().state, VoiceState::Idle);

    assert!(sink
        .events()
        .iter()
        .any(|event| matches!(&event.event, CoreEvent::VoiceChanged { snapshot } if snapshot.state == VoiceState::Streaming)));
}

#[test]
fn button_single_double_and_long_are_dispatched() {
    let (sink, trait_sink) = recording_sink();
    let publisher = Arc::new(EventPublisher::new(trait_sink));
    let actions = Arc::new(FakeActionSink::new());
    let service = ButtonService::new(publisher, actions.clone());
    let mut mappings = ButtonMappingSet::default();
    mappings.set_actions(
        LogicalButton::Ok,
        ButtonActions {
            single: LogicalAction::Shortcut {
                label: "Enter".to_owned(),
                keys: vec!["Enter".to_owned()],
            },
            double: LogicalAction::OpenApp {
                app: "wechat".to_owned(),
                focus: FocusStrategy::Direct,
            },
            long: LogicalAction::FocusInput,
            hold_repeat: None,
            ok_context_click: false,
        },
    );
    service.set_mappings(mappings);

    service.press(LogicalButton::Ok, 0).expect("press");
    service.release(LogicalButton::Ok, 20).expect("release");
    service.advance(400).expect("advance single");
    assert_eq!(actions.actions().len(), 1);

    service.press(LogicalButton::Ok, 500).expect("press");
    service.release(LogicalButton::Ok, 520).expect("release");
    service.press(LogicalButton::Ok, 600).expect("press");
    service.release(LogicalButton::Ok, 650).expect("release");
    assert_eq!(actions.actions().len(), 2);
    assert!(matches!(
        actions.actions()[1],
        LogicalAction::OpenApp { .. }
    ));

    service.press(LogicalButton::Ok, 1000).expect("press");
    service.advance(1600).expect("advance long");
    service.release(LogicalButton::Ok, 1700).expect("release");
    assert_eq!(actions.actions().last(), Some(&LogicalAction::FocusInput));
    assert!(sink.events().iter().any(|event| matches!(
        &event.event,
        CoreEvent::ButtonGesture { gesture } if gesture.trigger == ButtonTrigger::Long
    )));
}

#[test]
fn connection_failure_is_reported_and_can_retry() {
    let (sink, trait_sink) = recording_sink();
    let publisher = Arc::new(EventPublisher::new(trait_sink));
    let link = Arc::new(FakeRemoteLink::new().with_candidate(candidate()));
    link.fail_connect_with(shengjian_contract::EngineError::retryable(
        shengjian_contract::ErrorDomain::Bluetooth,
        "connect-timeout",
    ));
    let service = ConnectionService::new(link, publisher);
    assert!(service.connect(&RemoteId::new("remote-1")).is_err());
    assert_eq!(
        service.snapshot().phase,
        shengjian_contract::ConnectionPhase::Failed
    );
    assert!(sink.events().iter().any(|event| matches!(
        &event.event,
        CoreEvent::ConnectionChanged { snapshot } if snapshot.last_error.is_some()
    )));
    let recovered = service.connect(&RemoteId::new("remote-1")).expect("retry");
    assert_eq!(recovered.phase, shengjian_contract::ConnectionPhase::Ready);
}

#[test]
fn action_failure_is_emitted_as_engine_error() {
    let (sink, trait_sink) = recording_sink();
    let publisher = Arc::new(EventPublisher::new(trait_sink));
    let actions = Arc::new(FakeActionSink::new());
    actions.fail_next();
    let service = ButtonService::new(publisher, actions);
    let mut mappings = ButtonMappingSet::default();
    mappings.set_actions(
        LogicalButton::Ok,
        ButtonActions {
            single: LogicalAction::FocusInput,
            ..ButtonActions::default()
        },
    );
    service.set_mappings(mappings);
    assert!(service.press(LogicalButton::Ok, 0).is_err());
    assert!(sink
        .events()
        .iter()
        .any(|event| matches!(&event.event, CoreEvent::Error { .. })));
}

#[test]
fn audio_gain_is_clamped_by_the_service_contract() {
    let (sink, trait_sink) = recording_sink();
    let publisher = Arc::new(EventPublisher::new(trait_sink));
    let sink_port = Arc::new(FakeAudioSink::new());
    let service = AudioService::new(sink_port, publisher);
    assert_eq!(service.list_endpoints().expect("list").len(), 2);
    let selected = service.select_endpoint("cable").expect("select");
    assert_eq!(selected.endpoint.expect("endpoint").name, "CABLE Input");
    assert_eq!(service.set_gain_db(12).expect("gain").gain_db, 12);
    assert!(service.set_gain_db(25).is_err());
    assert!(sink
        .events()
        .iter()
        .any(|event| matches!(&event.event, CoreEvent::AudioChanged { .. })));
}

#[test]
fn settings_round_trip_and_bundled_catalog_capabilities() {
    let store = Arc::new(InMemorySettingsStore::new());
    let service = SettingsService::new(store);
    let settings = AppSettings {
        gain_db: 12.0,
        ..AppSettings::default()
    };
    service.save(&settings).expect("save");
    assert_eq!(service.load().expect("load").gain_db, 12.0);

    let catalog = DeviceCatalog::bundled().expect("catalog");
    let capabilities = catalog.engine_capabilities();
    assert!(capabilities.bluetooth);
    assert!(capabilities.audio);
    assert!(capabilities.full_key_capture);
    assert_eq!(
        capabilities.profiles,
        vec![
            DeviceId::new("xiaomi-rc003"),
            // ARN9 于 2026-11-12 真机验收，升为 implemented 后进入能力列表。
            DeviceId::new("xiaomi-arn9"),
        ]
    );
}

#[test]
fn ok_context_click_fires_after_recent_cursor_move() {
    let (sink, trait_sink) = recording_sink();
    let publisher = Arc::new(EventPublisher::new(trait_sink));
    let actions = Arc::new(FakeActionSink::new());
    let service = ButtonService::new(publisher, actions.clone());
    let mut mappings = ButtonMappingSet::default();
    mappings.set_actions(
        LogicalButton::Up,
        ButtonActions {
            single: LogicalAction::Mouse {
                kind: MouseActionKind::Move,
                direction: Some(Direction::Up),
                amount: 10,
            },
            ..ButtonActions::default()
        },
    );
    mappings.set_actions(
        LogicalButton::Ok,
        ButtonActions {
            single: LogicalAction::FocusInput,
            ok_context_click: true,
            ..ButtonActions::default()
        },
    );
    service.set_mappings(mappings);

    service.press(LogicalButton::Up, 0).expect("move");
    service
        .press(LogicalButton::Ok, 100)
        .expect("context click");
    service.release(LogicalButton::Ok, 120).expect("release");

    assert_eq!(
        actions.actions(),
        vec![
            LogicalAction::Mouse {
                kind: MouseActionKind::Move,
                direction: Some(Direction::Up),
                amount: 10,
            },
            LogicalAction::Mouse {
                kind: MouseActionKind::Click,
                direction: None,
                amount: 0,
            },
        ]
    );
    assert!(!sink
        .events()
        .iter()
        .any(|event| matches!(&event.event, CoreEvent::ButtonGesture { gesture } if gesture.button == LogicalButton::Ok)));
}
