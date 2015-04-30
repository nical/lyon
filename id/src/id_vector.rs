use super::{ Identifier, FromIndex };
use std::default::Default;
use std::slice;
use std::marker::PhantomData;
use std::ops;

pub struct IdVector<ID:Identifier, Data> {
    data: Vec<Data>,
    _idtype: PhantomData<ID>
}

impl<ID:Identifier, Data> IdVector<ID, Data> {

    pub fn new() -> IdVector<ID, Data> {
        IdVector {
            data: Vec::new(),
            _idtype: PhantomData
        }
    }

    pub fn with_capacity(size: u16) -> IdVector<ID, Data> {
        IdVector {
            data: Vec::with_capacity(size as usize),
            _idtype: PhantomData
        }
    }

    pub fn from_vec(vec: Vec<Data>) -> IdVector<ID, Data> {
        IdVector {
            data: vec,
            _idtype: PhantomData
        }
    }

    pub fn into_vec(self) -> Vec<Data> { self.data }

    pub fn len(&self) -> usize { self.data.len() }

    pub fn nth(&self, idx: usize) -> &Data { &self.data[idx] }

    pub fn nth_mut(&mut self, idx: usize) -> &mut Data { &mut self.data[idx] }

    pub fn iter<'l>(&'l self) -> slice::Iter<'l, Data> { self.data.iter() }

    pub fn iter_mut<'l>(&'l mut self) -> slice::IterMut<'l, Data> { self.data.iter_mut() }

    pub fn push(&mut self, elt: Data) -> ID {
        let index = self.data.len();
        self.data.push(elt);
        return FromIndex::from_index(index);
    }

    pub fn clear(&mut self) {
        self.data.clear();
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
