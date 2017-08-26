use core::math::{Transform2D, Transform3D};
use vector_image_renderer::{ColorId, TransformId};
use std::mem;
use std::sync::Arc;
use std::ops::{Add, Sub};

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GpuAddress(pub u32);


impl GpuAddress {
    pub fn new(ty: GpuAddressType, offset: GpuOffset) -> Self {
        offset.assert_mask();
        let header = match ty {
            GpuAddressType::Global => GPU_ADDR_GLOBAL,
            GpuAddressType::Instance => GPU_ADDR_INSTANCE,
            GpuAddressType::Shared => GPU_ADDR_SHARED,
        };
        GpuAddress(offset.0 | header)
    }

    pub fn global(offset: GpuOffset) -> Self {
        offset.assert_mask();
        GpuAddress(offset.0)
    }

    pub fn instance(offset: GpuOffset) -> Self {
        offset.assert_mask();
        GpuAddress(offset.0 | GPU_ADDR_INSTANCE)
    }

    pub fn shared(offset: GpuOffset) -> Self {
        offset.assert_mask();
        GpuAddress(offset.0 | GPU_ADDR_SHARED)
    }

    pub fn from_raw(bytes: u32) -> Self {
        GpuAddress(bytes)
    }

    pub fn address_type(&self) -> GpuAddressType {
        match self.0 & GPU_ADDR_HEADER_MASK {
            GPU_ADDR_INSTANCE => GpuAddressType::Instance,
            GPU_ADDR_SHARED => GpuAddressType::Shared,
            GPU_ADDR_GLOBAL => GpuAddressType::Global,
            _ => { panic!("Invalid gpu address type header"); }
        }
    }

    pub fn offset(&self) -> GpuOffset { GpuOffset(self.0 & GPU_ADDR_OFFSET_MASK) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GpuOffset(pub u32);

impl GpuOffset {
    pub fn as_u32(&self) -> u32 { self.0 }

    pub fn assert_mask(&self) {
        assert!(self.as_u32() & GPU_ADDR_HEADER_MASK == 0);
    }
}

impl Add for GpuOffset {
    type Output = GpuOffset;
    fn add(self, other: GpuOffset) -> GpuOffset {
        GpuOffset(self.0 + other.0)
    }
}

impl Add<GpuOffset> for GpuAddress {
    type Output = GpuAddress;
    fn add(self, rhs: GpuOffset) -> GpuAddress {
        GpuAddress(self.0 + rhs.0)
    }
}

impl Sub for GpuAddress {
    type Output = GpuOffset;
    fn sub(self, rhs: GpuAddress) -> GpuOffset {
        GpuOffset(self.0 - rhs.0)
    }
}

pub const GPU_ADDR_HEADER_MASK: u32 = 0b11111111_00000000_00000000_00000000;
pub const GPU_ADDR_OFFSET_MASK: u32 = !GPU_ADDR_HEADER_MASK;
pub const GPU_ADDR_GLOBAL: u32      = 0b00000000_00000000_00000000_00000000;
pub const GPU_ADDR_INSTANCE: u32    = 0b10000000_00000000_00000000_00000000;
pub const GPU_ADDR_SHARED: u32      = 0b01000000_00000000_00000000_00000000;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GpuAddressType {
    Global,
    Instance,
    Shared,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GpuAddressRange {
    pub start: GpuAddress,
    pub end: GpuAddress,
}

impl GpuAddressRange {
    pub fn start(&self) -> GpuAddress { self.start }

    pub fn is_empty(&self) -> bool { self.start == self.end }

    pub fn shrink_left(&mut self, amount: u32) {
        assert!(self.end.offset().as_u32() - self.start.offset().as_u32() >= amount);
        self.start = self.start + GpuOffset(amount);
    }
}

#[derive(Clone, Debug)]
pub struct GpuData {
    buffer: Vec<GpuWord>,
}

impl GpuData {
    pub fn new() -> Self {
        GpuData {
            buffer: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        GpuData {
            buffer: Vec::with_capacity(cap),
        }
    }

    pub fn len(&self) -> usize { self.buffer.len() }

    pub fn as_slice(&self) -> &[GpuWord] { &self.buffer[..] }

    pub fn push<Block: GpuBlock>(&mut self, block: &Block) -> GpuOffset {
        debug_assert_eq!(block.slice().len() % 4, 0);
        let offset = GpuOffset(self.buffer.len() as u32);
        self.buffer.extend(block.slice().iter().cloned());
        return offset;
    }

    pub fn set<Block: GpuBlock>(&mut self, base_offset: GpuOffset, block: &Block) {
        let base = base_offset.0  as usize;
        for (offset, element) in block.slice().iter().cloned().enumerate() {
            self.buffer[base + offset] = element;
        }
    }
}

pub struct RuntimeType {
    pub name: String,
    pub layout: MemoryLayout,
}

impl RuntimeType {
    pub fn new(name: String, layout: MemoryLayout) -> Self {
        RuntimeType { layout, name }
    }
}

pub struct GpuStruct {
    pub data: GpuData,
    pub ty: Arc<RuntimeType>,
}

pub type MemberId = u32;

impl GpuStruct {
    pub fn set<T: GpuBlock>(&mut self, id: MemberId, val: &T) {
        let offset = self.ty.layout.offset_of(id);
        self.data.set(offset, val);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScalarType {
    I32,
    F32,
    Address,
    Unknown,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DataType {
    pub scalar: ScalarType,
    pub size: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub ty: DataType,
    pub offset: GpuOffset,
}

impl DataType {
    pub fn size(&self) -> u32 { self.size }

    pub fn alignment(&self) -> u32 {
        // TODO: maybe it would be simpler to just align everything on 4 words.
        match self.size() {
            1 => { 1 }
            2 => { 2 }
            3 => { 4 }
            n => { (n / 4) * 4 }
        }
    }

    pub fn float() -> Self { DataType { scalar: ScalarType::F32, size: 1 } }
    pub fn vec2() -> Self { DataType { scalar: ScalarType::F32, size: 2 } }
    pub fn vec3() -> Self { DataType { scalar: ScalarType::F32, size: 3 } }
    pub fn vec4() -> Self { DataType { scalar: ScalarType::F32, size: 4 } }

    pub fn transform_2d() -> Self { DataType { scalar: ScalarType::F32, size: 6 } }
    pub fn transform_3d() -> Self { DataType { scalar: ScalarType::F32, size: 16 } }

    pub fn int() -> Self { DataType { scalar: ScalarType::I32, size: 1 } }
    pub fn ivec2() -> Self { DataType { scalar: ScalarType::I32, size: 2 } }
    pub fn ivec3() -> Self { DataType { scalar: ScalarType::I32, size: 3 } }
    pub fn ivec4() -> Self { DataType { scalar: ScalarType::I32, size: 4 } }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryLayout {
    members: Vec<Member>,
    size: u32,
}

impl MemoryLayout {
    pub fn new() -> Self {
        MemoryLayout {
            members: Vec::new(),
            size: 0,
        }
    }

    pub fn alloc(&mut self, ty: DataType) -> GpuOffset {
        let alignment = ty.alignment();
        let adjust = match self.size % alignment {
            0 => 0,
            n => { alignment - n }
        };
        let addr = GpuOffset(self.size + adjust);
        self.members.push(Member { ty, offset: addr });

        self.size += adjust + ty.size();

        return addr;
    }

    pub fn members(&self) -> &[Member] {
        &self.members
    }

    pub fn offset_of(&self, member: MemberId) -> GpuOffset {
        self.members[member as usize].offset
    }
}

