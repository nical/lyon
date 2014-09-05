use std::ty::Unsafe;
use std::sync::atomics;

use std::vec::{Vec};

use core::clone::Clone;
use libc::funcs::c95::stdlib::{malloc, free};
use std::mem;

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

#[deriving(Clone, Show)]
pub struct Id<T> {
    pub handle: u32,
    pub gen: u32,
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Id<T>) -> bool { self.handle == other.handle && self.gen == other.gen }
    fn ne(&self, other: &Id<T>) -> bool { self.handle != other.handle || self.gen != other.gen }
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
            return &*self.ptr;
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
            &(*self.data.get(index as uint).ptr).data
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
                return &mut (*self.data.get_mut(index as uint).ptr).data;
            }
            // "copy on write" scenario.
            let mut node_ref = self.data.get_mut(index as uint);
            *node_ref = node_ref.deep_clone();
            return &mut (*node_ref.ptr).data;
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
