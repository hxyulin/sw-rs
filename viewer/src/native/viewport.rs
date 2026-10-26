// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Depth-tested model texture embedded in an RXUI custom element.
use rxui::{
    CustomElement, CustomPrepare, ElementInfo, UiError,
    drawing::{PaintSession, Rect, TextureBinding, TextureBindingOptions, TextureDraw},
    wgpu,
};
use std::sync::Arc;
use sw_viewer::project::Scene;
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub pan: [f32; 2],
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.65,
            pitch: 0.45,
            zoom: 1.,
            pan: [0.; 2],
        }
    }
}
#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<f32>,
    center: [f32; 3],
    radius: f32,
}
impl Mesh {
    pub fn from_scene(scene: &Scene) -> Self {
        let mut vertices = Vec::with_capacity(scene.vertices.len() / 9 * 12);
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        let mut batch = 0;
        for (i, v) in scene.vertices.as_chunks::<9>().0.iter().enumerate() {
            for axis in 0..3 {
                min[axis] = min[axis].min(v[axis]);
                max[axis] = max[axis].max(v[axis]);
            }
            while scene
                .batches
                .get(batch)
                .is_some_and(|b| i >= b.first_vertex + b.vertex_count)
            {
                batch += 1;
            }
            let material = scene
                .batches
                .get(batch)
                .and_then(|b| b.material_index)
                .and_then(|i| scene.materials.get(i));
            let rgb = material.map(|m| {
                m.saved_surface
                    .as_ref()
                    .map_or(m.base_color, |s| s.base_color)
            });
            vertices.extend_from_slice(v);
            vertices.extend_from_slice(&rgb.unwrap_or([v[6], v[7], v[8]]));
        }
        if vertices.is_empty() {
            return Self::default();
        }
        Self {
            vertices,
            center: std::array::from_fn(|i| (min[i] + max[i]) * 0.5),
            radius: min
                .iter()
                .zip(max)
                .map(|(a, b)| (b - a).powi(2))
                .sum::<f32>()
                .sqrt()
                .max(1e-12)
                * 0.5,
        }
    }
}

#[derive(Clone)]
pub struct Viewport {
    pub mesh: Arc<Mesh>,
    pub camera: Camera,
    pub material_colors: bool,
    pub background: [f32; 4],
}
impl PartialEq for Viewport {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.mesh, &other.mesh)
            && self.camera == other.camera
            && self.material_colors == other.material_colors
            && self.background == other.background
    }
}
#[derive(Default)]
pub struct State {
    renderer: Option<Renderer>,
    last: Option<Viewport>,
}
struct Renderer {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    size: [u32; 2],
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    image: TextureBinding,
}
fn bytes(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_ne_bytes()).collect()
}
fn attachment(device: &wgpu::Device, size: [u32; 2], depth: bool) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("CAD viewport"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if depth {
                wgpu::TextureFormat::Depth32Float
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | if depth {
                    wgpu::TextureUsages::empty()
                } else {
                    wgpu::TextureUsages::TEXTURE_BINDING
                },
            view_formats: &[],
        })
        .create_view(&Default::default())
}
impl Renderer {
    fn new(cx: &mut CustomPrepare<'_>, size: [u32; 2]) -> Result<Self, UiError> {
        let device = cx.device;
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CAD camera"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("CAD shading"),
            source: wgpu::ShaderSource::Wgsl(include_str!("viewport.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("CAD shaded mesh"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vertex"), compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 48, step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x3] })] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fragment"), compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }),
            multisample: Default::default(), multiview_mask: None, cache: None,
        });
        let color = attachment(device, size, false);
        let depth = attachment(device, size, true);
        let image = cx
            .painter
            .create_image_binding(&color, TextureBindingOptions::default())?;
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 48,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        Ok(Self {
            pipeline,
            uniform,
            group,
            vertices,
            size,
            color,
            depth,
            image,
        })
    }
}
impl CustomElement for Viewport {
    type State = State;
    fn prepare(
        &self,
        state: &mut State,
        element: &ElementInfo<'_>,
        cx: &mut CustomPrepare<'_>,
    ) -> Result<(), UiError> {
        let b = element.content_bounds;
        let limit = cx.device.limits().max_texture_dimension_2d;
        let size = [
            (b.width * cx.raster_scale).round() as u32,
            (b.height * cx.raster_scale).round() as u32,
        ]
        .map(|v| v.clamp(1, limit));
        if state.renderer.as_ref().is_none_or(|r| r.size != size) {
            state.renderer = Some(Renderer::new(cx, size)?);
            state.last = None;
        }
        let renderer = state.renderer.as_mut().unwrap();
        cx.painter.prepare_image(&renderer.image, cx.format)?;
        if state.last.as_ref() == Some(self) {
            return Ok(());
        }
        if state
            .last
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(&old.mesh, &self.mesh))
            && !self.mesh.vertices.is_empty()
        {
            renderer.vertices = cx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("CAD vertices"),
                    contents: &bytes(&self.mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        }
        let Camera {
            yaw,
            pitch,
            zoom,
            pan,
        } = self.camera;
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let scale = 0.82 * zoom / self.mesh.radius.max(1e-12);
        let aspect = size[0] as f32 / size[1] as f32;
        let [x, y, z] = self.mesh.center;
        let uniforms = [
            x,
            y,
            z,
            self.mesh.radius.max(1e-12),
            cy * scale / aspect,
            0.,
            -sy * scale / aspect,
            pan[0] / aspect,
            -sp * sy * scale,
            cp * scale,
            -sp * cy * scale,
            pan[1],
            cp * sy,
            sp,
            cp * cy,
            0.,
            if self.material_colors { 1. } else { 0. },
            0.,
            0.,
            0.,
        ];
        cx.queue
            .write_buffer(&renderer.uniform, 0, &bytes(&uniforms));
        let mut encoder = cx.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CAD viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &renderer.color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.background[0] as f64,
                            g: self.background[1] as f64,
                            b: self.background[2] as f64,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &renderer.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !self.mesh.vertices.is_empty() {
                pass.set_pipeline(&renderer.pipeline);
                pass.set_bind_group(0, &renderer.group, &[]);
                pass.set_vertex_buffer(0, renderer.vertices.slice(..));
                pass.draw(0..(self.mesh.vertices.len() / 12) as u32, 0..1);
            }
        }
        cx.queue.submit([encoder.finish()]);
        state.last = Some(self.clone());
        Ok(())
    }
    fn paint(
        &self,
        state: &State,
        element: &ElementInfo<'_>,
        paint: &mut PaintSession<'_, '_>,
    ) -> Result<(), UiError> {
        if let Some(renderer) = &state.renderer {
            let b = element.content_bounds;
            paint.draw_image(
                &renderer.image,
                TextureDraw::new(Rect::new(b.x, b.y, b.width, b.height)),
            )?;
        }
        Ok(())
    }
}
