use std::ptr::{ write_bytes };
use std::mem::{forget, transmute, size_of};

/// An Owned buffer that can be recycled into a Vec to help with avoiding allocations.
///
/// # Examples
///
/// ```
/// use geom::mem::Allocation;
///
/// let mut v = vec![1u16, 2, 3, 4, 5];
/// let mut storage = Allocation::from_vec(v);
/// // v is now gone into the void.
///
/// let mut v: Vec<f32> = storage.into_vec();
///
/// assert_eq!(v.len(), 0);
/// assert!(v.capacity() > 0);
///
/// ```


/// Holds memory that was allocated by a Vec and can be reused by another Vec.
///
/// This is useful to avoid reallocating temporary vectors.
pub struct Allocation {
    ptr: *mut u8,
    cap: usize,
}

pub fn pre_allocate(size: usize) -> Allocation {
    let alloc: Vec<u8> = Vec::with_capacity(size);
    return Allocation::from_vec(alloc);
}

impl Allocation {
    /// Creates an empty Allocation.
    pub fn empty() -> Allocation {
        unsafe {
            Allocation {
                ptr: transmute(0 as usize),
                cap: 0
            }
        }
    }

    /// Consumes a Vec and creates a Allocation that holds the Vec's buffer.
    ///
    /// The vector is cleared and its data is dropped before the it is consumed.
    pub fn from_vec<T>(mut vector: Vec<T>) -> Allocation {
        vector.clear();
        let capacity = vector.capacity() * size_of::<T>();
        unsafe {
            let p = vector.as_mut_ptr();
            forget(vector);
            return Allocation {
                ptr: transmute(p),
                cap: capacity,
            };
        }
    }

    /// Creates a Vec recycling this vector storage.
    ///
    /// The length of the new vector is 0 and the capacity is self.capacity() / size_of::<T>().
    pub fn into_vec<T>(self) -> Vec<T> {
        unsafe {
            let vector = Vec::from_raw_parts(transmute(self.ptr), 0, self.cap / size_of::<T>());
            forget(self);
            return vector;
        }
    }

    /// Returns the size of the buffer in bytes.
    pub fn capacity(&self) -> usize { self.cap }

    /// Fills the buffer with the byte pattern.
    pub fn poison(&mut self, pattern: u8) {
        if self.cap == 0 { return; }
        unsafe {
            write_bytes(self.ptr, pattern, self.cap);
        }
    }

    /// Fills the buffer with zeros.
    pub fn zero_out(&mut self) {
        self.poison(0);
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            // let a vector acquire the buffer and drop it
            let _ : Vec<u8> = Vec::from_raw_parts(transmute(self.ptr), 0, self.cap);
        }
    }
}