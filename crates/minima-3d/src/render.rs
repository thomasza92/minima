use crate::depth::create_depth;
use crate::model::{Model, create_model_ubo};
use crate::pipeline::{Layouts, create_pipeline};
use wgpu::*;

pub struct Renderer3D {
    pub render_pipeline: RenderPipeline,
    pub depth_view: TextureView,
    pub depth_tex: Texture,
    pub camera_bg: BindGroup,
    pub camera_buf: Buffer,
    pub model_bg: BindGroup,
    pub model_buf: Buffer,
    pub model: Model,
}

impl Renderer3D {
    pub fn new(
        device: &Device,
        _queue: &Queue,
        surface_format: TextureFormat,
        width: u32,
        height: u32,
        model: Model,
        model_xform: glam::Mat4,
        layouts: &Layouts,
    ) -> Self {
        let (depth_view, depth_tex) = create_depth(device, width, height);

        let (render_pipeline, camera_bg, camera_buf, model_bgl) =
            create_pipeline(device, surface_format, layouts);

        let (model_buf, model_bg) = create_model_ubo(device, &model_bgl, model_xform);

        Self {
            render_pipeline,
            depth_view,
            depth_tex,
            camera_bg,
            camera_buf,
            model_bg,
            model_buf,
            model,
        }
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        let (dv, dt) = create_depth(device, width, height);
        self.depth_view = dv;
        self.depth_tex = dt;
    }

    pub fn render(&self, encoder: &mut CommandEncoder, target_view: &TextureView) {
        let mut r_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("scene_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target_view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        r_pass.set_pipeline(&self.render_pipeline);
        r_pass.set_bind_group(0, &self.camera_bg, &[]);
        r_pass.set_bind_group(1, &self.model_bg, &[]);

        for mesh in &self.model.meshes {
            let mat = &self.model.materials[mesh.material_id.min(self.model.materials.len() - 1)];
            r_pass.set_bind_group(2, &mat.bind_group, &[]);
            r_pass.set_vertex_buffer(0, mesh.vbuf.slice(..));
            r_pass.set_index_buffer(mesh.ibuf.slice(..), IndexFormat::Uint32);
            r_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}
