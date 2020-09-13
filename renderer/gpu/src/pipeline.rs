use crate::GpuImageSource;
use glue::units::*;

pub const DEFAULT_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Alpha,
    Add,
    None
}

impl BlendMode {
    pub fn color_descriptor(&self) -> wgpu::BlendDescriptor {
        match *self {
            BlendMode::None => wgpu::BlendDescriptor::REPLACE,
            BlendMode::Alpha => {
                wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                }
            },
            BlendMode::Add => {
                wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::DstAlpha,
                    operation: wgpu::BlendOperation::Add,
                }
            },
        }
    }
    pub fn alpha_descriptor(&self) -> wgpu::BlendDescriptor {
        match *self {
            BlendMode::None => wgpu::BlendDescriptor::REPLACE,
            BlendMode::Alpha
            | BlendMode::Add => {
                wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                }
            },
        }
    }
}

pub struct DepthBuffer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl DepthBuffer {
    pub fn new(width: u32, height: u32, device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            mip_level_count: 1,
            sample_count: 1,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        DepthBuffer {
            texture,
            view,
            width,
            height,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderTargetState {
    pub depth_stencil: Option<wgpu::DepthStencilStateDescriptor>,
    pub color: wgpu::ColorStateDescriptor,
}

impl RenderTargetState {
    pub fn new() -> Self {
        RenderTargetState {
            depth_stencil: None,
            color: wgpu::ColorStateDescriptor {
                format: DEFAULT_COLOR_FORMAT,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }
        }
    }

    pub fn opaque_pass() -> Self {
        RenderTargetState {
            depth_stencil: None,
            color: wgpu::ColorStateDescriptor {
                format: DEFAULT_COLOR_FORMAT,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }
        }
    }

    pub fn opaque_pass_with_depth_test() -> Self {
        RenderTargetState {
            depth_stencil: Some(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: wgpu::StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
            }),
            color: wgpu::ColorStateDescriptor { 
                format: DEFAULT_COLOR_FORMAT,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }
        }
    }

    pub fn blend_pass(mode: BlendMode) -> Self {
        RenderTargetState {
            depth_stencil: None,
            color: wgpu::ColorStateDescriptor {
                format: DEFAULT_COLOR_FORMAT,
                color_blend: mode.color_descriptor(),
                alpha_blend: mode.alpha_descriptor(),
                write_mask: wgpu::ColorWrite::ALL,
            }
        }
    }

    pub fn blend_pass_with_depth_test(mode: BlendMode) -> Self {
        RenderTargetState {
            depth_stencil: Some(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: wgpu::StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
            }),
            color: wgpu::ColorStateDescriptor {
                format: DEFAULT_COLOR_FORMAT,
                color_blend: mode.color_descriptor(),
                alpha_blend: mode.alpha_descriptor(),
                write_mask: wgpu::ColorWrite::ALL,
            }
        }
    }

    pub fn float_target(mode: BlendMode) -> Self {
        RenderTargetState {
            depth_stencil: None,
            color: wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::R32Float,
                color_blend: mode.color_descriptor(),
                alpha_blend: mode.alpha_descriptor(),
                write_mask: wgpu::ColorWrite::ALL,
            }
        }        
    }

    pub fn with_depth_stencil(mut self, state: wgpu::DepthStencilStateDescriptor) -> Self {
        self.depth_stencil = Some(state);
        self
    }

    pub fn disable_depth_stencil(mut self) -> Self {
        self.depth_stencil = None;
        self
    }

}

pub struct SourceTexture {
    pub size: wgpu::Extent3d,
    pub format: wgpu::TextureFormat,
}

impl SourceTexture {
    pub fn pixel_src(&self, p: DeviceIntPoint) -> GpuImageSource {
        let w = self.size.width as f32;
        let h = self.size.height as f32;
        let p = p.to_f32();
        // offset by half a pixel to hit texel center
        let x = (p.x + 0.5) / w;
        let y = (p.y + 0.5) / h;
        GpuImageSource {
            rect: [x, y, x, y],
            parameters: [0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn sub_image_src(&self, r: &DeviceIntBox) -> GpuImageSource {
        let w = self.size.width as f32;
        let h = self.size.height as f32;
        let r = r.to_f32();
        GpuImageSource {
            rect: [r.min.x / w, r.min.y /h, r.max.x / w, r.max.y /h],
            parameters: [0.0, 0.0, 0.0, 0.0],
        }
    }
}
