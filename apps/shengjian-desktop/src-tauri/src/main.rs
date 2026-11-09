#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod events;
mod helper;
mod host;
mod macro_history;
mod macros;
mod platform;
mod splash;
mod state;
mod transcript_history;
mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;

fn main() {
    match shengjian_windows::key_host::parse_host_args(std::env::args()) {
        shengjian_windows::key_host::HostArgParse::Launch(launch) => {
            std::process::exit(shengjian_windows::key_host::run_host(launch));
        }
        shengjian_windows::key_host::HostArgParse::Invalid(reason) => {
            eprintln!("invalid key host arguments: {reason}");
            std::process::exit(5);
        }
        shengjian_windows::key_host::HostArgParse::NotHost => {}
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch during startup should skip the splash and focus
            // the main window instead of leaving it hidden.
            splash::reveal_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(splash::StartupClock::default());
            platform::initialize_diagnostics();

            let sink = Arc::new(events::TauriEventSink::new(app.handle().clone()));
            let (engine, windows_platform) =
                platform::live_engine(sink).map_err(|error| error.to_string())?;
            let macro_path = macros::default_path();
            let macro_library = match macros::load(&macro_path) {
                Ok(document) => document,
                Err(error) => {
                    println!(
                        "shengjian macro library unavailable code={} message={}",
                        error.code, error.message
                    );
                    Default::default()
                }
            };
            let host = platform::create_host(windows_platform);
            let key_host_ready = shengjian_windows::key_host::start_global();
            if !key_host_ready {
                eprintln!("key host unavailable; voice F5 suppression disabled");
            }
            spawn_key_host_supervisor();
            let transcript_app = app.handle().clone();
            host.configure_transcript_capture(Arc::new(move |record| {
                let _ = transcript_history::append(record.clone());
                let _ = transcript_app.emit("transcript-captured", record);
            }));
            host.set_macro_library(macro_library);
            let mut tray_language = "zh-Hans";
            if let Ok(settings) = engine.settings().load() {
                if settings.application_language.is_english() {
                    tray_language = "en";
                }
                host.set_enhanced_capture_enabled(settings.rc003_capture_enabled);
                host.set_voice_input_tool(settings.voice_input_tool);
                host.set_voice_hotkey(settings.voice_hotkey.clone());
                host.set_transcript_capture_enabled(settings.local_transcript_history_enabled);
            }
            let engine = Arc::new(engine);
            commands::apply_startup_audio_settings(&engine, host.platform_id());
            let foreground_stop = Arc::new(AtomicBool::new(false));
            spawn_foreground_poll(
                Arc::clone(&engine),
                Arc::clone(&host),
                Arc::clone(&foreground_stop),
            );
            let history_host = Arc::clone(&host);
            app.manage(state::AppState {
                engine,
                host,
                macro_path,
                foreground_stop,
            });
            tray::setup_tray(app, tray_language)?;
            // The frontend calls commands as soon as it mounts, so create the
            // main webview only after AppState exists. Config windows are
            // created before the setup hook runs; `create: false` keeps this
            // window out of that early pass. This also prevents the startup
            // race where get_settings ran before .manage(AppState).
            let main_config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or_else(|| "main window config missing".to_string())?;
            let main_window =
                tauri::WebviewWindowBuilder::from_config(app.handle(), &main_config)?.build()?;
            // Keep the centered startup position even if the platform defers
            // placement until the hidden window is first mapped.
            let _ = main_window.center();
            // Tray / taskbar icons follow the taskbar theme: bright pine on dark,
            // deep pine on light (brand: tray-*-color / tray-*-deep).
            let mut tray_theme = None;
            tray::sync_brand_icons(app.handle(), &mut tray_theme);
            spawn_tray_battery_status(app.handle().clone());
            spawn_macro_history_recorder(history_host);
            splash::schedule_fallback(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "splash" {
                    // The splash is only dismissed by the startup handoff (or
                    // the timeout fallback). Alt+F4 must not strand the app.
                    api.prevent_close();
                    return;
                }
                let close_to_tray = window
                    .app_handle()
                    .try_state::<state::AppState>()
                    .and_then(|state| state.engine.settings().load().ok())
                    .map(|settings| settings.close_to_tray)
                    .unwrap_or(true);
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    api.prevent_close();
                    window.app_handle().exit(0);
                }
            }
            tauri::WindowEvent::Destroyed => {
                if window.label() == "main" {
                    if let Some(state) = window.app_handle().try_state::<state::AppState>() {
                        state.foreground_stop.store(true, Ordering::Relaxed);
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            splash::finish_startup,
            commands::engine_snapshot,
            commands::engine_event_cursor,
            commands::scan_remotes,
            commands::connect_remote,
            commands::disconnect_remote,
            commands::get_settings,
            commands::save_settings,
            commands::open_diagnostic_log_folder,
            commands::open_external_url,
            commands::list_audio_endpoints,
            commands::select_audio_endpoint,
            commands::set_gain_db,
            commands::button_profile_state,
            commands::activate_button_profile,
            commands::save_button_profiles,
            commands::export_profiles,
            commands::import_profiles,
            commands::clear_button_profile_activation,
            commands::export_configuration,
            commands::import_configuration,
            commands::macro_library,
            commands::save_macro_library,
            commands::test_macro,
            commands::cancel_macro,
            commands::macro_run_status,
            commands::export_macro_library,
            commands::import_macro_library,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::rc003_helper_installed,
            commands::install_rc003_helper,
            commands::host_capabilities,
            commands::remote_battery,
            commands::voice_level,
            commands::foreground_executable,
            commands::resolve_app_target,
            commands::device_status,
            commands::device_support,
            commands::diagnostic_log,
            commands::export_diagnostic_bundle,
            commands::macro_run_history,
            commands::clear_macro_run_history,
            commands::reset_settings,
            commands::transcript_history,
            commands::clear_transcript_history,
            commands::set_tray_language,
            commands::set_tray_update,
            tray::sync_tray_icons,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Shengjian desktop");
}

fn spawn_tray_battery_status(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("shengjian-tray-battery".to_owned())
        .spawn(move || {
            let mut tray_theme = None;
            loop {
                std::thread::sleep(Duration::from_secs(15));
                tray::sync_brand_icons(&app, &mut tray_theme);
                let tooltip = app
                    .try_state::<state::AppState>()
                    .map(|state| {
                        let english = state
                            .engine
                            .settings()
                            .load()
                            .map(|settings| settings.application_language.is_english())
                            .unwrap_or(false);
                        match state.host.battery_level() {
                            Some(level) if level <= 20 => {
                                if english {
                                    format!("声键 · Low battery {level}%")
                                } else {
                                    format!("声键 · 电量低 {level}%")
                                }
                            }
                            Some(level) => {
                                if english {
                                    format!("声键 · Battery {level}%")
                                } else {
                                    format!("声键 · 电量 {level}%")
                                }
                            }
                            None => "声键".to_owned(),
                        }
                    })
                    .unwrap_or_else(|| "声键".to_owned());
                if let Some(tray) = app.tray_by_id("main-tray") {
                    let _ = tray.set_tooltip(Some(&tooltip));
                }
            }
        })
        .expect("tray battery thread");
}

fn spawn_key_host_supervisor() {
    std::thread::Builder::new()
        .name("shengjian-key-host-supervisor".to_owned())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(5));
            if !shengjian_windows::key_host::global_ready() {
                let _ = shengjian_windows::key_host::start_global();
            }
        })
        .expect("key host supervisor thread");
}

fn spawn_macro_history_recorder(host: Arc<dyn host::HostPlatform>) {
    std::thread::Builder::new()
        .name("shengjian-macro-history".to_owned())
        .spawn(move || {
            let mut last: std::collections::BTreeMap<String, String> =
                std::collections::BTreeMap::new();
            let mut primed = false;
            loop {
                std::thread::sleep(Duration::from_secs(1));
                for info in host.macro_infos() {
                    let status = host.macro_run_status(&info.macro_id);
                    let Some(report) = status.report else {
                        continue;
                    };
                    if status.running {
                        continue;
                    }
                    let Ok(key) = serde_json::to_string(&report) else {
                        continue;
                    };
                    let is_new = last.get(&info.macro_id) != Some(&key);
                    if primed && is_new {
                        let result = report
                            .get("result")
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown")
                            .to_owned();
                        let elapsed_ms = report
                            .get("elapsedMs")
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0);
                        let steps = report
                            .get("steps")
                            .and_then(|value| value.as_array())
                            .map(|steps| steps.len())
                            .unwrap_or(0);
                        let _ = macro_history::append(macro_history::MacroHistoryEntry {
                            macro_id: info.macro_id.clone(),
                            name: info.name,
                            result,
                            elapsed_ms,
                            at_unix: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            steps,
                        });
                    }
                    last.insert(info.macro_id, key);
                }
                primed = true;
            }
        })
        .expect("macro history thread");
}

fn spawn_foreground_poll(
    engine: Arc<shengjian_app::CoreEngine>,
    host: Arc<dyn host::HostPlatform>,
    stop: Arc<AtomicBool>,
) {
    std::thread::Builder::new()
        .name("shengjian-desktop-foreground".to_owned())
        .spawn(move || {
            let mut last_voice_tool: Option<Option<String>> = None;
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(500));
                engine.sync_foreground_if_changed();
                let base_tool = engine.settings().load_document().ok().and_then(|document| {
                    let device_id = engine
                        .connection()
                        .snapshot()
                        .remote
                        .and_then(|remote| remote.device_id)
                        .map(|device| device.as_str().to_owned());
                    document
                        .resolve(device_id.as_deref(), host.platform_id())
                        .voice_input_tool
                        .map(voice_tool_token)
                });
                let desired = engine.active_profile_voice_input_tool().or(base_tool);
                if last_voice_tool.as_ref() != Some(&desired) {
                    host.set_voice_input_tool_name(desired.as_deref());
                    last_voice_tool = Some(desired);
                }
            }
        })
        .expect("foreground poll thread");
}

fn voice_tool_token(tool: shengjian_domain::VoiceInputTool) -> String {
    match tool {
        shengjian_domain::VoiceInputTool::Wechat => "wechat",
        shengjian_domain::VoiceInputTool::Doubao => "doubao",
        shengjian_domain::VoiceInputTool::Vokie => "vokie",
        shengjian_domain::VoiceInputTool::Other => "other",
    }
    .to_owned()
}
