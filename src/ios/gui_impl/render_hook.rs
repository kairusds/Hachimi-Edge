//! iOS render hook — Metal/wgpu stub (Phase 3, not yet implemented).
//!
//! Full CAMetalLayer + egui-wgpu integration is pending Phase 3.
//! This file must compile cleanly for the iOS target.

use egui_wgpu;

pub fn init() {
    info!("iOS: render_hook initialized (Metal rendering is Phase 3 stub)");
    // Phase 3: hook into CADisplayLink and composite egui via wgpu Metal backend
    let _ = egui_wgpu::WgpuSetup::default(); // ensure egui_wgpu is used (suppress unused dep warning)
}

pub fn render_frame(_width: u32, _height: u32) {
    // Phase 3: render egui onto the Metal surface
}
