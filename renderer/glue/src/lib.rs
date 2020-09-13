pub mod units;

use std::ops::Range;
pub use lyon_geom as geom;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameStamp(pub usize);

impl FrameStamp {
    pub fn advance(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u16);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferAllocatorId(pub u32);


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferKindId(pub u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferId {
    pub kind: BufferKindId,
    pub id: ResourceId,
}

impl BufferKindId {
    pub fn buffer_id(&self, res: ResourceId) -> BufferId {
        BufferId {
            kind: *self,
            id: res,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutId(pub ResourceId);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BindGroupId(pub ResourceId);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureKindId(pub u16);

impl TextureKindId {
    pub fn texture_id(&self, res: ResourceId) -> TextureId {
        TextureId {
            kind: *self,
            id: res,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureId {
    pub kind: TextureKindId,
    pub id: ResourceId,
}

impl BufferId {
    pub fn range(&self, range: Range<u32>) -> BufferRange {
        BufferRange {
            buffer: *self,
            range,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
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

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct BufferOffset {
    pub buffer: BufferId,
    pub offset: u32,
}

