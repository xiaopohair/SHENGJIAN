use shengjian_app::EventSink;
use shengjian_contract::EventEnvelope;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct TauriEventSink {
    app: AppHandle,
}

impl TauriEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventSink for TauriEventSink {
    fn emit(&self, envelope: EventEnvelope) {
        if let Ok(json) = serde_json::to_string(&envelope) {
            crate::platform::note(format!("core_event {json}"));
        }
        let _ = self.app.emit("core-event", envelope);
    }
}
