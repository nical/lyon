use super::{ Identifier, FromIndex, ToIndex };
use std::default::Default;
use std::slice;
use std::marker::PhantomData;
use std::ops;

pub struct SparseIdVector<ID:Identifier, Data> {
    data: Vec<Data>,
    _idtype: PhantomData<ID>
}


impl<ID:Identifier, Data> SparseIdVector<ID, Data> {

    /// Create an empty SparseIdVector
    pub fn new() -> SparseIdVector<ID, Data> {
        SparseIdVector {
            data: Vec::new(),
            _idtype: PhantomData
        }
    }

    /// Create an SparseIdVector with preallocated storage
    pub fn with_capacity(size: ID::Handle) -> SparseIdVector<ID, Data> {
        SparseIdVector {
            data: Vec::with_capacity(size.to_index()),
            _idtype: PhantomData
        }
    }

    /// Create an SparseIdVector by recycling a Vec and its content.
    pub fn from_vec(vec: Vec<Data>) -> SparseIdVector<ID, Data> {
        SparseIdVector {
            data: vec,
            _idtype: PhantomData
        }
    }

    /// Consume the SparseIdVector and create a Vec.
    pub fn into_vec(self) -> Vec<Data> { self.data }

    /// Number of elements in the SparseIdVector
    pub fn len(&self) -> usize { self.data.len() }

    /// Return the nth element of the SparseIdVector using an usize index rather than an Id (à la Vec).
    pub fn nth(&self, idx: usize) -> &Data { &self.data[idx] }

    /// Return the nth element of the SparseIdVector using an usize index rather than an Id (à la Vec).
    pub fn nth_mut(&mut self, idx: usize) -> &mut Data { &mut self.data[idx] }

    // /// Iterate over the elements of the SparseIdVector
    // pub fn iter<'l>(&'l self) -> slice::Iter<'l, Data> { self.data.iter() }
    // /// Iterate over the elements of the SparseIdVector
    // pub fn iter_mut<'l>(&'l mut self) -> slice::IterMut<'l, Data> { self.data.iter_mut() }

    /// Add an element to the SparseIdVector and return its Id.
    /// This method can cause the storage to be reallocated.
    pub fn add(&mut self, elt: Data) -> ID {
        // TODO: have a proper allocator-like logic
        return self.push(elt);
    }


    /// Push an element to the SparseIdVector at the end of the storage and return its Id.
    /// This method can cause the storage to be reallocated.
    pub fn push(&mut self, elt: Data) -> ID {
        let index = self.data.len();
        self.data.push(elt);
        return FromIndex::from_index(index);
    }

    pub fn remove(&mut self, _id: ID) {
        // TODO
    }

    pub fn has_id(&self, id: ID) -> bool { id.to_index() < self.data.len() }

    pub fn first_id(&self) -> Option<ID> {
        return if self.data.len() > 0 { Some(ID::from_index(0)) } else { None };
    }

    /// Drop all of the contained elements and clear the SparseIdVector's storage.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn reserve(&mut self, size: ID::Handle) {
        self.data.reserve(size.to_index());
    }
}

impl<ID:Identifier, Data: Default> SparseIdVector<ID, Data> {
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

impl<Data:Default, ID:Identifier> SparseIdVector<ID, Data> {
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

    pub fn with_length(size: u16) -> SparseIdVector<ID, Data> {
        let mut result: SparseIdVector<ID, Data> = SparseIdVector::new();
        result.resize(size);
        return result;
    }
}

impl<ID:Identifier, Data> ops::Index<ID> for SparseIdVector<ID, Data> {
    type Output = Data;
    fn index<'l>(&'l self, id: ID) -> &'l Data { &self.data[id.to_index()] }
}

impl<ID:Identifier, Data> ops::IndexMut<ID> for SparseIdVector<ID, Data> {
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

    pub fn as_slice<'a>(&'a self) -> &'a[Data] { self.slice }

    pub fn iter<'a>(&'a self) -> slice::Iter<'a, Data> { self.slice.iter() }
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
