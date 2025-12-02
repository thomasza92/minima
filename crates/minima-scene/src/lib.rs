use glam::Mat4;
use minima_3d::Model;
use std::sync::Arc;

pub struct ModelInstance {
    pub model: Arc<Model>,
    pub transform: Mat4,
}

pub struct Scene {
    pub models: Vec<ModelInstance>,
}

impl Scene {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    pub fn add_model(&mut self, model: Arc<Model>, transform: Mat4) {
        self.models.push(ModelInstance { model, transform });
    }
}
