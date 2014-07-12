use std::vec::{Vec};

use std::ty::Unsafe;
use core::atomics;
use core::clone::Clone;
use libc::funcs::c95::stdlib::{malloc, free};
use std::mem;

type Index = u32;
static FREE_LIST_NONE: Index = 2147483647 as Index;

struct PodFreeListVector<T> {
    data: Vec<FreeListVectorSlot<T>>,
    free_list: Index,
}

struct FreeListVectorSlot<T> {
    payload: T,
    free_list: Index,
}

impl<T: Copy> PodFreeListVector<T> {
    pub fn new() -> PodFreeListVector<T> {
        PodFreeListVector {
            data: Vec::new(),
            free_list: FREE_LIST_NONE
        }
    }

    pub fn with_capacity(capacity: uint) -> PodFreeListVector<T> {
        PodFreeListVector {
            data: Vec::with_capacity(capacity),
            free_list: FREE_LIST_NONE
        }
    }

    pub fn add(&mut self, val: T) -> Index {
        if self.free_list == FREE_LIST_NONE {
            self.data.push(FreeListVectorSlot{ payload: val, free_list: FREE_LIST_NONE });
            return (self.data.len()-1) as Index;
        } else {
            let index = self.free_list;
            let next_free_list = self.data.get(index as uint).free_list;
            self.data.get_mut(self.free_list as uint).payload = val;
            self.free_list = next_free_list;
            return index;
        }
    }

    pub fn remove(&mut self, idx: Index) {
        self.data.get_mut(idx as uint).free_list = self.free_list;
        self.free_list = idx;
    }

    pub fn clear(&mut self) {
        self.free_list = FREE_LIST_NONE;
    }

    pub fn borrow<'l>(&'l self, id: Index) -> &'l T {
        assert!(self.data.get(id as uint).free_list == FREE_LIST_NONE);
        return &'l self.data.get(id as uint).payload;
    }

    pub fn borrow_mut<'l>(&'l mut self, idx: Index) -> &'l mut T {
        assert!(self.data.get(idx as uint).free_list == FREE_LIST_NONE);
        return &'l mut self.data.get_mut(idx as uint).payload;
    }
}

struct IdLookupTable {
    // Dense array
    data_to_index: Vec<Index>,
    // Sparse array
    index_to_data: Vec<Index>,
    // offset of the first empty element in the sparse array
    free_list: Index,
}

impl IdLookupTable {
    pub fn new() -> IdLookupTable {
        IdLookupTable {
            data_to_index: Vec::new(),
            index_to_data: Vec::new(),
            free_list: FREE_LIST_NONE,
        }
    }

    pub fn with_capacity(capacity: uint) -> IdLookupTable {
        IdLookupTable {
            data_to_index: Vec::with_capacity(capacity),
            index_to_data: Vec::with_capacity(capacity),
            free_list: FREE_LIST_NONE,
        }
    }

    pub fn add(&mut self) -> Index {
        if self.free_list == FREE_LIST_NONE {
            self.index_to_data.push(self.data_to_index.len() as Index);
            self.data_to_index.push((self.index_to_data.len()-1) as Index);
            return (self.index_to_data.len()-1) as Index;
        }
        let idx = self.free_list as uint;
        self.free_list = *self.index_to_data.get(idx);
        *self.index_to_data.get_mut(idx) = self.data_to_index.len() as Index;
        self.data_to_index.push(idx as Index);
        return idx as Index;
    }

    pub fn remove(&mut self, idx: Index) {
        let o = *self.index_to_data.get(idx as uint) as uint;
        if o == self.data_to_index.len()-1 {
            self.data_to_index.pop();
        } else {
            let moved = self.data_to_index.pop().unwrap();
            *self.index_to_data.get_mut(moved as uint) = o as Index;
            *self.data_to_index.get_mut(o) = moved;
        }
        *self.index_to_data.get_mut(idx as uint) = self.free_list;
        self.free_list = idx;
    }

    pub fn clear(&mut self) {
        self.free_list = FREE_LIST_NONE;
        self.data_to_index.clear();
        self.index_to_data.clear();
    }

    pub fn lookup(&self, idx: Index) -> Index { *self.index_to_data.get(idx as uint) }

    pub fn index_for_offset(&self, idx: Index) -> Index { *self.data_to_index.get(idx as uint) }

    pub fn len(&self) -> uint { self.data_to_index.len() }

    pub fn reserve(&mut self, size: uint) {
        self.index_to_data.reserve(size);
        self.data_to_index.reserve(size);
    }

    pub fn indices<'l>(&'l self) -> &'l[Index] {
        return self.data_to_index.as_slice();
    }

    pub fn swap_at_indices(&mut self, idx1: Index, idx2: Index) {
        let o1 = self.lookup(idx1);
        let o2 = self.lookup(idx2);
        self.swap_offsets(o1, o2);
    }

    pub fn swap_offsets(&mut self, o1: Index, o2: Index) {
        let temp = *self.data_to_index.get(o1 as uint);
        *self.data_to_index.get_mut(o1 as uint) = *self.data_to_index.get(o2 as uint);
        *self.data_to_index.get_mut(o2 as uint) = temp;
        *self.index_to_data.get_mut(*self.data_to_index.get(o2 as uint) as uint) = o1;
        *self.index_to_data.get_mut(*self.data_to_index.get(o1 as uint) as uint) = o2;
    }
}

/// vector of Copy-on-write atomically reference counted values.
/// Nodes are stored in a vector and refered to using an id equivalent to their
/// position in the vector.
/// When a node is removed, it is not deallocated right away: instead its index
/// is added to the list of available slots, and the allocated node will be
/// recycled next time a node is created.
pub struct CwArcTable<T> {
    data: Vec<UnsafeArc<T>>,
    free_slots: Vec<u16>,
    next_node_gen: u32,
}

pub struct UnsafeArcInner<T> {
    ref_count: atomics::AtomicInt,
    data: T,
    gen: u32,
}

struct UnsafeArc<T> {
    ptr: *mut UnsafeArcInner<T>
}

#[deriving(PartialEq, Clone)]
pub struct Id<T> {
    pub handle: u32,
    pub gen: u32,
}

impl<T: Clone> UnsafeArcInner<T> {
    fn new(data: T, gen: u32) -> UnsafeArcInner<T> {
        let n = UnsafeArcInner {
            ref_count: atomics::AtomicInt::new(1),
            data: data,
            gen: gen,
        };
        return n;
    }
}

impl<T: Clone> UnsafeArc<T> {
    fn new(data: T, gen: u32) -> UnsafeArc<T> {
        unsafe {
            let result = UnsafeArc {
                ptr: mem::transmute(malloc(mem::size_of::<UnsafeArcInner<T>>() as u64))
            };
            (*result.ptr) = UnsafeArcInner::new(data, gen);
            return result;
        }
    }

    fn deep_clone(&self) -> UnsafeArc<T> {
        unsafe {
            let result = UnsafeArc {
                ptr: mem::transmute(malloc(mem::size_of::<UnsafeArcInner<T>>() as u64))
            };
            (*result.ptr) = UnsafeArcInner::new(
                (*self.ptr).data.clone(),
                (*self.ptr).gen
            );
            return result;
        }
    }

    #[inline]
    pub fn add_ref(& self) {
        unsafe {
            (*self.ptr).ref_count.fetch_add(1, atomics::SeqCst);
        }
    }

    #[inline]
    pub fn release_ref(&mut self) {
        unsafe {
            if (*self.ptr).ref_count.fetch_sub(1, atomics::Release) == 1 {
                atomics::fence(atomics::Acquire);
                free(mem::transmute(self.ptr));
            }
        }
    }

    #[inline]
    pub fn ref_count(&self) -> int {
        unsafe {
            return (*self.ptr).ref_count.load(atomics::SeqCst);
        }
    }

    #[inline]
    fn inner<'l>(&'l self) -> &'l UnsafeArcInner<T> {
        unsafe {
            return &'l (*self.ptr);
        }
    }
}

impl<T: Clone> Clone for UnsafeArc<T> {
    fn clone(&self) -> UnsafeArc<T> {
        unsafe {
            self.add_ref();
            UnsafeArc { ptr: self.ptr }
        }
    }
}

#[unsafe_destructor]
impl<T: Clone> Drop for UnsafeArc<T> {
    fn drop(&mut self) {
        unsafe {
            self.release_ref();
        }
    }
}

// TODO: iterator, clear
impl<T: Clone> CwArcTable<T> {
    pub fn new() -> CwArcTable<T> {
        CwArcTable {
            data: Vec::new(),
            free_slots: Vec::new(),
            next_node_gen: 0,
        }
    }

    pub fn add(&mut self, data: T) -> Id<T> {
        unsafe {
            let new_node = UnsafeArc::new(data, self.next_node_gen);
            let node_id;
            if !self.free_slots.is_empty() {
                // Re-use availale slot...
                *self.data.get_mut(self.free_slots.len()-1) = new_node;
                let idx = self.free_slots.pop().unwrap();
                node_id = Id {
                    handle: idx as u32,
                    gen: self.next_node_gen,
                };
            } else {
                // ...or push to the end of the vector.
                self.data.push(new_node);
                node_id = Id {
                    handle: self.data.len() as u32 - 1,
                    gen: self.next_node_gen,
                };
            }
            self.next_node_gen += 1;
            return node_id;
        }
    }

    // Adds this node to the list of available slots.
    // The node will be actually removed whenever we add another node in its place.
    pub fn remove(&mut self, node: Id<T>)
    {
        if self.free_slots.contains(&(node.handle as u16)) {
            fail!("Removed node {} twice!", node.handle);
        }
        self.free_slots.push(node.handle as u16);
    }

    pub fn get<'l>(&'l self, node: Id<T>) -> &'l T {
        if node.gen != self.get_gen(node.handle) { fail!("invalid handle"); }
        return self.unchecked_get(node.handle);
    }

    pub fn unchecked_get<'l>(&'l self, index: u32) -> &'l T {
        unsafe {
            &'l (*self.data.get(index as uint).ptr).data
        }
    }

    pub fn get_mut<'l>(&'l mut self, node: Id<T>) -> &'l mut T {
        if node.gen != self.get_gen(node.handle) { fail!("invalid handle"); }
        return self.unchecked_get_mut(node.handle);
    }

    pub fn unchecked_get_mut<'l>(&'l mut self, index: u32) -> &'l mut T {
        unsafe {
            // If we are holding the only ref, no need to copy.
            if self.data.get_mut(index as uint).ref_count() == 1 {
                return &'l mut (*self.data.get_mut(index as uint).ptr).data;
            }
            // "copy on write" scenario.
            let mut node_ref = self.data.get_mut(index as uint);
            *node_ref = node_ref.deep_clone();
            return &'l mut (*node_ref.ptr).data;
        }
    }

    pub fn len(&self) -> uint {
        return self.data.len() - self.free_slots.len();
    }

    pub fn snapshot(&mut self) -> CwArcTable<T> {
        unsafe {
            let mut clone: CwArcTable<T> = CwArcTable {
                data: self.data.clone(),
                free_slots: self.free_slots.clone(),
                next_node_gen: self.next_node_gen + 1000,
            };
            return clone;
        }
    }

    pub fn get_gen(&self, index: u32) -> u32 {
        return self.data.get(index as uint).inner().gen;
    }
}

mod tests {
    use super::{IdLookupTable, Index};

    fn check_lookup_table(table: &mut IdLookupTable) {
        assert_eq!(table.len(), 0);

        for i in range(0, 100) {
            table.add();
            assert_eq!(table.lookup(table.index_for_offset(i as Index)), i as Index);
            assert_eq!(table.len(), (i+1) as uint);
        }

        for i in range(0, table.len()-1) {
            assert_eq!(table.lookup(table.index_for_offset(i as Index)), i as Index);
        }

        table.remove(10);
        table.remove(1);
        table.remove(0);
        table.remove(5);
        table.remove(25);

        for i in range(0, table.len()-1) {
            assert_eq!(table.lookup(table.index_for_offset(i as Index)), i as Index);
        }

        for _ in range(0, 10) {
            table.add();
            for i in range(0, table.len()-1) {
                assert_eq!(table.lookup(table.index_for_offset(i as Index)), i as Index);
            }
        }
    }

    #[test]
    fn id_lookup_table() {
        let mut t1 = IdLookupTable::new();
        check_lookup_table(&mut t1);
        t1.clear();
        check_lookup_table(&mut t1);

        let mut t2 = IdLookupTable::with_capacity(30);
        check_lookup_table(&mut t2);
        t2.clear();
        check_lookup_table(&mut t2);
    }
}