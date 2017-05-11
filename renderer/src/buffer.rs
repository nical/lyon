use std;
use std::fmt;
use std::cmp;
use std::hash;
use std::marker::PhantomData;
use std::ops;

pub struct Id<T> {
    handle: u16,
    _marker: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new(handle: u16) -> Self {
        Id {
            handle: handle,
            _marker: PhantomData,
        }
    }
    pub fn index(&self) -> usize { self.handle as usize }
    pub fn from_index(handle: usize) -> Self { Id::new(handle as u16) }
    pub fn to_i32(&self) -> i32 { self.handle as i32 }
    pub fn to_u16(&self) -> u16 { self.handle }
    pub fn as_range(&self) -> IdRange<T> { IdRange::new(self.handle..self.handle + 1) }
}

impl<T> Copy for Id<T> {}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> ::std::cmp::Eq for Id<T> {}
impl<T> ::std::cmp::PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.handle == other.handle }
    fn ne(&self, other: &Self) -> bool { self.handle != other.handle }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "#{}", self.handle) }
}

impl<T> hash::Hash for Id<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) { self.handle.hash(state); }
}

pub struct IdRange<T> {
    start: u16,
    end: u16,
    _marker: PhantomData<T>,
}

impl<T> IdRange<T> {
    #[inline]
    pub fn new(range: ops::Range<u16>) -> Self {
        IdRange {
            start: range.start,
            end: range.end,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn empty() -> Self { IdRange::new(0..0) }

    #[inline]
    pub fn start(&self) -> Id<T> { Id::new(self.start) }

    #[inline]
    pub fn start_index(&self) -> usize { self.start as usize }

    #[inline]
    pub fn end_index(&self) -> usize { self.end as usize }

    #[inline]
    pub fn usize_range(&self) -> ops::Range<usize> { self.start_index()..self.end_index() }

    #[inline]
    pub fn u16_range(&self) -> ops::Range<u16> { self.start..self.end }

    #[inline]
    pub fn from_indices(indices: ops::Range<usize>) -> Self {
        IdRange::new(indices.start as u16..indices.end as u16)
    }

    #[inline]
    pub fn from_start_count(start: u16, count: u16) -> Self { IdRange::new(start..(start + count)) }

    #[inline]
    pub fn count(&self) -> u16 { self.end - self.start }

    #[inline]
    pub fn get(&self, n: u16) -> Id<T> {
        assert!(n < (self.end - self.start), "Shape id out of range.");
        Id::new(self.start + n)
    }

    #[inline]
    pub fn is_empty(&self) -> bool { self.start == self.end }

    #[inline]
    pub fn contains(&self, id: Id<T>) -> bool { id.handle >= self.start && id.handle < self.end }

    #[inline]
    pub fn intersection(&self, other: Self) -> Self {
        let start = cmp::max(self.start, other.start);
        let end = cmp::min(self.end, other.end);
        if end < start {
            return IdRange::empty();
        }
        return IdRange::new(start..end);
    }

    #[inline]
    pub fn including_id(&self, id: Id<T>) -> Self {
        if id.handle < self.start {
            return IdRange::new(id.handle..(self.count() + self.start - id.handle));
        }

        if id.handle >= self.end {
            return IdRange::new(self.start..(id.handle - self.start + 1));
        }

        return *self;
    }
}

impl<T> Copy for IdRange<T> {}

impl<T> Clone for IdRange<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> ::std::cmp::Eq for IdRange<T> {}
impl<T> ::std::cmp::PartialEq for IdRange<T> {
    fn eq(&self, other: &Self) -> bool { self.start == other.start && self.end == other.end }
    fn ne(&self, other: &Self) -> bool { self.start != other.start || self.end != other.end }
}

impl<T> fmt::Debug for IdRange<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IdRange({}..{})", self.start, self.end)
    }
}

pub struct BufferId<T> {
    handle: u32,
    _marker: PhantomData<T>,
}
impl<T> BufferId<T> {
    pub fn new(handle: u32) -> Self {
        BufferId {
            handle: handle,
            _marker: PhantomData,
        }
    }
    pub fn index(&self) -> usize { self.handle as usize }
    pub fn to_i32(&self) -> i32 { self.handle as i32 }
    pub fn to_u32(&self) -> u32 { self.handle }
}

impl<T> Copy for BufferId<T> {}

impl<T> Clone for BufferId<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> ::std::cmp::Eq for BufferId<T> {}
impl<T> ::std::cmp::PartialEq for BufferId<T> {
    fn eq(&self, other: &Self) -> bool { self.handle == other.handle }
    fn ne(&self, other: &Self) -> bool { self.handle != other.handle }
}

impl<T> fmt::Debug for BufferId<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Buffer#{}", self.handle) }
}

impl<T> hash::Hash for BufferId<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) { self.handle.hash(state); }
}

impl<T> Copy for BufferRange<T> {}
impl<T> Clone for BufferRange<T> {
    fn clone(&self) -> Self { *self }
}
#[derive(Debug, PartialEq, Eq)]
pub struct BufferRange<T> {
    pub buffer: BufferId<T>,
    pub range: IdRange<T>,
}

impl<T> BufferRange<T> {
    pub fn get(&self, nth: u16) -> BufferElement<T> {
        BufferElement {
            buffer: self.buffer,
            element: self.range.get(nth),
        }
    }

    pub fn first(&self) -> BufferElement<T> { self.get(0) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BufferElement<T> {
    pub buffer: BufferId<T>,
    pub element: Id<T>,
}

pub struct CpuBuffer<T> {
    data: Vec<T>,
    allocator: SimpleBufferAllocator,
    dirty: bool, // TODO: Track dirty ranges
}

impl<T: Default + Copy> CpuBuffer<T> {
    pub fn new(size: u16) -> Self {
        CpuBuffer {
            data: vec![Default::default(); size as usize],
            allocator: SimpleBufferAllocator::new(size),
            dirty: true,
        }
    }

    pub fn try_alloc(&mut self) -> Option<Id<T>> { self.allocator.alloc().map(|idx| Id::new(idx)) }

    pub fn alloc(&mut self) -> Id<T> { self.try_alloc().unwrap() }

    pub fn alloc_back(&mut self) -> Id<T> { self.try_alloc_back().unwrap() }

    pub fn push<U: Copy + Into<T>>(&mut self, val: U) -> Id<T> {
        let id = self.alloc();
        self[id] = val.into();
        return id;
    }

    pub fn push_range<U: Copy + Into<T>>(&mut self, values: &[U]) -> IdRange<T> {
        let id_range = self.alloc_range(values.len() as u16);
        for i in 0..values.len() {
            self[id_range.get(i as u16)] = values[i].into();
        }
        return id_range;
    }

    pub fn try_alloc_range(&mut self, count: u16) -> Option<IdRange<T>> {
        self.allocator.alloc_range(count).map(|range| IdRange::new(range.0..range.0 + range.1))
    }

    pub fn alloc_range(&mut self, count: u16) -> IdRange<T> { self.try_alloc_range(count).unwrap() }

    pub fn try_alloc_back(&mut self) -> Option<Id<T>> {
        self.allocator.alloc_back().map(|idx| Id::new(idx))
    }

    pub fn try_alloc_range_back(&mut self, count: u16) -> Option<IdRange<T>> {
        self.allocator
            .alloc_range_back(count)
            .map(|range| IdRange::new(range.0..range.0 + range.1))
    }

    pub fn alloc_range_back(&mut self, count: u16) -> IdRange<T> {
        self.try_alloc_range_back(count).unwrap()
    }

    pub fn push_back<U: Into<T>>(&mut self, val: U) -> Id<T> {
        let id = self.alloc_back();
        self[id] = val.into();
        return id;
    }

    pub fn as_slice(&self) -> &[T] { &self.data[..] }

    pub fn as_mut_slice(&mut self) -> &mut [T] { &mut self.data[..] }

    pub fn set_range<U: Copy + Into<T>>(&mut self, range: IdRange<T>, values: &[U]) {
        debug_assert!(range.count() as usize == values.len());
        for i in 0..range.count() {
            self[range.get(i)] = values[i as usize].into();
        }
    }

    pub fn len(&self) -> usize { self.data.len() }

    pub fn range(&self) -> IdRange<T> { IdRange::new(0..self.len() as u16) }

    pub fn sub_slice(&self, range: IdRange<T>) -> &[T] {
        let range = self.range().intersection(range);
        return &self.data[range.start_index()..(range.end as usize)];
    }

    pub fn flush_dirty_range(&mut self) -> IdRange<T> {
        if self.dirty {
            self.dirty = false;
            return self.range();
        }
        return IdRange::empty();
    }
}

impl<T> std::ops::Index<Id<T>> for CpuBuffer<T> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &T { &self.data[id.index()] }
}

impl<T> std::ops::IndexMut<Id<T>> for CpuBuffer<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut T { &mut self.data[id.index()] }
}

impl<T: Copy + Default> std::ops::Index<IdRange<T>> for CpuBuffer<T> {
    type Output = [T];
    fn index(&self, ids: IdRange<T>) -> &[T] { &self.data[ids.usize_range()] }
}

impl<T: Copy + Default> std::ops::IndexMut<IdRange<T>> for CpuBuffer<T> {
    fn index_mut(&mut self, ids: IdRange<T>) -> &mut [T] { &mut self.data[ids.usize_range()] }
}

pub struct SimpleBufferAllocator {
    back_index: u16,
    front_index: u16,
    len: u16,
}

impl SimpleBufferAllocator {
    pub fn new(len: u16) -> Self {
        SimpleBufferAllocator {
            back_index: len,
            front_index: 0,
            len: len,
        }
    }

    pub fn len(&self) -> u16 { self.len }

    pub fn available_size(&self) -> u16 { self.back_index - self.front_index }

    pub fn alloc_range_back(&mut self, len: u16) -> Option<(u16, u16)> {
        if self.available_size() < len {
            return None;
        }

        self.back_index -= len;

        return Some((self.back_index, len));
    }

    pub fn alloc_back(&mut self) -> Option<u16> { self.alloc_range_back(1).map(|range| range.0) }

    pub fn alloc_range(&mut self, len: u16) -> Option<(u16, u16)> {
        if self.available_size() < len {
            return None;
        }

        let id = self.front_index;
        self.front_index += len;

        return Some((id, len));
    }

    pub fn alloc(&mut self) -> Option<u16> { self.alloc_range(1).map(|range| range.0) }
}

pub struct TypedSimpleBufferAllocator<T> {
    alloc: SimpleBufferAllocator,
    _marker: PhantomData<T>,
}

impl<T> TypedSimpleBufferAllocator<T> {
    pub fn new(len: u16) -> Self {
        TypedSimpleBufferAllocator {
            alloc: SimpleBufferAllocator::new(len),
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> u16 { self.alloc.len() }

    pub fn alloc(&mut self) -> Option<Id<T>> { self.alloc.alloc().map(|id| Id::new(id)) }

    pub fn alloc_back(&mut self) -> Option<Id<T>> { self.alloc.alloc_back().map(|id| Id::new(id)) }

    pub fn alloc_range(&mut self, len: u16) -> Option<IdRange<T>> {
        self.alloc.alloc_range(len).map(|(first, count)| IdRange::new(first..(first + count)))
    }

    pub fn alloc_range_back(&mut self, len: u16) -> Option<IdRange<T>> {
        self.alloc
            .alloc_range_back(len)
            .map(|(first, count)| IdRange::new(first..(first + count)))
    }
}


pub struct BufferStore<Primitive> {
    pub buffers: Vec<CpuBuffer<Primitive>>,
    current: BufferId<Primitive>,
    buffer_len: u16,
}

impl<Primitive: Copy + Default> BufferStore<Primitive> {
    pub fn new(count: u16, size: u16) -> Self {
        let mut store = BufferStore {
            buffers: Vec::new(),
            current: BufferId::new(0),
            buffer_len: size,
        };
        for _ in 0..count {
            store.alloc_buffer();
        }
        return store;
    }

    pub fn first_buffer_id(&self) -> BufferId<Primitive> { BufferId::new(0) }

    pub fn current_buffer_id(&self) -> BufferId<Primitive> { self.current }

    pub fn current_buffer(&self) -> &CpuBuffer<Primitive> { &self[self.current] }

    pub fn mut_current_buffer(&mut self) -> &mut CpuBuffer<Primitive> {
        let current = self.current;
        &mut self[current]
    }

    pub fn bump_current_buffer(&mut self) {
        self.current = BufferId::new(self.current.to_u32() + 1);
        while self.current.index() >= self.buffers.len() {
            self.alloc_buffer();
        }
    }

    pub fn alloc_buffer(&mut self) {
        let len = self.buffer_len;
        self.buffers.push(CpuBuffer::new(len));
    }

    pub fn alloc_range(&mut self, count: u16) -> BufferRange<Primitive> {
        assert!(count <= self.buffer_len);
        loop {
            if let Some(range) = self.mut_current_buffer().try_alloc_range(count) {
                return BufferRange {
                           buffer: self.current,
                           range: range,
                       };
            }
            self.bump_current_buffer();
        }
    }

    pub fn alloc(&mut self) -> BufferElement<Primitive> {
        loop {
            if let Some(id) = self.mut_current_buffer().try_alloc() {
                return BufferElement {
                           buffer: self.current,
                           element: id,
                       };
            }
            self.bump_current_buffer();
        }
    }

    pub fn push(&mut self, value: Primitive) -> BufferElement<Primitive> {
        let id = self.alloc();
        self[id.buffer][id.element] = value;
        return id;
    }

    pub fn alloc_range_back(&mut self, count: u16) -> BufferRange<Primitive> {
        assert!(count <= self.buffer_len);
        loop {
            if let Some(range) = self.mut_current_buffer().try_alloc_range(count) {
                return BufferRange {
                           buffer: self.current,
                           range: range,
                       };
            }
            self.bump_current_buffer();
        }
    }

    pub fn alloc_back(&mut self) -> BufferElement<Primitive> {
        loop {
            if let Some(id) = self.mut_current_buffer().try_alloc_back() {
                return BufferElement {
                           buffer: self.current,
                           element: id,
                       };
            }
            self.bump_current_buffer();
        }
    }

    pub fn push_back(&mut self, value: Primitive) -> BufferElement<Primitive> {
        let id = self.alloc_back();
        self[id.buffer][id.element] = value;
        return id;
    }
}

impl<T> ops::Index<BufferId<T>> for BufferStore<T> {
    type Output = CpuBuffer<T>;
    fn index(&self, id: BufferId<T>) -> &CpuBuffer<T> { &self.buffers[id.index()] }
}

impl<T> ops::IndexMut<BufferId<T>> for BufferStore<T> {
    fn index_mut(&mut self, id: BufferId<T>) -> &mut CpuBuffer<T> { &mut self.buffers[id.index()] }
}

impl<T: Copy + Default> ops::Index<BufferRange<T>> for BufferStore<T> {
    type Output = [T];
    fn index(&self, id: BufferRange<T>) -> &[T] { &self.buffers[id.buffer.index()][id.range] }
}

impl<T: Copy + Default> ops::IndexMut<BufferRange<T>> for BufferStore<T> {
    fn index_mut(&mut self, id: BufferRange<T>) -> &mut [T] {
        &mut self.buffers[id.buffer.index()][id.range]
    }
}

impl<T: Copy + Default> ops::Index<BufferElement<T>> for BufferStore<T> {
    type Output = T;
    fn index(&self, id: BufferElement<T>) -> &T { &self.buffers[id.buffer.index()][id.element] }
}

impl<T: Copy + Default> ops::IndexMut<BufferElement<T>> for BufferStore<T> {
    fn index_mut(&mut self, id: BufferElement<T>) -> &mut T {
        &mut self.buffers[id.buffer.index()][id.element]
    }
}
