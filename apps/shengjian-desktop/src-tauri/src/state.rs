use crate::host::HostPlatform;
use shengjian_app::CoreEngine;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct AppState {
    pub engine: Arc<CoreEngine>,
    pub host: Arc<dyn HostPlatform>,
    pub macro_path: PathBuf,
    pub foreground_stop: Arc<AtomicBool>,
}
