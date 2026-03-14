//! iOS render hook — Metal backend via wgpu + egui-wgpu.
//!
//! Strategy: Hook Unity's CADisplayLink callback to inject the egui render pass
//! after the Unity frame is complete.  This is safer than hooking Metal at the
//! CAMetalLayer level and avoids Metal validation errors.
//!
//! # Status
//! This module is a scaffold.  Full wgpu + CAMetalLayer integration will be done
//! once the base compile target (`aarch64-apple-ios`) is verified to link cleanly.

use crate::core::{Error, Hachimi};

// ── wgpu / egui-wgpu state ─────────────────────────────────────────────────
// Stored as a static Option so the render hook closure can access it without locks.

struct RenderState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub renderer: egui_wgpu::Renderer,
    pub surface_config: wgpu::SurfaceConfiguration,
}

static mut RENDER_STATE: Option<RenderState> = None;

/// Initialise the wgpu Metal device and egui-wgpu renderer.
///
/// Must be called from the game's render thread after the Metal surface
/// (CAMetalLayer) is available.  The `layer_ptr` is a `*mut CAMetalLayer`
/// cast to `usize` to remain FFI-safe at declaration time.
pub async fn init_wgpu(layer_ptr: usize) -> Result<(), Error> {
    use wgpu::InstanceDescriptor;

    let instance = wgpu::Instance::new(&InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..Default::default()
    });

    // SAFETY: layer_ptr is a valid CAMetalLayer* from the game's render thread.
    let surface = unsafe {
        instance.create_surface_unsafe(
            wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer_ptr as *mut std::ffi::c_void)
        ).map_err(|e| Error::GuiRendererInitError(e.to_string()))?
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
    let format = caps.formats[0]; // prefer first (usually Bgra8Unorm)

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: 0,  // will be updated on first frame
        height: 0,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);

    let renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);

    unsafe {
        RENDER_STATE = Some(RenderState { device, queue, surface, renderer, surface_config });
    }

    info!("iOS: wgpu Metal renderer initialized (format={:?})", format);
    Ok(())
}

/// Called every frame (from the hooked CADisplayLink callback).
/// Runs the egui pass and presents to the Metal surface.
pub fn render_frame(width: u32, height: u32) {
    let Some(rs) = (unsafe { RENDER_STATE.as_mut() }) else { return };
    let Some(gui_mutex) = crate::core::Gui::instance() else { return };
    let mut gui = gui_mutex.lock().unwrap();

    gui.set_screen_size(width as i32, height as i32);

    // Reconfigure surface if size changed
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
    let view = output_frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

    let full_output = gui.run();
    let clipped_primitives = gui.context.tessellate(full_output.shapes, full_output.pixels_per_point);

    let mut encoder = rs.device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor { label: Some("egui_encoder") }
    );

    let screen_descriptor = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [width, height],
        pixels_per_point: full_output.pixels_per_point,
    };

    for (id, delta) in &full_output.textures_delta.set {
        rs.renderer.update_texture(&rs.device, &rs.queue, *id, delta);
    }
    rs.renderer.update_buffers(&rs.device, &rs.queue, &mut encoder, &clipped_primitives, &screen_descriptor);

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // composite over game frame
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
    // The wgpu device cannot be created synchronously here because it requires
    // the CAMetalLayer to already exist.  The actual init happens lazily on the
    // first render tick via a hook on Unity's display-link callback.
    info!("iOS: render_hook module loaded (wgpu/Metal, lazy init)");
}
