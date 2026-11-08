mod migrate;

use shengjian_app::EventSink;
use shengjian_contract::{ButtonMappingSet, ButtonProfilesDocument, EventEnvelope, RemoteId};
use shengjian_windows_adapter::{live_core_engine, live_core_engine_with_platform};
use std::error::Error;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct JsonSink;

impl EventSink for JsonSink {
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
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "scan" => scan(),
        "run" => run(&args),
        "migrate" => migrate::run(&args),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command: {other}; use help").into()),
    }
}

fn print_help() {
    println!(
        "shengjian-windows-host <command>\n\
         \n\
         scan\n\
           list paired remotes (token/name/model); does not connect\n\
         run [--connect <token-or-id>] [--seconds N] [--mappings <json>] [--profiles <json>] [--persist]\n\
           connect, select CABLE, apply optional core mappings/profiles, stream events\n\
           explicit imports are transient unless --persist is given\n\
         migrate --out-dir <dir> [--legacy-settings <json>] [--legacy-mappings <json>] [--legacy-profiles <json>]\n\
           convert legacy inputs into core settings/mappings/profiles files\n\
         help\n"
    );
}

fn init_log() -> bool {
    let path = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Shengjian")
        .join("Logs")
        .join("shengjian-diagnostic.log");
    shengjian_windows::initialize_diagnostic_log(
        path,
        shengjian_windows::DiagnosticLogMetadata {
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            app_build: "host".to_owned(),
            source_revision: String::new(),
            build_channel: "headless".to_owned(),
            release_tag: String::new(),
        },
    )
}

fn scan() -> Result<(), Box<dyn Error>> {
    println!("diagnostic log ready={}", init_log());
    let engine = live_core_engine(Arc::new(JsonSink))?;
    let candidates = engine.connection().scan()?;
    println!("paired={}", candidates.len());
    for candidate in candidates {
        println!(
            "candidate token={:?} name={} model={:?} device={:?} supported={}",
            candidate.token,
            candidate.name,
            candidate.model,
            candidate.device_id,
            candidate.is_supported
        );
    }
    engine.shutdown();
    Ok(())
}

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let seconds = parse_flag(args, "--seconds")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(15);
    let connect = parse_flag(args, "--connect");
    let mappings_path = parse_flag(args, "--mappings");
    let profiles_path = parse_flag(args, "--profiles");
    let persist = has_flag(args, "--persist");

    println!("diagnostic log ready={}", init_log());
    let (engine, platform) = live_core_engine_with_platform(Arc::new(JsonSink))?;
    println!("raw_input={:?}", platform.raw_input_snapshot());

    if let Some(path) = mappings_path {
        let mappings = read_json::<ButtonMappingSet>(&path)?;
        if persist {
            engine.set_button_mappings(mappings);
            println!("loaded and persisted core mappings from {path}");
        } else {
            engine.set_button_mappings_transient(mappings);
            println!("loaded transient core mappings from {path}");
        }
    }
    if let Some(path) = profiles_path {
        let profiles = read_json::<ButtonProfilesDocument>(&path)?;
        if persist {
            engine.set_button_profiles(profiles);
            println!("loaded and persisted core profiles from {path}");
        } else {
            engine.set_button_profiles_transient(profiles);
            println!("loaded transient core profiles from {path}");
        }
        engine.sync_foreground();
    }

    let candidates = engine.connection().scan()?;
    if candidates.is_empty() {
        return Err("no paired remote".into());
    }
    let selected =
        select_candidate(&candidates, connect.as_deref()).ok_or("requested remote not found")?;
    println!(
        "connecting token={:?} name={} model={:?}",
        selected.token, selected.name, selected.model
    );
    engine
        .connection()
        .connect(&RemoteId::new(selected.remote_id.as_str()))?;
    let connected = engine.connection().snapshot();

    let endpoints = engine.audio().list_endpoints()?;
    if let Some(endpoint) = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.recommended && endpoint.name.to_ascii_lowercase().contains("cable input")
        })
        .or_else(|| endpoints.iter().find(|endpoint| endpoint.recommended))
    {
        match engine.audio().select_endpoint(&endpoint.id) {
            Ok(_) => println!("selected audio endpoint={}", endpoint.name),
            Err(error) => println!("select audio endpoint failed: {error}"),
        }
    }

    if let Ok(document) = engine.settings().load_document() {
        let device_id = connected
            .remote
            .as_ref()
            .and_then(|remote| remote.device_id.as_ref())
            .map(|device| device.as_str());
        let effective = document.resolve(device_id, "windows");
        let gain = effective.gain_db.round().clamp(0.0, 24.0) as i16;
        if gain != engine.audio().snapshot().gain_db {
            if let Err(error) = engine.audio().set_gain_db(gain) {
                println!("apply settings gain failed: {error}");
            }
        }
        if let Some(endpoint_id) = effective.audio_endpoint_id.as_deref() {
            if let Err(error) = engine.audio().select_endpoint(endpoint_id) {
                println!("apply settings audio endpoint failed: {error}");
            }
        }
        println!(
            "loaded settings device={:?} gain={} endpoint={:?}",
            device_id, gain, effective.audio_endpoint_id
        );
    }

    println!("running for {seconds}s; press voice/buttons on the remote");
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        std::thread::sleep(remaining.min(Duration::from_millis(500)));
        if engine.sync_foreground_if_changed() {
            println!("foreground profile changed");
        }
    }
    let snapshot = engine.snapshots();
    println!(
        "final connection={:?} voice={:?} audio={:?}",
        snapshot.connection.phase, snapshot.voice.state, snapshot.audio.state
    );
    engine.disconnect()?;
    engine.shutdown();
    Ok(())
}

fn select_candidate<'a>(
    candidates: &'a [shengjian_contract::RemoteCandidate],
    requested: Option<&str>,
) -> Option<&'a shengjian_contract::RemoteCandidate> {
    match requested {
        Some(value) => candidates.iter().find(|candidate| {
            candidate.remote_id.as_str() == value || candidate.token.as_deref() == Some(value)
        }),
        None => candidates
            .iter()
            .find(|candidate| candidate.is_supported)
            .or_else(|| candidates.first()),
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

pub fn parse_flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|value| value == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

pub fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|value| value == name)
}

pub fn require_flag(args: &[String], name: &str) -> Result<String, Box<dyn Error>> {
    parse_flag(args, name).ok_or_else(|| format!("missing {name}").into())
}
