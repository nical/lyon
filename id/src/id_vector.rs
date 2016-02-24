use super::{ Identifier, FromIndex, IdRange, IntegerHandle };
use std::default::Default;
use std::slice;
use std::marker::PhantomData;
use std::ops;

/// Similar to Vec except that it is indexed using an Id rather than an usize index.
/// if the stored type implements Default, IdVector can also use the set(...) method which can
/// grow the vector to accomodate for the requested id.
pub struct IdVector<ID:Identifier, Data> {
    data: Vec<Data>,
    _idtype: PhantomData<ID>
}

impl<ID:Identifier, Data> IdVector<ID, Data> {

    /// Create an empty IdVector
    pub fn new() -> IdVector<ID, Data> {
        IdVector {
            data: Vec::new(),
            _idtype: PhantomData
        }
    }

    /// Create an IdVector with preallocated storage
    pub fn with_capacity(size: u16) -> IdVector<ID, Data> {
        IdVector {
            data: Vec::with_capacity(size as usize),
            _idtype: PhantomData
        }
    }

    /// Create an IdVector by recycling a Vec and its content.
    pub fn from_vec(vec: Vec<Data>) -> IdVector<ID, Data> {
        IdVector {
            data: vec,
            _idtype: PhantomData
        }
    }

    /// Consume the IdVector and create a Vec.
    pub fn into_vec(self) -> Vec<Data> { self.data }

    /// Number of elements in the IdVector
    pub fn len(&self) -> usize { self.data.len() }

    /// Return the nth element of the IdVector using an usize index rather than an Id (à la Vec).
    pub fn nth(&self, idx: usize) -> &Data { &self.data[idx] }

    /// Return the nth element of the IdVector using an usize index rather than an Id (à la Vec).
    pub fn nth_mut(&mut self, idx: usize) -> &mut Data { &mut self.data[idx] }

    /// Iterate over the elements of the IdVector
    pub fn iter<'l>(&'l self) -> slice::Iter<'l, Data> { self.data.iter() }

    /// Iterate over the elements of the IdVector
    pub fn iter_mut<'l>(&'l mut self) -> slice::IterMut<'l, Data> { self.data.iter_mut() }

    /// Add an element to the IdVector and return its Id.
    /// This method can cause the storage to be reallocated.
    pub fn push(&mut self, elt: Data) -> ID {
        let index = self.data.len();
        self.data.push(elt);
        return FromIndex::from_index(index);
    }

    /// Drop all of the contained elements and clear the IdVector's storage.
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl<ID:Identifier, Data: Default> IdVector<ID, Data> {
    /// Set the value for a certain Id, possibly adding default values if the Id's index is Greater
    /// than the size of the underlying vector.
    pub fn set(&mut self, id: ID, val: Data) {
        while self.len() < id.to_index() {
            self.push(Data::default());
        }
        if self.len() == id.to_index() {
            self.push(val);
        } else {
            self[id] = val;
        }
    }
}

impl<Data:Default, ID:Identifier> IdVector<ID, Data> {
    pub fn resize(&mut self, size: u16) {
        let d = size as i32 - self.data.len() as i32;
        if d > 0 {
            self.data.reserve(d as usize);
            for _ in 0 .. d {
                self.data.push(Default::default());
            }
        } else {
            for _ in 0 .. -d {
                self.data.pop();
            }
        }
    }

    pub fn with_length(size: u16) -> IdVector<ID, Data> {
        let mut result: IdVector<ID, Data> = IdVector::new();
        result.resize(size);
        return result;
    }
}

impl<ID:Identifier, Data> ops::Index<ID> for IdVector<ID, Data> {
    type Output = Data;
    fn index<'l>(&'l self, id: ID) -> &'l Data { &self.data[id.to_index()] }
}

impl<ID:Identifier, Data> ops::IndexMut<ID> for IdVector<ID, Data> {
    fn index_mut<'l>(&'l mut self, id: ID) -> &'l mut Data { &mut self.data[id.to_index()] }
}


pub struct IdSlice<'l, ID:Identifier, Data> where Data:'l {
    slice: &'l[Data],
    _idtype: PhantomData<ID>
}

impl<'l, Data, ID:Identifier> Copy for IdSlice<'l, ID, Data> where Data:'l {}
impl<'l, Data, ID:Identifier> Clone for IdSlice<'l, ID, Data> where Data:'l {
    fn clone(&self) -> IdSlice<'l, ID, Data> {
        IdSlice {
            slice: self.slice,
            _idtype: PhantomData,
        }
    }
}

impl<'l, Data, ID:Identifier> IdSlice<'l, ID, Data> where Data:'l {
    pub fn new(slice: &'l[Data]) -> IdSlice<'l, ID, Data> {
        IdSlice {
            slice: slice,
            _idtype: PhantomData
        }
    }

    pub fn len(&self) -> usize { self.slice.len() }

    pub fn as_slice<'a>(&'a self) -> &'a[Data] { self.slice }

    pub fn iter<'a>(&'a self) -> slice::Iter<'a, Data> { self.slice.iter() }

    pub fn ids<T:IntegerHandle>(&self) -> IdRange<ID::Unit, T> {
        IdRange::new(FromIndex::from_index(0), FromIndex::from_index(self.len()))
    }
}

impl<'l, ID:Identifier, Data> ops::Index<ID> for IdSlice<'l, ID, Data> where Data:'l {
    type Output = Data;
    fn index<'a>(&'a self, id: ID) -> &'a Data { &self.slice[id.to_index()] }
}



pub struct MutIdSlice<'l, ID:Identifier, Data:'l> {
    slice: &'l mut[Data],
    _idtype: PhantomData<ID>
}

impl<'l, ID:Identifier, Data:'l> MutIdSlice<'l, ID, Data>{
    pub fn new(slice: &'l mut[Data]) -> MutIdSlice<'l, ID, Data> {
        MutIdSlice {
            slice: slice,
            _idtype: PhantomData
        }
    }

    pub fn iter<'a>(&'a self) -> slice::Iter<'a, Data> { self.slice.iter() }
    pub fn iter_mut<'a>(&'a mut self) -> slice::IterMut<'a, Data> { self.slice.iter_mut() }
}

impl<'l, ID:Identifier, Data:'l> ops::Index<ID> for MutIdSlice<'l, ID, Data> {
    type Output = Data;
    fn index<'a>(&'a self, id: ID) -> &'a Data { &self.slice[id.to_index()] }
}

impl<'l, ID:Identifier, Data:'l> ops::IndexMut<ID> for MutIdSlice<'l, ID, Data> {
    fn index_mut<'a>(&'a mut self, id: ID) -> &'a mut Data { &mut self.slice[id.to_index()] }
}

#[test]
fn test_id_vector() {
    use super::*;

    #[derive(Debug)]
    struct T;

    fn id(i: u16) -> Id<T, u16> { Id::new(i) }

    let mut v = IdVector::new();
    let a = v.push(42 as u32);
    assert_eq!(v[a], 42);
    v.set(a, 0);
    assert_eq!(v[a], 0);

    v.set(id(10), 100);
    assert_eq!(v[id(10)], 100);

    v.set(id(5), 50);
    assert_eq!(v[id(5)], 50);

    v.set(id(20), 200);
    assert_eq!(v[id(20)], 200);
    assert_eq!(v.len(), 21);
}
