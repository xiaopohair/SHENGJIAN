#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{create_host, initialize_diagnostics, live_engine, note};

#[cfg(not(windows))]
compile_error!("shengjian-desktop currently has a Windows host implementation only");
