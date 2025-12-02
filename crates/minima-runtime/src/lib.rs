use std::{path::Path, time::Instant};

use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, WindowEvent},
    event_loop::EventLoopProxy,
    window::Window,
};

use wgpu::{
    Adapter, CommandEncoderDescriptor, Device, ExperimentalFeatures, Features, Instance, Limits,
    MemoryHints, PowerPreference, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    Texture, TextureFormat, TextureView, TextureViewDescriptor,
};

pub type RcWindow = std::sync::Arc<Window>;

use minima_3d::{Layouts, Renderer3D, create_bind_group_layouts};
use minima_camera::{CameraController, OrbitCamera, update_camera_buffer};
use minima_gltf::load_gltf_model;

use glam::Vec3;

const CAMERA_SPEED: f32 = 3.0;

pub struct Viewport {
    pub color: Texture,
    pub color_view: TextureView,
    pub depth: Texture,
    pub depth_view: TextureView,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

impl Viewport {
    pub fn new(device: &wgpu::Device, format: TextureFormat, width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewport_color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewport_depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            color,
            color_view,
            depth,
            depth_view,
            width,
            height,
            format,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Viewport::new(device, self.format, width, height);
    }
}

pub async fn create_graphics(window: RcWindow, proxy: EventLoopProxy<Graphics>) {
    let instance = Instance::default();
    let surface = instance
        .create_surface(std::sync::Arc::clone(&window))
        .unwrap();

    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Could not get an adapter (GPU).");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: Features::empty(),
            required_limits: Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            memory_hints: MemoryHints::Performance,
            trace: Default::default(),
            experimental_features: ExperimentalFeatures::disabled(),
        })
        .await
        .expect("Failed to get device");

    let size = window.inner_size();
    let width = size.width.max(1);
    let height = size.height.max(1);

    let surface_config = surface
        .get_default_config(&adapter, width, height)
        .expect("Failed to create surface config");
    surface.configure(&device, &surface_config);

    let layouts: Layouts = create_bind_group_layouts(&device);

    let model = load_gltf_model(
        &device,
        &queue,
        &layouts.material_bgl,
        Path::new("assets/BoomBox.glb"),
    )
    .await
    .expect("Failed to load glTF model");

    let model_xform = model.recommended_xform;

    let viewport = Viewport::new(
        &device,
        surface_config.format,
        surface_config.width,
        surface_config.height,
    );

    let renderer = Renderer3D::new(
        &device,
        &queue,
        surface_config.format,
        surface_config.width,
        surface_config.height,
        model,
        model_xform,
        &layouts,
    );

    let camera = OrbitCamera::new(Vec3::new(0.0, 0.0, 0.0), 0.0_f32, 0.0_f32);
    let controller = CameraController::new(CAMERA_SPEED);

    update_camera_buffer(
        &queue,
        &renderer.camera_buf,
        &camera,
        surface_config.width,
        surface_config.height,
    );

    let gfx = Graphics {
        window,
        instance,
        surface,
        surface_config,
        adapter,
        device,
        queue,
        renderer,
        camera,
        controller,
        viewport,
        last_frame_time: Instant::now(),
    };

    let _ = proxy.send_event(gfx);
}

#[allow(dead_code)]
pub struct Graphics {
    pub(crate) window: RcWindow,
    pub viewport: Viewport,
    instance: Instance,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    renderer: Renderer3D,
    camera: OrbitCamera,
    controller: CameraController,
    last_frame_time: Instant,
}

impl Graphics {
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn viewport_view(&self) -> &TextureView {
        &self.viewport.color_view
    }

    pub fn viewport_size(&self) -> (u32, u32) {
        let cfg = self.surface_config();
        (cfg.width, cfg.height)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
        self.viewport.resize(
            &self.device,
            self.surface_config.width,
            self.surface_config.height,
        );
        self.renderer
            .resize(&self.device, self.viewport.width, self.viewport.height);

        update_camera_buffer(
            &self.queue,
            &self.renderer.camera_buf,
            &self.camera,
            self.viewport.width,
            self.viewport.height,
        );
    }

    pub fn draw<F>(&mut self, overlay: F)
    where
        F: FnOnce(&mut Self, &TextureView, &mut wgpu::CommandEncoder),
    {
        let now = Instant::now();
        let mut dt = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        if dt > 0.1 {
            dt = 0.1;
        }
        self.controller.update(&mut self.camera, dt);

        update_camera_buffer(
            &self.queue,
            &self.renderer.camera_buf,
            &self.camera,
            self.viewport.width,
            self.viewport.height,
        );
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture.");

        let swap_view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        self.renderer
            .render(&mut encoder, &self.viewport.color_view);
        overlay(self, &swap_view, &mut encoder);
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
    pub fn draw_no_overlay(&mut self) {
        self.draw(|_, _, _| {});
    }
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        self.controller.handle_window_event(event, &mut self.camera);
    }
    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        self.controller.handle_device_event(event, &mut self.camera);
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn surface_config(&self) -> &SurfaceConfiguration {
        &self.surface_config
    }

    pub fn eye(&self) -> Vec3 {
        self.camera.eye
    }

    pub fn yaw(&self) -> f32 {
        self.camera.yaw
    }

    pub fn pitch(&self) -> f32 {
        self.camera.pitch
    }
}
