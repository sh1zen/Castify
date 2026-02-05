use iced_wgpu::primitive::Primitive;
use iced_wgpu::wgpu;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use super::video::FrameBuffer;

#[repr(C)]
struct Uniforms {
    rect: [f32; 4],
}

pub struct VideoPipeline {
    pipeline: wgpu::RenderPipeline,
    bg0_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Maps video_id → (Y texture, U texture, V texture, uniform buffer, bind group)
    textures: BTreeMap<
        u64,
        (
            wgpu::Texture,
            wgpu::Texture,
            wgpu::Texture,
            wgpu::Buffer,
            wgpu::BindGroup,
        ),
    >,
}

impl VideoPipeline {
    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        video_id: u64,
        (width, height): (u32, u32),
        frame: &[u8],
    ) {
        let uw = width / 2;
        let uh = height / 2;
        let y_size = (width * height) as usize;
        let uv_size = (uw * uh) as usize;

        // Validate frame size
        if frame.len() < y_size + uv_size * 2 {
            log::warn!(
                "VideoPipeline::upload() - frame too small: {} < {} ({}x{})",
                frame.len(),
                y_size + uv_size * 2,
                width,
                height
            );
            return;
        }

        // Check if textures need (re)creation due to resolution change or first frame
        let needs_recreate = match self.textures.get(&video_id) {
            None => {
                log::info!(
                    "VideoPipeline::upload() - first frame, creating textures {}x{}",
                    width,
                    height
                );
                true
            }
            Some((y_tex, _, _, _, _)) => {
                let cur = y_tex.size();
                if cur.width != width || cur.height != height {
                    log::info!(
                        "VideoPipeline::upload() - resolution changed from {}x{} to {}x{}",
                        cur.width,
                        cur.height,
                        width,
                        height
                    );
                    true
                } else {
                    false
                }
            }
        };

        if needs_recreate {
            // Remove old entry if exists
            self.textures.remove(&video_id);

            let y_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("video Y texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let u_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("video U texture"),
                size: wgpu::Extent3d {
                    width: uw,
                    height: uh,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let v_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("video V texture"),
                size: wgpu::Extent3d {
                    width: uw,
                    height: uh,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let y_view = y_tex.create_view(&wgpu::TextureViewDescriptor::default());
            let u_view = u_tex.create_view(&wgpu::TextureViewDescriptor::default());
            let v_view = v_tex.create_view(&wgpu::TextureViewDescriptor::default());

            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("video uniform buffer"),
                size: std::mem::size_of::<Uniforms>() as _,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("video bind group"),
                layout: &self.bg0_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&y_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&u_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&v_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
            });

            self.textures
                .insert(video_id, (y_tex, u_tex, v_tex, buffer, bind_group));
        }

        let (y_tex, u_tex, v_tex, _, _) = self.textures.get(&video_id).unwrap();

        let y_data = &frame[..y_size];
        let u_data = &frame[y_size..y_size + uv_size];
        let v_data = &frame[y_size + uv_size..];

        // Upload Y plane
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: y_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            y_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // Upload U plane
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: u_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            u_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(uw),
                rows_per_image: Some(uh),
            },
            wgpu::Extent3d {
                width: uw,
                height: uh,
                depth_or_array_layers: 1,
            },
        );

        // Upload V plane
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: v_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            v_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(uw),
                rows_per_image: Some(uh),
            },
            wgpu::Extent3d {
                width: uw,
                height: uh,
                depth_or_array_layers: 1,
            },
        );
    }

    fn prepare_uniforms(&mut self, queue: &wgpu::Queue, video_id: u64, bounds: &iced::Rectangle) {
        if let Some((_, _, _, buffer, _)) = self.textures.get(&video_id) {
            let uniforms = Uniforms {
                rect: [
                    bounds.x,
                    bounds.y,
                    bounds.x + bounds.width,
                    bounds.y + bounds.height,
                ],
            };
            queue.write_buffer(buffer, 0, unsafe {
                std::slice::from_raw_parts(
                    &uniforms as *const _ as *const u8,
                    size_of::<Uniforms>(),
                )
            });
        }
    }

    fn draw(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: &iced::Rectangle<u32>,
        video_id: u64,
    ) {
        if let Some((_, _, _, _, bind_group)) = self.textures.get(&video_id) {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("video render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_viewport(
                viewport.x as _,
                viewport.y as _,
                viewport.width as _,
                viewport.height as _,
                0.0,
                1.0,
            );
            pass.draw(0..4, 0..1);
        }
    }
}

impl iced_wgpu::primitive::Pipeline for VideoPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("video shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let bg0_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("video bind group 0 layout"),
            entries: &[
                // binding 0: Y texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 1: U texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 2: V texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 3: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 4: uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("video pipeline layout"),
            bind_group_layouts: &[&bg0_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("video pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("video sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 1.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        VideoPipeline {
            pipeline,
            bg0_layout,
            sampler,
            textures: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoPrimitive {
    video_id: u64,
    frame: Arc<Mutex<FrameBuffer>>,
    has_new_frame: Arc<AtomicBool>,
}

impl VideoPrimitive {
    pub fn new(
        video_id: u64,
        frame: Arc<Mutex<FrameBuffer>>,
        _size: (u32, u32),
        has_new_frame: Arc<AtomicBool>,
    ) -> Self {
        VideoPrimitive {
            video_id,
            frame,
            has_new_frame,
        }
    }
}

impl Primitive for VideoPrimitive {
    type Pipeline = VideoPipeline;

    fn prepare(
        &self,
        pipeline: &mut VideoPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &iced::Rectangle,
        _viewport: &iced_wgpu::graphics::Viewport,
    ) {
        // Upload only when a new frame is available.
        // Keep this non-blocking and re-arm the flag if lock contention occurs.
        let should_upload = self.has_new_frame.swap(false, Ordering::AcqRel);
        if should_upload {
            if let Ok(mut buffer) = self.frame.try_lock()
                && let Some((frame_data, w, h)) = buffer.read()
            {
                pipeline.upload(
                    device,
                    queue,
                    self.video_id,
                    (w as u32, h as u32),
                    frame_data,
                );
            } else {
                // Consumed a fresh-frame signal but couldn't upload yet.
                // Re-arm it so the next prepare retries.
                self.has_new_frame.store(true, Ordering::Release);
            }
        }

        pipeline.prepare_uniforms(queue, self.video_id, bounds);
    }

    fn render(
        &self,
        pipeline: &VideoPipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        pipeline.draw(target, encoder, clip_bounds, self.video_id);
    }
}
