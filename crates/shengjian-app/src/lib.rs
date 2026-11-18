pub mod audio;
pub mod buttons;
pub mod catalog;
pub mod connection;
pub mod engine;
pub mod event_bus;
pub mod ports;
pub mod profiles;
pub mod settings;
pub mod settings_namespace;
pub mod voice;

pub use audio::AudioService;
pub use buttons::ButtonService;
pub use catalog::DeviceCatalog;
pub use connection::ConnectionService;
pub use engine::{CoreEngine, CoreEnginePorts, EngineSnapshot};
pub use event_bus::EventPublisher;
pub use ports::{
    ActionSinkPort, AudioSinkPort, ButtonMappingStorePort, ButtonProfilesStorePort,
    ButtonSourcePort, EventSink, ForegroundPort, RemoteLinkPort, SettingsStorePort,
    VoiceSourcePort,
};
pub use profiles::ButtonProfileService;
pub use settings::SettingsService;
pub use settings_namespace::{
    DeviceSettingsOverride, SettingsDocument, SETTINGS_NAMESPACE_SCHEMA_VERSION,
};
pub use voice::VoiceService;

/// 最小日志钩子：`shengjian-app` 内部不依赖任何日志载体，
/// 由宿主（Tauri / headless）在启动时注册成 `gatt_note` 之类的写入器。
///
/// 为什么需要（2026-11-18）：映射合成在 app 层完成，但排障时看不到它算出了什么；
/// 早先只能靠"下游有没有日志"反推，绕了好几轮。
static APP_LOG_HOOK: std::sync::OnceLock<fn(&str)> = std::sync::OnceLock::new();

/// 注册日志写入器（只生效一次；重复调用忽略）。
pub fn set_log_hook(hook: fn(&str)) {
    let _ = APP_LOG_HOOK.set(hook);
}

/// 写一行 app 层日志；未注册钩子时静默丢弃。
pub fn app_note(message: impl Into<String>) {
    if let Some(hook) = APP_LOG_HOOK.get() {
        hook(&message.into());
    }
}
