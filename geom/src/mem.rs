use std::ptr::{Unique, write_bytes};
use std::mem::{forget, transmute, size_of};

/// An Owned buffer that can be recycled into a Vec to help with avoiding allocations.
///
/// # Examples
///
/// ```
/// use geom::mem::VecStorage;
///
/// let mut v = vec![1u16, 2, 3, 4, 5];
/// let mut storage = VecStorage::from_vec(v);
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
pub struct VecStorage {
    ptr: Unique<u8>,
    cap: usize,
}

impl VecStorage {

    /// Creates an empty VecStorage.
    pub fn new() -> VecStorage {
        unsafe {
            VecStorage {
                ptr: transmute(0 as usize),
                cap: 0
            }
        }
    }

    /// Consumes a Vec and creates a VecStorage that holds the Vec's buffer.
    ///
    /// The vector is cleared and its data is dropped before the it is consumed.
    pub fn from_vec<T>(mut vector: Vec<T>) -> VecStorage {
        vector.clear();
        let capacity = vector.capacity() * size_of::<T>();
        unsafe {
            let p = vector.as_mut_ptr();
            forget(vector);
            return VecStorage {
                ptr: Unique::new(transmute(p)),
                cap: capacity,
            };
        }
    }
  
    /// Creates a Vec recycling this vector storage.
    ///
    /// The length of the new vector is 0 and the capacity is self.capacity() / size_of::<T>().
    pub fn into_vec<T>(self) -> Vec<T> {
        unsafe {
            let vector = Vec::from_raw_parts(transmute(self.ptr.get()), 0,
            self.cap / size_of::<T>());
            forget(self);
            return vector;
        }
    }
  
    /// Returns the size of the buffer in bytes.
    pub fn capacity(&self) -> usize { self.cap }

    /// Fills the buffer with zeros.
    pub fn zero_out(&mut self) {
        if self.cap == 0 { return; }
        unsafe {
            write_bytes(self.ptr.get_mut(), 0, self.cap);
        }
    }
}
