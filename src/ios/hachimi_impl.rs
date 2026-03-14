use serde::{Deserialize, Serialize};
use crate::core::Hachimi;

/// Returns true if the given filename is the IL2CPP library.
/// On iOS Unity games, it's bundled as GameAssembly (no .so extension).
pub fn is_il2cpp_lib(filename: &str) -> bool {
    filename.contains("GameAssembly") || filename.ends_with("libil2cpp.dylib")
}

/// Returns true if the given filename is the CRI Ware middleware library.
pub fn is_criware_lib(filename: &str) -> bool {
    filename.contains("cri_ware") || filename.ends_with("libcri_ware_unity.dylib")
}

/// Called by the core after all hooks are installed.
pub fn on_hooking_finished(_hachimi: &Hachimi) {
    info!("iOS hooking finished");
}

/// iOS-specific configuration fields (none initially; expand as needed).
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Config {
    /// Position X of the floating action button (persisted across sessions)
    #[serde(default = "Config::default_fab_x")]
    pub fab_x: f32,
    /// Position Y of the floating action button
    #[serde(default = "Config::default_fab_y")]
    pub fab_y: f32,
}

impl Config {
    fn default_fab_x() -> f32 { 16.0 }
    fn default_fab_y() -> f32 { 100.0 }
}
