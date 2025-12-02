use crate::project::Project;
use egui::Sense;
use egui::load::SizedTexture;
use minima_runtime::{Graphics, RcWindow, create_graphics};
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

const FPS: u64 = 120;
const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / FPS);

enum State {
    Ready(ReadyState),
    Init(Option<EventLoopProxy<Graphics>>),
}

struct ReadyState {
    gfx: Graphics,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    viewport_tex_id: egui::TextureId,
}

pub struct NewProjectDialog {
    pub open: bool,
    pub name_input: String,
    pub location_input: String,
    pub error: Option<String>,
}

impl NewProjectDialog {
    pub fn new() -> Self {
        Self {
            open: false,
            name_input: "MyGame".into(),
            location_input: "./projects".into(),
            error: None,
        }
    }
}

pub struct EditorUi {
    pub show_debug_panel: bool,
    pub camera_active: bool,
    pub cursor_grab_request: Option<bool>,
    pub current_project: Option<Project>,
    pub new_project: NewProjectDialog,
}

impl EditorUi {
    pub fn new() -> Self {
        Self {
            show_debug_panel: true,
            camera_active: false,
            cursor_grab_request: None,
            current_project: None,
            new_project: NewProjectDialog::new(),
        }
    }
}

pub struct App {
    state: State,
    render_target: Instant,
    ui: EditorUi,
}

impl App {
    pub fn new(event_loop: &EventLoop<Graphics>) -> Self {
        Self {
            state: State::Init(Some(event_loop.create_proxy())),
            render_target: Instant::now(),
            ui: EditorUi::new(),
        }
    }

    fn init_egui_for_graphics(
        gfx: &Graphics,
    ) -> (
        egui::Context,
        egui_winit::State,
        egui_wgpu::Renderer,
        egui::TextureId,
    ) {
        let egui_ctx = egui::Context::default();
        let viewport_id = egui_ctx.viewport_id();

        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            gfx.window(),
            None,
            None,
            None,
        );

        let mut egui_renderer = egui_wgpu::Renderer::new(
            gfx.device(),
            gfx.surface_config().format,
            egui_wgpu::RendererOptions::default(),
        );

        let viewport_tex_id = egui_renderer.register_native_texture(
            gfx.device(),
            gfx.viewport_view(),
            wgpu::FilterMode::Linear,
        );

        (egui_ctx, egui_state, egui_renderer, viewport_tex_id)
    }

    fn draw(&mut self) {
        if let State::Ready(ready) = &mut self.state {
            Self::draw_editor(ready, &mut self.ui);
        }
    }

    fn resized(&mut self, size: PhysicalSize<u32>) {
        if let State::Ready(ready) = &mut self.state {
            ready.gfx.resize(size);
            ready.egui_renderer.free_texture(&ready.viewport_tex_id);
            ready.viewport_tex_id = ready.egui_renderer.register_native_texture(
                ready.gfx.device(),
                ready.gfx.viewport_view(),
                wgpu::FilterMode::Linear,
            );
        }
    }
    fn draw_editor(ready: &mut ReadyState, ui_state: &mut EditorUi) {
        let raw_input = ready.egui_state.take_egui_input(ready.gfx.window());
        let viewport_tex_id = ready.viewport_tex_id;
        let cam_eye = ready.gfx.eye();
        let cam_yaw = ready.gfx.yaw();
        let cam_pitch = ready.gfx.pitch();
        let surface_cfg = ready.gfx.surface_config();
        let viewport_w = surface_cfg.width as f32;
        let viewport_h = surface_cfg.height as f32;
        let egui_ctx = ready.egui_ctx.clone();
        let ui_ptr: *mut EditorUi = ui_state;
        let full_output = egui_ctx.run(raw_input, |ctx| {
            let ui_state: &mut EditorUi = unsafe { &mut *ui_ptr };
            egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New Project…").clicked() {
                            ui_state.new_project.open = true;
                            ui_state.new_project.error = None;
                            ui.close();
                        }

                        if ui.button("Save Project").clicked() {
                            ui.close();
                        }

                        ui.separator();

                        if ui.button("Quit").clicked() {
                            ui.close();
                        }
                    });

                    ui.menu_button("View", |ui| {
                        ui.checkbox(&mut ui_state.show_debug_panel, "Show viewport debug panel");
                    });

                    ui.menu_button("Help", |ui| {
                        ui.label("Minima Editor");
                    });
                });
            });
            egui::SidePanel::left("scene_panel")
                .resizable(true)
                .default_width(220.0)
                .show(ctx, |ui| {
                    ui.heading("Scene");
                    ui.separator();
                    ui.label("Scene contents will go here.");
                });
            egui::SidePanel::right("inspector_panel")
                .resizable(true)
                .default_width(260.0)
                .show(ctx, |ui| {
                    ui.heading("Inspector");
                    ui.separator();

                    if let Some(proj) = &ui_state.current_project {
                        ui.label(format!("Project: {}", proj.config.project.name));
                        ui.label(format!("Root: {}", proj.root.to_string_lossy()));
                    } else {
                        ui.label("No project loaded.");
                    }
                });
            egui::TopBottomPanel::bottom("debug_panel")
                .resizable(true)
                .default_height(120.0)
                .show_animated(ctx, ui_state.show_debug_panel, |ui| {
                    ui.heading("Viewport Debug");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Camera eye:");
                        ui.monospace(format!("{:?}", cam_eye));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Yaw / Pitch:");
                        ui.monospace(format!("{:.3} / {:.3}", cam_yaw, cam_pitch));
                    });

                    ui.separator();
                    ui.label(
                        "Double-click viewport to capture camera.\n\
                         Esc to release.",
                    );
                });

            egui::CentralPanel::default().show(ctx, |ui| {
                let available = ui.available_size();

                if available.x > 0.0 && available.y > 0.0 && viewport_w > 0.0 && viewport_h > 0.0 {
                    let tex_aspect = viewport_w / viewport_h;
                    let panel_aspect = available.x / available.y;
                    let (w, h) = if panel_aspect > tex_aspect {
                        let h = available.y;
                        let w = h * tex_aspect;
                        (w, h)
                    } else {
                        let w = available.x;
                        let h = w / tex_aspect;
                        (w, h)
                    };

                    let viewport_size = egui::vec2(w, h);
                    let sized = SizedTexture::new(viewport_tex_id, viewport_size);
                    let image = egui::Image::from_texture(sized).sense(Sense::click_and_drag());
                    let response = ui.add(image);

                    if response.double_clicked() && !ui_state.camera_active {
                        ui_state.camera_active = true;
                        ui_state.cursor_grab_request = Some(true);
                    }

                    if ui_state.camera_active {
                        let painter = ui.painter();
                        painter.rect_stroke(
                            response.rect.shrink(1.0),
                            0.0,
                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                            egui::StrokeKind::Inside,
                        );
                        painter.text(
                            response.rect.right_top() + egui::vec2(-10.0, 10.0),
                            egui::Align2::RIGHT_TOP,
                            "Camera Control (Esc to exit)",
                            egui::FontId::proportional(14.0),
                            egui::Color32::YELLOW,
                        );
                    }
                } else {
                    ui.label("Viewport area is too small.");
                }
            });

            if ui_state.new_project.open {
                egui::Window::new("New Project")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.label("Create a new Minima game project.");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut ui_state.new_project.name_input);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Location:");
                            ui.text_edit_singleline(&mut ui_state.new_project.location_input);
                        });

                        if let Some(err) = &ui_state.new_project.error {
                            ui.colored_label(egui::Color32::RED, err);
                        }

                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("Create").clicked() {
                                let name = ui_state.new_project.name_input.trim();
                                let base = std::path::PathBuf::from(
                                    ui_state.new_project.location_input.trim(),
                                );

                                if name.is_empty() {
                                    ui_state.new_project.error =
                                        Some("Project name cannot be empty".into());
                                } else if base.as_os_str().is_empty() {
                                    ui_state.new_project.error =
                                        Some("Location cannot be empty".into());
                                } else {
                                    let project_dir = base.join(name);
                                    match Project::create_scaffold(&project_dir, name, "0.1.0") {
                                        Ok(project) => {
                                            ui_state.current_project = Some(project);
                                            ui_state.new_project.open = false;
                                            ui_state.new_project.error = None;
                                        }
                                        Err(e) => {
                                            ui_state.new_project.error =
                                                Some(format!("Failed to create: {e}"));
                                        }
                                    }
                                }
                            }

                            if ui.button("Cancel").clicked() {
                                ui_state.new_project.open = false;
                                ui_state.new_project.error = None;
                            }
                        });
                    });
            }
        });

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = full_output;

        ready
            .egui_state
            .handle_platform_output(ready.gfx.window(), platform_output);

        let paint_jobs = ready.egui_ctx.tessellate(shapes, pixels_per_point);

        if let Some(grab) = ui_state.cursor_grab_request.take() {
            let window = ready.gfx.window();
            if grab {
                window.set_cursor_visible(false);
                let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
            } else {
                window.set_cursor_visible(true);
                let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
            }
        }
        ready.gfx.draw(|gfx_inner, swap_view, encoder| {
            for (id, image_delta) in &textures_delta.set {
                ready.egui_renderer.update_texture(
                    gfx_inner.device(),
                    gfx_inner.queue(),
                    *id,
                    image_delta,
                );
            }
            for id in &textures_delta.free {
                ready.egui_renderer.free_texture(id);
            }

            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [
                    gfx_inner.surface_config().width,
                    gfx_inner.surface_config().height,
                ],
                pixels_per_point,
            };

            ready.egui_renderer.update_buffers(
                gfx_inner.device(),
                gfx_inner.queue(),
                encoder,
                &paint_jobs,
                &screen_descriptor,
            );

            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_overlay_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: swap_view,
                    depth_slice: None,
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

            let mut rpass = rpass.forget_lifetime();
            ready
                .egui_renderer
                .render(&mut rpass, &paint_jobs, &screen_descriptor);
        });
    }
}

impl ApplicationHandler<Graphics> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let State::Init(proxy) = &mut self.state {
            if let Some(proxy) = proxy.take() {
                let mut win_attr = Window::default_attributes();
                win_attr = win_attr.with_title("Minima Editor");

                let window: RcWindow = std::sync::Arc::new(
                    event_loop
                        .create_window(win_attr)
                        .expect("create window err."),
                );
                pollster::block_on(create_graphics(window, proxy));
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, graphics: Graphics) {
        let (egui_ctx, egui_state, egui_renderer, viewport_tex_id) =
            App::init_egui_for_graphics(&graphics);

        graphics.request_redraw();
        self.state = State::Ready(ReadyState {
            gfx: graphics,
            egui_ctx,
            egui_state,
            egui_renderer,
            viewport_tex_id,
        });
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        if self.render_target <= Instant::now() {
            self.render_target += FRAME_TIME;
            if let State::Ready(ready) = &mut self.state {
                ready.gfx.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => self.resized(size),
            WindowEvent::RedrawRequested => {
                self.draw();
                let now = Instant::now();
                if self.render_target <= now {
                    self.render_target = now + FRAME_TIME;
                    if let State::Ready(ready) = &mut self.state {
                        ready.gfx.request_redraw();
                    }
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            other => {
                if let State::Ready(ready) = &mut self.state {
                    let response = ready.egui_state.on_window_event(ready.gfx.window(), &other);
                    if response.repaint {
                        ready.gfx.request_redraw();
                    }
                    if let WindowEvent::KeyboardInput {
                        event: key_event, ..
                    } = &other
                    {
                        use winit::event::ElementState;
                        use winit::keyboard::{KeyCode, PhysicalKey};

                        if let PhysicalKey::Code(KeyCode::Escape) = key_event.physical_key {
                            if key_event.state == ElementState::Pressed
                                && !key_event.repeat
                                && self.ui.camera_active
                            {
                                self.ui.camera_active = false;
                                self.ui.cursor_grab_request = Some(false);
                                ready.gfx.request_redraw();
                            }
                        }
                    }
                    if self.ui.camera_active && !response.consumed {
                        ready.gfx.handle_window_event(&other);
                    }
                }
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let State::Ready(ready) = &mut self.state {
            if self.ui.camera_active {
                ready.gfx.handle_device_event(&event);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.render_target));
    }
}
