#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod catalog;
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
mod transcript_probe;
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
            // 迁移（改名）：旧布局目录尽力清理一次（旧助手进程还在时会失败，忽略）。
            helper::cleanup_legacy_layout(&app.handle());
            app.manage(splash::StartupClock::default());
            platform::initialize_diagnostics();
            catalog::initialize(app.handle());

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
                // 测试面板打开时：只放内存槽，不写历史也不广播（测试不该产生记录）。
                if transcript_probe::is_active() {
                    transcript_probe::store(record);
                    return;
                }
                let _ = transcript_history::append(record.clone());
                let _ = transcript_app.emit("transcript-captured", record);
            }));
            host.set_macro_library(macro_library);
            let mut tray_language = "zh-Hans";
            let mut open_window_at_launch = true;
            if let Ok(settings) = engine.settings().load() {
                open_window_at_launch = settings.open_window_at_launch;
                if settings.application_language.is_english() {
                    tray_language = "en";
                }
                host.set_enhanced_capture_enabled(settings.rc003_capture_enabled);
                host.set_voice_input_tool(settings.voice_input_tool.as_deref());
                host.set_voice_hotkey(settings.voice_hotkey.clone());
                host.set_transcript_capture_enabled(settings.local_transcript_history_enabled);
            }
            if !open_window_at_launch {
                // 「设置 > 启动与窗口 > 启动后隐藏到系统托盘」：收起启动画面，
                // 主窗口保持隐藏，应用只驻留托盘；托盘菜单或再次启动仍可打开。
                splash::dismiss_splash(app.handle());
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
            if let Ok(scale) = main_window.scale_factor() {
                shengjian_windows::display::set_scale_factor(scale);
            }
            if let Ok(theme) = main_window.theme() {
                shengjian_windows::display::set_dark_theme(matches!(theme, tauri::Theme::Dark));
            }
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
            tauri::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                shengjian_windows::display::set_scale_factor(*scale_factor);
            }
            tauri::WindowEvent::ThemeChanged(theme) => {
                shengjian_windows::display::set_dark_theme(matches!(theme, tauri::Theme::Dark));
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
            commands::forget_remote,
            commands::get_settings,
            commands::save_settings,
            commands::open_diagnostic_log_folder,
            commands::open_external_url,
            commands::list_audio_endpoints,
            commands::audio_dependency_status,
            commands::select_audio_endpoint,
            commands::set_gain_db,
            commands::button_profile_state,
            commands::activate_button_profile,
            commands::save_button_profiles,
            commands::recommended_button_profiles,
            commands::copy_recommended_button_profile,
            commands::export_profile_as_preset,
            commands::export_profiles,
            commands::import_profiles,
            commands::preview_import_profiles,
            commands::clear_button_profile_activation,
            commands::export_configuration,
            commands::import_configuration,
            commands::macro_library,
            commands::recommended_macro_definitions,
            commands::save_macro_library,
            commands::test_macro,
            commands::test_macro_draft,
            commands::cancel_macro,
            commands::macro_run_status,
            commands::export_macro_library,
            commands::import_macro_library,
            commands::preview_import_macro_library,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::hid_host_installed,
            commands::virtual_hid_status,
            commands::install_virtual_hid_driver,
            commands::uninstall_virtual_hid_driver,
            commands::refresh_virtual_hid_status,
            commands::install_hid_host,
            commands::uninstall_hid_host,
            commands::restart_hid_host,
            commands::host_capabilities,
            commands::remote_battery,
            commands::voice_level,
            commands::foreground_executable,
            commands::list_launchable_apps,
            commands::describe_app_targets,
            commands::canonicalize_app_target,
            commands::app_icon_data_urls,
            commands::voice_tool_catalog,
            commands::current_process_id,
            commands::resolve_app_target,
            commands::device_status,
            commands::raw_input_status,
            commands::set_button_listening,
            commands::common_phrases_document,
            commands::save_common_phrases,
            commands::open_common_phrases_panel,
            commands::device_support,
            commands::diagnostic_log,
            commands::export_diagnostic_bundle,
            commands::macro_run_history,
            commands::clear_macro_run_history,
            commands::reset_settings,
            commands::transcript_history,
            commands::open_voice_test_pad,
            commands::raise_voice_test_pad,
            commands::voice_test_pad_foreground,
            commands::collapse_voice_test_pad,
            commands::expand_voice_test_pad,
            commands::close_voice_test_pad,
            commands::voice_test_pad_text,
            commands::begin_transcript_probe,
            commands::end_transcript_probe,
            commands::take_transcript_probe,
            commands::clear_transcript_history,
            commands::set_tray_language,
            commands::set_tray_update,
            tray::sync_tray_icons,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Shengjian desktop");
}

fn tray_status_for(state: &state::AppState) -> tray::TrayStatus {
    let connection = state.engine.connection().snapshot();
    let model = connection
        .remote
        .as_ref()
        .and_then(|remote| remote.model.clone());
    let profiles = state.engine.button_profiles();
    let profile_name = state
        .engine
        .active_button_profile_id()
        .and_then(|id| profiles.profile(&id).map(|profile| profile.name.clone()))
        .or_else(|| {
            profiles
                .default_profile_for(model.as_deref())
                .map(|profile| profile.name.clone())
        })
        .unwrap_or_else(|| "默认".to_owned());
    let remote_name = connection
        .remote
        .as_ref()
        .map(|remote| remote.name.clone())
        .unwrap_or_default();
    let device_id = connection
        .remote
        .as_ref()
        .and_then(|remote| remote.device_id.as_ref())
        .map(|device| device.as_str().to_owned());
    let resolved = state
        .engine
        .settings()
        .load_document()
        .ok()
        .map(|document| document.resolve(device_id.as_deref(), state.host.platform_id()));
    let tool_id = state
        .engine
        .active_profile_voice_input_tool()
        .or_else(|| resolved.and_then(|settings| settings.voice_input_tool));
    let input_method = tool_id
        .map(|id| {
            shengjian_domain::voice_tools::tools()
                .into_iter()
                .find(|tool| tool.id == id)
                .map(|tool| tool.label)
                .unwrap_or(id)
        })
        .unwrap_or_default();
    tray::TrayStatus {
        profile_name,
        remote_name,
        input_method,
        battery_level: state.host.battery_level(),
    }
}

fn spawn_tray_battery_status(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("shengjian-tray-battery".to_owned())
        .spawn(move || {
            let mut tray_theme = None;
            loop {
                // 先更新方案/遥控器/输入法/电量，再做主题切换；
                // 语言/更新刷新不会再把完整状态覆盖掉。
                let status = app
                    .try_state::<state::AppState>()
                    .map(|state| tray_status_for(&state))
                    .unwrap_or_default();
                tray::set_tray_status(&app, status);
                std::thread::sleep(Duration::from_secs(5));
                tray::sync_brand_icons(&app, &mut tray_theme);
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
