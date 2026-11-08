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
