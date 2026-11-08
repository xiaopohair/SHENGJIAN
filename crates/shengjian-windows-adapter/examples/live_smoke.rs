//! Real-device smoke harness for the live core.
//!
//! Examples:
//!   cargo run -p shengjian-windows-adapter --example live_smoke
//!   cargo run -p shengjian-windows-adapter --example live_smoke -- --connect <remote-id>
//!
//! It starts the real Windows platform (BLE worker, audio runtime, input hooks)
//! and should be run once per process.

use shengjian_app::EventSink;
use shengjian_contract::{
    ButtonActions, ButtonMappingSet, EventEnvelope, LogicalAction, LogicalButton, RemoteId,
};
use shengjian_windows_adapter::live_core_engine_with_platform;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

struct PrintSink;

impl EventSink for PrintSink {
    fn emit(&self, envelope: EventEnvelope) {
        if let Ok(json) = serde_json::to_string(&envelope) {
            println!("event {json}");
            shengjian_windows::gatt_note(format!("core_event {json}"));
        } else {
            println!(
                "event seq={} generation={}",
                envelope.seq, envelope.generation
            );
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut connect: Option<String> = None;
    let mut wait_seconds = 2_u64;
    let mut map_ok = false;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--connect" => {
                index += 1;
                connect = args.get(index).cloned();
            }
            "--wait" => {
                index += 1;
                wait_seconds = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(wait_seconds);
            }
            "--map-ok" => map_ok = true,
            "--help" | "-h" => {
                println!("usage: live_smoke [--connect <remote-id>] [--wait <seconds>] [--map-ok]");
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
        index += 1;
    }

    let log_path = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Shengjian")
        .join("Logs")
        .join("shengjian-diagnostic.log");
    let log_ready = shengjian_windows::initialize_diagnostic_log(
        log_path,
        shengjian_windows::DiagnosticLogMetadata {
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            app_build: "smoke".to_owned(),
            source_revision: String::new(),
            build_channel: "smoke".to_owned(),
            release_tag: String::new(),
        },
    );
    println!("diagnostic log ready={log_ready}");

    let (engine, platform) = live_core_engine_with_platform(Arc::new(PrintSink))?;
    println!("raw input after start: {:?}", platform.raw_input_snapshot());
    let candidates = engine.connection().scan()?;
    println!("found {} paired remote(s)", candidates.len());
    for candidate in &candidates {
        println!(
            "candidate id={} name={} model={:?} device={:?} supported={} token={:?}",
            candidate.remote_id,
            candidate.name,
            candidate.model,
            candidate.device_id,
            candidate.is_supported,
            candidate.token
        );
        if connect.is_none() && candidate.is_supported {
            connect = Some(candidate.remote_id.as_str().to_owned());
        }
    }

    if let Some(remote_id) = connect {
        println!("connecting {remote_id}");
        let snapshot = engine.connection().connect(&RemoteId::new(remote_id))?;
        println!(
            "connected phase={:?} generation={} remote={:?}",
            snapshot.phase,
            snapshot.generation.value(),
            snapshot.remote.as_ref().map(|remote| &remote.name)
        );
        let endpoints = engine.audio().list_endpoints()?;
        for endpoint in &endpoints {
            println!(
                "audio endpoint id={} name={} recommended={}",
                endpoint.id, endpoint.name, endpoint.recommended
            );
        }
        let preferred = endpoints
            .iter()
            .find(|endpoint| {
                endpoint.recommended && endpoint.name.to_ascii_lowercase().contains("cable input")
            })
            .or_else(|| endpoints.iter().find(|endpoint| endpoint.recommended));
        if let Some(endpoint) = preferred {
            match engine.audio().select_endpoint(&endpoint.id) {
                Ok(snapshot) => println!(
                    "selected audio endpoint={} state={:?}",
                    endpoint.name, snapshot.state
                ),
                Err(error) => println!("select audio endpoint failed: {error}"),
            }
        }
        if map_ok {
            let mut mappings = ButtonMappingSet::default();
            mappings.set_actions(
                LogicalButton::Ok,
                ButtonActions {
                    single: LogicalAction::FocusInput,
                    double: LogicalAction::FocusInput,
                    long: LogicalAction::FocusInput,
                    hold_repeat: None,
                    ok_context_click: false,
                },
            );
            engine.set_button_mappings(mappings);
            println!("test mapping enabled: OK single/double/long -> FocusInput");
        }
        println!(
            "voice state={:?}; audio snapshot gain={} state={:?}",
            engine.voice().snapshot().state,
            engine.audio().snapshot().gain_db,
            engine.audio().snapshot().state
        );
        println!(
            "raw input after connect: {:?}",
            platform.raw_input_snapshot()
        );
        println!("listening for {wait_seconds}s: press voice and buttons on the remote now");
        std::thread::sleep(Duration::from_secs(wait_seconds));
        println!(
            "after wait: voice state={:?} generation={}",
            engine.voice().snapshot().state,
            engine.voice().snapshot().generation.value()
        );
        println!("raw input after wait: {:?}", platform.raw_input_snapshot());
        println!("disconnecting");
        let snapshot = engine.disconnect()?;
        println!("disconnected phase={:?}", snapshot.phase);
    } else {
        println!("no supported candidate; pass --connect <remote-id> to force one");
    }

    engine.shutdown();
    Ok(())
}
