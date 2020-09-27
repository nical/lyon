pub mod units;

use std::ops::Range;
pub use lyon_geom as geom;

use std::num::{NonZeroU16, NonZeroU32};

#[test]
fn size_of_id() {
    assert_eq!(std::mem::size_of::<Option<SomeId>>(), 4);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameStamp(pub usize);

impl FrameStamp {
    pub fn advance(&mut self) {
        self.0 += 1;
    }
}

pub type Generation = u8;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceIndex(pub u8, pub u8);

impl ResourceIndex {
    pub fn index(&self) -> usize {
        self.0 as usize
    }

    pub fn generation(&self) -> u8 {
        self.1
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferAllocatorId(pub u32);


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferKind(pub NonZeroU16);

impl BufferKind {
    pub fn buffer_id(&self, res: ResourceIndex) -> BufferId {
        BufferId {
            kind: *self,
            id: res,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferId {
    pub kind: BufferKind,
    pub id: ResourceIndex,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutId(pub NonZeroU16);

impl BindGroupLayoutId {
    pub fn bind_group_id(&self, index: ResourceIndex) -> BindGroupId {
        BindGroupId {
            kind: *self,
            index,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BindGroupId {
    pub kind: BindGroupLayoutId,
    pub index: ResourceIndex,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureKind(pub NonZeroU16);

impl TextureKind {
    pub fn texture_id(&self, res: ResourceIndex) -> TextureId {
        TextureId {
            kind: *self,
            id: res,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureId {
    pub kind: TextureKind,
    pub id: ResourceIndex,
}

impl BufferId {
    pub fn range(&self, range: Range<u32>) -> BufferRange {
        BufferRange {
            buffer: *self,
            range,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferRange {
    pub buffer: BufferId,
    // In bytes
    pub range: Range<u32>,
}

impl BufferRange {
    pub fn new(buffer: BufferId, range: Range<u32>) -> Self {
        BufferRange { buffer, range }
    }

    pub fn len(&self) -> u32 {
        self.range.end - self.range.start
    }

    pub fn start(&self) -> u32 {
        self.range.start
    }

    pub fn end(&self) -> u32 {
        self.range.end
    }

    pub fn range_of<T>(&self) -> Range<u32> {
        let sz = std::mem::size_of::<T>() as u32;
        debug_assert_eq!(self.range.start % sz, 0);
        (self.range.start / sz) .. (self.range.end / sz)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferOffset {
    pub buffer: BufferId,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BindGroupInput {
    Buffer(BufferId),
    Texture(TextureId),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BindGroupInputKind {
    Buffer(BufferKind),
    Texture(TextureKind),
}

pub type PipelineFeatures = u32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub kind: PipelineKind,
    pub features: PipelineFeatures,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKind(pub NonZeroU32);

impl PipelineKind {
    pub fn with_no_feature(&self) -> PipelineKey {
        self.with_features(0)
    }
 
    pub fn with_features(&self, features: PipelineFeatures) -> PipelineKey {
        PipelineKey {
            kind: *self,
            features,
        }
    }
}
