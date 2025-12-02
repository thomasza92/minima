use glam::{Mat4, Vec3};
use wgpu::{Buffer, Queue};
use winit::event::{DeviceEvent, ElementState, KeyEvent, WindowEvent};
use winit::keyboard::KeyCode;

pub fn forward_from_yaw_pitch(yaw: f32, pitch: f32) -> Vec3 {
    let cp = pitch.cos();
    let sp = pitch.sin();
    let cy = yaw.cos();
    let sy = yaw.sin();
    Vec3::new(cy * cp, sp, -sy * cp)
}

pub struct OrbitCamera {
    pub eye: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl OrbitCamera {
    pub fn new(eye: Vec3, yaw: f32, pitch: f32) -> Self {
        Self { eye, yaw, pitch }
    }
}

pub struct CameraController {
    move_forward: bool,
    move_back: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
    boost_speed: bool,
    base_speed: f32,
}

impl CameraController {
    pub fn new(base_speed: f32) -> Self {
        Self {
            move_forward: false,
            move_back: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
            boost_speed: false,
            base_speed,
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent, _cam: &mut OrbitCamera) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(code),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                if *repeat {
                    return;
                }
                let pressed = *state == ElementState::Pressed;
                match code {
                    KeyCode::KeyW => self.move_forward = pressed,
                    KeyCode::KeyS => self.move_back = pressed,
                    KeyCode::KeyA => self.move_left = pressed,
                    KeyCode::KeyD => self.move_right = pressed,
                    KeyCode::KeyJ => self.move_up = pressed,
                    KeyCode::KeyK => self.move_down = pressed,
                    KeyCode::ShiftLeft => self.boost_speed = pressed,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent, cam: &mut OrbitCamera) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            let sensitivity = 0.0025;
            cam.yaw -= (*dx as f32) * sensitivity;
            cam.pitch -= (*dy as f32) * sensitivity;
            let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
            cam.pitch = cam.pitch.clamp(-max_pitch, max_pitch);
        }
    }

    pub fn update(&mut self, cam: &mut OrbitCamera, dt: f32) {
        let mut movement = Vec3::ZERO;

        let forward = forward_from_yaw_pitch(cam.yaw, cam.pitch);
        let mut flat_forward = Vec3::new(forward.x, 0.0, forward.z);
        if flat_forward.length_squared() > 0.0 {
            flat_forward = flat_forward.normalize();
        }

        let mut right = flat_forward.cross(Vec3::Y);
        if right.length_squared() > 0.0 {
            right = right.normalize();
        }

        if self.move_forward {
            movement += flat_forward;
        }
        if self.move_back {
            movement -= flat_forward;
        }
        if self.move_right {
            movement += right;
        }
        if self.move_left {
            movement -= right;
        }
        if self.move_up {
            movement += Vec3::Y;
        }
        if self.move_down {
            movement -= Vec3::Y;
        }

        if movement.length_squared() > 0.0 {
            movement = movement.normalize();
            let mut speed = self.base_speed;
            if self.boost_speed {
                speed *= 5.0;
            }
            cam.eye += movement * speed * dt;
        }
    }
}

pub fn update_camera_buffer(
    queue: &Queue,
    camera_buf: &Buffer,
    camera: &OrbitCamera,
    width: u32,
    height: u32,
) {
    let forward = forward_from_yaw_pitch(camera.yaw, camera.pitch);
    let target = camera.eye + forward;
    let up = Vec3::Y;

    let view = Mat4::look_at_rh(camera.eye, target, up);
    let aspect = (width.max(1) as f32) / (height.max(1) as f32);
    let proj = Mat4::perspective_rh_gl(45.0_f32.to_radians(), aspect, 0.1, 100.0);

    let vp = (proj * view).to_cols_array();
    queue.write_buffer(camera_buf, 0, bytemuck::cast_slice(&[vp]));
}
