use std::collections::HashMap;
use glue::*;

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
    payload: Option<T>,
    last_used: FrameStamp,
    gen: u8,
} 

pub struct ResourceList<Entry> {
    entries: Vec<ResourceEntry<Entry>>,
    free_list: Vec<ResourceIndex>,
}

impl<Entry> ResourceList<Entry> {
    pub fn new() -> Self {
        ResourceList {
            entries: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn add(&mut self, now: FrameStamp, payload: Entry) -> ResourceIndex {
        if let Some(mut id) = self.free_list.pop() {
            let gen = self.entries[id.index()].gen.overflowing_add(1).0;
            id.1 = gen;
            self.entries[id.index()] = ResourceEntry {
                payload: Some(payload),
                last_used: now,
                gen,
            };

            return id;
        }

        let id = ResourceIndex(self.entries.len() as u8, 0);
        self.entries.push(ResourceEntry {
            payload: Some(payload),
            last_used: now,
            gen: 0,
        });

        id
    }

    pub fn remove(&mut self, id: ResourceIndex) {
        debug_assert!(self.entries[id.index()].payload.is_some());
        assert_eq!(self.entries[id.index()].gen, id.generation());
        self.entries[id.index()].payload = None;
        self.free_list.push(id);
    }

    pub fn get(&mut self, now: FrameStamp, id: ResourceIndex) -> &Entry {
        let entry = &mut self.entries[id.index()];
        assert_eq!(entry.gen, id.generation());
        entry.last_used = now;
        entry.payload.as_ref().unwrap()
    }

    pub fn get_mut(&mut self, now: FrameStamp, id: ResourceIndex) -> &mut Entry {
        let entry = &mut self.entries[id.index()];
        assert_eq!(entry.gen, id.generation());
        entry.last_used = now;
        entry.payload.as_mut().unwrap()
    }

    pub fn mark_used(&mut self, now: FrameStamp, id: ResourceIndex) {
        let entry = &mut self.entries[id.index()];
        assert_eq!(entry.gen, id.generation());
        entry.last_used = now;
    }

    pub fn last_used(&self, id: ResourceIndex) -> FrameStamp {
        let entry = &self.entries[id.index()];
        assert_eq!(entry.gen, id.generation());

        entry.last_used
    }

    pub fn contains(&self, id: ResourceIndex) -> bool {
        let entry = &self.entries[id.index()];

        entry.gen == id.generation() && entry.payload.is_some()
    }

    pub fn for_each<F>(&self, mut f: F) where F: FnMut(ResourceIndex, FrameStamp, &Entry) {
        for (idx, entry) in self.entries.iter().enumerate() {
            if let Some(payload) = &entry.payload {
                let id = ResourceIndex(idx as u8, entry.gen);
                f(id, entry.last_used, payload);
            }
        }
    }


    pub fn gc(&mut self, threshold: FrameStamp, mut cb: impl FnMut(ResourceIndex)) {
        for (idx, entry) in self.entries.iter_mut().enumerate() {
            if entry.payload.is_some() {
                if entry.last_used.0 < threshold.0 {
                    let id = ResourceIndex(idx as u8, entry.gen);
                    cb(id);
                    entry.payload = None;
                    self.free_list.push(id);
                }
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
    pub added_buffers: Vec<BufferId>,
    pub removed_buffers: Vec<BufferId>,
    pub added_textures: Vec<TextureId>,
    pub removed_textures: Vec<TextureId>,
    pub added_bind_groups: Vec<(BindGroupId, Vec<BindGroupInput>)>,
    pub removed_bind_groups: Vec<BindGroupId>,
}

impl FrameContext {
    pub fn new() -> Self {
        FrameContext {
            now: FrameStamp(1),
            added_buffers: Vec::new(),
            removed_buffers: Vec::new(),
            added_textures: Vec::new(),
            removed_textures: Vec::new(),
            added_bind_groups: Vec::new(),
            removed_bind_groups: Vec::new(),
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
    id: BufferKind,
    size: u32,
}

impl BufferCache {
    pub fn new(id: BufferKind, size: u64) -> Self {
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

        ctx.added_buffers.push(id);

        id
    }

    pub fn remove_buffer(&mut self, id: BufferId, ctx: &mut FrameContext) {
        debug_assert_eq!(self.id, id.kind);
        ctx.removed_buffers.push(id);
        self.res.remove(id.id);
    }

    pub fn clear(&mut self, ctx: &mut FrameContext) {
        self.res.for_each(|id, _, _| {
            ctx.removed_buffers.push(self.id.buffer_id(id));
        });
        self.res.clear();
    }

    pub fn mark_used(&mut self, id: BufferId, ctx: &mut FrameContext) {
        self.res.mark_used(ctx.now, id.id);
    }

    pub fn get_buffer_space(&mut self, size: u32, ctx: &mut FrameContext) -> BufferRange {
        for (index, entry) in self.res.entries.iter_mut().enumerate() {
            if let Some(buffer) = &mut entry.payload {
                match buffer.allocator.allocate(size) {
                    Ok(range) => {
                        entry.last_used = ctx.now;
                        let id = self.id.buffer_id(ResourceIndex(index as u8, entry.gen));

                        return id.range(range);
                    }
                    Err(..) => {}
                }
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

        ctx.added_buffers.push(id);

        id.range(range)
    }
}


pub struct TextureEntry {
    allocator: Option<usize>,
    eviction: Eviction,
}

pub struct TextureCache {
    res: ResourceList<TextureEntry>,
    id: TextureKind,
}

impl TextureCache {
    pub fn new(id: TextureKind) -> Self {
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

        ctx.added_textures.push(id);

        id
    }

    pub fn remove_texture(&mut self, id: TextureId, ctx: &mut FrameContext) {
        debug_assert_eq!(self.id, id.kind);
        ctx.removed_textures.push(id);
        self.res.remove(id.id);
    }

    pub fn clear(&mut self, ctx: &mut FrameContext) {
        self.res.for_each(|id, _, _| {
            ctx.removed_textures.push(self.id.texture_id(id));
        });
        self.res.clear();
    }

    pub fn mark_used(&mut self, id: TextureId, ctx: &mut FrameContext) {
        self.res.mark_used(ctx.now, id.id);
    }
}

pub struct BindGroupCache {
    lookup: HashMap<Vec<BindGroupInput>, ResourceIndex>,
    bind_groups: ResourceList<()>,
    kind: BindGroupLayoutId,
}


impl BindGroupCache {
    pub fn new(kind: BindGroupLayoutId) -> Self {
        BindGroupCache {
            lookup: HashMap::new(),
            bind_groups: ResourceList::new(),
            kind,
        }
    }

    pub fn get_bind_group(&mut self, inputs: &[BindGroupInput], ctx: &mut FrameContext) -> Option<BindGroupId> {
        if let Some(&index) = self.lookup.get(inputs) {
            self.bind_groups.mark_used(ctx.now, index);

            return Some(BindGroupId {
                kind: self.kind,
                index,
            });
        }

        None
    }

    pub fn create_bind_group(&mut self, inputs: &[BindGroupInput], ctx: &mut FrameContext) -> BindGroupId {
        debug_assert!(!self.lookup.contains_key(inputs));

        let index = self.bind_groups.add(ctx.now, ());

        let mut key = Vec::new();
        key.extend_from_slice(inputs);

        self.lookup.insert(key.clone(), index);

        let id = self.kind.bind_group_id(index);

        ctx.added_bind_groups.push((id, key));

        id
    }

    pub fn get_or_create_bind_group(&mut self, inputs: &[BindGroupInput], ctx: &mut FrameContext) -> BindGroupId {
        if let Some(id) = self.get_bind_group(inputs, ctx) {
            return id;
        }

        self.create_bind_group(inputs, ctx)
    }

    pub fn try_reuse_bind_group(&mut self, id: BindGroupId, ctx: &mut FrameContext) -> bool {
        if !self.bind_groups.contains(id.index) {
            return false;
        }

        self.bind_groups.mark_used(ctx.now, id.index);

        true
    }

    pub fn gc(&mut self, threshold: FrameStamp, ctx: &mut FrameContext) {
        let bind_groups = &mut self.bind_groups;
        let kind = self.kind;
        self.lookup.retain(|_, index| {
            if bind_groups.last_used(*index).0 >= threshold.0 {
                return true;
            }

            bind_groups.remove(*index);

            ctx.removed_bind_groups.push(kind.bind_group_id(*index));

            false
        });
    }
}
