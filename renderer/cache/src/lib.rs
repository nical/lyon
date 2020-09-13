use glue::{ResourceId, BufferId, BufferKindId, TextureId, TextureKindId, BufferRange, FrameStamp};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Eviction {
    Auto,
    Manual,
}

pub struct AttributeVector<T> {
    data: Vec<T>,
}

impl<T: Default> AttributeVector<T> {
    pub fn new() -> Self {
        AttributeVector {
            data: Vec::new(),
        }
    }

    pub fn set(&mut self, id: ItemId, val: T) {
        let idx = id.0 as usize;
        while self.data.len() <= idx {
            self.data.push(T::default());
        }

        self.data[idx] = val;
    }

    pub fn get(&self, id: ItemId) -> &T {
        &self.data[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: ItemId) -> &T {
        &mut self.data[id.0 as usize]
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}


struct ResourceEntry<T> {
    payload: T,
    last_used: FrameStamp,
    allocated: bool,
} 

pub struct ResourceList<Entry> {
    entries: Vec<ResourceEntry<Entry>>,
    free_list: Vec<ResourceId>,
}

impl<Entry> ResourceList<Entry> {
    pub fn new() -> Self {
        ResourceList {
            entries: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn add(&mut self, now: FrameStamp, payload: Entry) -> ResourceId {
        let entry = ResourceEntry {
            payload,
            last_used: now,
            allocated: true,
        };

        if let Some(id) = self.free_list.pop() {
            self.entries[id.0 as usize] = entry;
            return id;
        }

        let id = ResourceId(self.entries.len() as u16);
        self.entries.push(entry);

        id
    }

    pub fn remove(&mut self, id: ResourceId) {
        debug_assert!(self.entries[id.0 as usize].allocated);
        self.entries[id.0 as usize].allocated = false;
        self.free_list.push(id);
    }

    pub fn get(&mut self, now: FrameStamp, id: ResourceId) -> &Entry {
        let entry = &mut self.entries[id.0 as usize];
        debug_assert!(entry.allocated);
        entry.last_used = now;
        &entry.payload
    }

    pub fn get_mut(&mut self, now: FrameStamp, id: ResourceId) -> &mut Entry {
        let entry = &mut self.entries[id.0 as usize];
        debug_assert!(entry.allocated);
        entry.last_used = now;
        &mut entry.payload
    }

    pub fn mark_used(&mut self, now: FrameStamp, id: ResourceId) {
        debug_assert!(self.entries[id.0 as usize].allocated);
        self.entries[id.0 as usize].last_used = now;
    }

    pub fn for_each<F>(&self, mut f: F) where F: FnMut(ResourceId, FrameStamp, &Entry) {
        for (idx, entry) in self.entries.iter().enumerate() {
            if entry.allocated {
                let id = ResourceId(idx as u16);
                f(id, entry.last_used, &entry.payload);
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.free_list.clear();
    }
}

pub struct FrameContext {
    pub now: FrameStamp,
    pub buffers_to_add: Vec<BufferId>,
    pub buffers_to_remove: Vec<BufferId>,
    pub textures_to_add: Vec<TextureId>,
    pub textures_to_remove: Vec<TextureId>,
}

impl FrameContext {
    pub fn new() -> Self {
        FrameContext {
            now: FrameStamp(1),
            buffers_to_add: Vec::new(),
            buffers_to_remove: Vec::new(),
            textures_to_add: Vec::new(),
            textures_to_remove: Vec::new(),
        }
    }

    pub fn begin_frame(&mut self) {
        self.now.0 += 1;
    }
}

pub struct BufferEntry {
    allocator: pipe::SegmentAllocator,
    eviction: Eviction,
}

pub struct BufferCache {
    res: ResourceList<BufferEntry>,
    id: BufferKindId,
    size: u32,
}

impl BufferCache {
    pub fn new(id: BufferKindId, size: u64) -> Self {
        BufferCache {
            res: ResourceList::new(),
            id,
            size: size as u32,
        }
    }

    pub fn add_buffer(&mut self, ctx: &mut FrameContext) -> BufferId {
        let res_id = self.res.add(ctx.now, BufferEntry {
            allocator: pipe::SegmentAllocator::new(self.size),
            eviction: Eviction::Auto,
        });

        let id = self.id.buffer_id(res_id);

        ctx.buffers_to_add.push(id);

        id
    }

    pub fn remove_buffer(&mut self, id: BufferId, ctx: &mut FrameContext) {
        debug_assert_eq!(self.id, id.kind);
        ctx.buffers_to_remove.push(id);
        self.res.remove(id.id);
    }

    pub fn clear(&mut self, ctx: &mut FrameContext) {
        self.res.for_each(|id, _, _| {
            ctx.buffers_to_remove.push(self.id.buffer_id(id));
        });
        self.res.clear();
    }

    pub fn mark_used(&mut self, id: BufferId, ctx: &mut FrameContext) {
        self.res.mark_used(ctx.now, id.id);
    }

    pub fn get_buffer_space(&mut self, size: u32, ctx: &mut FrameContext) -> BufferRange {
        for (index, buffer) in self.res.entries.iter_mut().enumerate() {
            if !buffer.allocated {
                continue;
            }
            match buffer.payload.allocator.allocate(size) {
                Ok(range) => {
                    buffer.last_used = ctx.now;
                    let id = self.id.buffer_id(ResourceId(index as u16));

                    return id.range(range);
                }
                Err(..) => {}
            }
        }

        let mut allocator = pipe::SegmentAllocator::new(self.size);
        let range = allocator.allocate(size).unwrap();

        let id = self.id.buffer_id(
            self.res.add(ctx.now, BufferEntry {
                allocator: allocator,
                eviction: Eviction::Auto,
            })
        );

        ctx.buffers_to_add.push(id);

        id.range(range)
    }
}


pub struct TextureEntry {
    allocator: Option<usize>,
    eviction: Eviction,
}

pub struct TextureCache {
    res: ResourceList<TextureEntry>,
    id: TextureKindId,
}

impl TextureCache {
    pub fn new(id: TextureKindId) -> Self {
        TextureCache {
            res: ResourceList::new(),
            id,
        }
    }

    pub fn add_texture(&mut self, ctx: &mut FrameContext) -> TextureId {
        let res_id = self.res.add(ctx.now, TextureEntry {
            allocator: None,
            eviction: Eviction::Auto,
        });

        let id = self.id.texture_id(res_id);

        ctx.textures_to_add.push(id);

        id
    }

    pub fn remove_texture(&mut self, id: TextureId, ctx: &mut FrameContext) {
        debug_assert_eq!(self.id, id.kind);
        ctx.textures_to_remove.push(id);
        self.res.remove(id.id);
    }

    pub fn clear(&mut self, ctx: &mut FrameContext) {
        self.res.for_each(|id, _, _| {
            ctx.textures_to_remove.push(self.id.texture_id(id));
        });
        self.res.clear();
    }

    pub fn mark_used(&mut self, id: TextureId, ctx: &mut FrameContext) {
        self.res.mark_used(ctx.now, id.id);
    }
}

