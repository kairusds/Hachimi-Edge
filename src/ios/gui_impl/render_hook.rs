//! iOS render hook — Metal backend via wgpu + egui-wgpu.
//!
//! Uses `egui_wgpu::wgpu` re-exports so there is no version conflict with
//! any other wgpu dependency in the graph.
//!
//! # Status
//! Scaffold — full CAMetalLayer / CADisplayLink integration is Phase 3.
//! The module compiles cleanly and defines the public API surface.

// Use wgpu types re-exported by egui-wgpu to avoid duplicate-version conflicts
use egui_wgpu::wgpu;
use egui_wgpu::{Renderer, ScreenDescriptor};

use crate::core::Error;

// ── renderer state ────────────────────────────────────────────────────────────

pub struct RenderState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub renderer: Renderer,
    pub surface_config: wgpu::SurfaceConfiguration,
}

static mut RENDER_STATE: Option<RenderState> = None;

/// Initialise the wgpu Metal device and egui-wgpu renderer.
///
/// Must be called from the game's render thread once the Metal surface
/// (CAMetalLayer) is available. `layer_ptr` is a `*mut CAMetalLayer`
/// cast to `usize` to remain FFI-safe at the declaration site.
pub async fn init_wgpu(layer_ptr: usize) -> Result<(), Error> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..Default::default()
    });

    // SAFETY: layer_ptr is a valid CAMetalLayer* obtained from the game's render thread.
    let surface = unsafe {
        instance.create_surface_unsafe(
            wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer_ptr as *mut std::ffi::c_void),
        )
        .map_err(|e| Error::GuiRendererInitError(e.to_string()))?
    };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| Error::GuiRendererInitError("no Metal adapter found".into()))?;

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .map_err(|e| Error::GuiRendererInitError(e.to_string()))?;

    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats[0];

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: 0,  // updated on first frame
        height: 0,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);

    let renderer = Renderer::new(
        &device,
        format,
        egui_wgpu::RendererOptions::default(),
    );

    unsafe {
        RENDER_STATE = Some(RenderState { device, queue, surface, renderer, surface_config });
    }

    info!("iOS: wgpu Metal renderer initialized (format={:?})", format);
    Ok(())
}

/// Called every frame (from the hooked CADisplayLink callback).
/// Runs the egui pass and composites onto the Metal surface.
pub fn render_frame(width: u32, height: u32) {
    let Some(rs) = (unsafe { RENDER_STATE.as_mut() }) else { return };
    let Some(gui_mutex) = crate::core::Gui::instance() else { return };
    let mut gui = gui_mutex.lock().unwrap();

    gui.set_screen_size(width as i32, height as i32);

    if rs.surface_config.width != width || rs.surface_config.height != height {
        rs.surface_config.width = width;
        rs.surface_config.height = height;
        rs.surface.configure(&rs.device, &rs.surface_config);
    }

    let output_frame = match rs.surface.get_current_texture() {
        Ok(f) => f,
        Err(e) => {
            error!("iOS render: surface error {:?}", e);
            return;
        }
    };
    let view = output_frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let full_output = gui.run();
    let clipped_primitives = gui
        .context
        .tessellate(full_output.shapes, full_output.pixels_per_point);

    let mut encoder = rs.device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor { label: Some("egui_encoder") },
    );

    let screen_descriptor = ScreenDescriptor {
        size_in_pixels: [width, height],
        pixels_per_point: full_output.pixels_per_point,
    };

    for (id, delta) in &full_output.textures_delta.set {
        rs.renderer.update_texture(&rs.device, &rs.queue, *id, delta);
    }
    rs.renderer.update_buffers(
        &rs.device,
        &rs.queue,
        &mut encoder,
        &clipped_primitives,
        &screen_descriptor,
    );

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rs.renderer.render(&mut render_pass, &clipped_primitives, &screen_descriptor);
    }

    for id in &full_output.textures_delta.free {
        rs.renderer.free_texture(id);
    }

    rs.queue.submit(std::iter::once(encoder.finish()));
    output_frame.present();
}

pub fn init() {
    info!("iOS: render_hook module loaded (wgpu/Metal, lazy init)");
}
