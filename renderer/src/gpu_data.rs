use core::math::{Transform2D, Transform3D};
use vector_image_renderer::{ColorId, TransformId};
use std::mem;

pub type GpuWord = f32;

pub trait GpuBlock {
    fn slice(&self) -> &[GpuWord];
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuBlock4([f32; 4]);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuBlock8([f32; 8]);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuBlock16([f32; 16]);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GpuBlock32([f32; 32]);

impl GpuBlock for GpuBlock4 { fn slice(&self) -> &[GpuWord] { &self.0[..] } }
impl GpuBlock for GpuBlock8 { fn slice(&self) -> &[GpuWord] { &self.0[..] } }
impl GpuBlock for GpuBlock16 { fn slice(&self) -> &[GpuWord] { &self.0[..] } }
impl GpuBlock for GpuBlock32 { fn slice(&self) -> &[GpuWord] { &self.0[..] } }

macro_rules! default_uninitialized {
    ($Type:ident) => (
        impl Default for $Type {
            fn default() -> Self {
                $Type(unsafe { mem::uninitialized() })
            }
        }
    )
}

default_uninitialized!(GpuBlock4);
default_uninitialized!(GpuBlock8);
default_uninitialized!(GpuBlock16);
default_uninitialized!(GpuBlock32);

macro_rules! gpu_data_impl {
    (
        struct $Type:ident : $BlockType:ident {
            $($field_name:ident : $FieldType:ty,)*
        }
    ) => (
        impl Copy for $Type {}
        impl Clone for $Type { fn clone(&self) -> Self { *self } }

        impl $Type {
            pub fn new($($field_name: $FieldType,)*) -> Self {
                $Type {
                    $($field_name: $field_name,)*
                    .. unsafe { mem::uninitialized() }
                }
            }
        }

        impl Into<$BlockType> for $Type {
            fn into(self) -> $BlockType {
                unsafe { mem::transmute(self) }
            }
        }

        impl From<$BlockType> for $Type {
            fn from(block: $BlockType) -> $Type {
                unsafe { mem::transmute(block) }
            }
        }

        impl GpuBlock for $Type {
            fn slice(&self) -> &[GpuWord] {
                let block: &$BlockType = unsafe { mem::transmute(self) };
                return block.slice();
            }
        }
    )
}

#[macro_export]
macro_rules! gpu_data {
    (
        #[padding($padding:expr)]
        struct $Type:ident : $BlockType:ident {
            $($field_name:ident : $FieldType:ty,)*
        }
    ) => (
        #[repr(C)]
        #[derive(Debug)]
        pub struct $Type {
            $(pub $field_name: $FieldType,)*
            _padding: [u32; $padding],
        }

        gpu_data_impl! {
            struct $Type : $BlockType {
                $($field_name: $FieldType,)*
            }
        }
    );

    (
        struct $Type:ident : $BlockType:ident {
            $($field_name:ident : $FieldType:ty,)*
        }
    ) => (
        #[repr(C)]
        #[derive(Debug)]
        pub struct $Type {
            $(pub $field_name: $FieldType,)*
        }

        gpu_data_impl! {
            struct $Type : $BlockType {
                $($field_name: $FieldType,)*
            }
        }
    )
}

gpu_data! {
    struct GpuColorF: GpuBlock4 {
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    }
}

gpu_data! {
    struct GpuRect: GpuBlock4 {
        top_left: [f32; 2],
        bottom_right: [f32; 2],
    }
}

gpu_data! {
    #[padding(2)]
    struct GpuTransform2D: GpuBlock8 {
        matrix: Transform2D,
    }
}

gpu_data! {
    struct GpuTransform3D: GpuBlock16 {
        matrix: Transform3D,
    }
}

gpu_data! {
    #[padding(4)]
    struct FillPrimitive: GpuBlock8 {
        //texture_rect: GpuRect,
        z_index: u32,
        color: ColorId,
        transforms: [TransformId; 2],
    }
}

gpu_data! {
    #[padding(3)]
    struct StrokePrimitive: GpuBlock8 {
        z_index: u32,
        width: f32,
        color: ColorId,
        transforms: [TransformId; 2],
    }
}
