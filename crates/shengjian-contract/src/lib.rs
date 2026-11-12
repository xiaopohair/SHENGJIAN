//! Shengjian core contract v1.
//!
//! This crate owns language-neutral facts and types. It must not depend on
//! Windows, Tauri, a UI toolkit, or any platform I/O.

pub mod button_profile;
pub mod device;
pub mod error;
pub mod ids;
pub mod model;

pub use button_profile::{
    compose_profile_mappings, default_profile_id, is_profile_button, normalized_model,
    profile_buttons_for_model, ButtonProfile, ButtonProfilesDocument, TriggerBindings,
    ARN9_BUTTONS, BUTTON_PROFILES_SCHEMA_VERSION, DEFAULT_PROFILE_NAME, PROFILE_BUTTONS,
};
pub use device::{
    Capability, DeviceIdentity, DeviceProfile, HidIdentity, PlatformKind, PlatformSupport,
    ProfileCatalog, ProfileError, Protocol, Support, SupportStatus, Transport, TransportKind,
    TransportRole,
};
pub use error::{CoreResult, EngineError, ErrorDomain};
pub use ids::{DeviceId, Generation, InstanceId, RemoteId, SessionId};
pub use model::{
    AudioEndpoint, AudioFormat, AudioSnapshot, AudioState, ButtonActions, ButtonEdge,
    ButtonGesture, ButtonMappingSet, ButtonTrigger, ConnectionPhase, ConnectionSnapshot, CoreEvent,
    Direction, EngineCapabilities, EventEnvelope, FocusResult, FocusStrategy, LogicalAction,
    LogicalButton, MacroRunState, MouseActionKind, RemoteCandidate, VoiceSnapshot, VoiceState,
};
