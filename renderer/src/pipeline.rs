#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Alpha,
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
        }
    }
    pub fn alpha_descriptor(&self) -> wgpu::BlendDescriptor {
        match *self {
            BlendMode::None => wgpu::BlendDescriptor::REPLACE,
            BlendMode::Alpha => {
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
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_size: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::D32Float,
            usage: wgpu::TextureUsageFlags::OUTPUT_ATTACHMENT,
        });

        let view = texture.create_default_view();

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
                format: wgpu::TextureFormat::Bgra8Unorm,
                color: wgpu::BlendDescriptor::REPLACE,
                alpha: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWriteFlags::ALL,
            }
        }
    }

    pub fn opaque_pass() -> Self {
        RenderTargetState {
            depth_stencil: Some(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::D32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            }),
            color: wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8Unorm,
                color: wgpu::BlendDescriptor::REPLACE,
                alpha: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWriteFlags::ALL,
            }
        }
    }

    pub fn blend_pass(mode: BlendMode) -> Self {
        RenderTargetState {
            depth_stencil: None,
            color: wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8Unorm,
                color: mode.color_descriptor(),
                alpha: mode.alpha_descriptor(),
                write_mask: wgpu::ColorWriteFlags::ALL,
            }
        }
    }

    pub fn blend_pass_with_depth_test(mode: BlendMode) -> Self {
        RenderTargetState {
            depth_stencil: Some(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::D32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            }),
            color: wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8Unorm,
                color: mode.color_descriptor(),
                alpha: mode.alpha_descriptor(),
                write_mask: wgpu::ColorWriteFlags::ALL,
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
