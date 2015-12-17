use std::marker::PhantomData;
use std::ops::{ Add, Sub };

pub mod id_vector;
pub mod id_list;


// --------------------------------------------------------------------------------------------- Id

#[derive(Debug)]
pub struct Id<T, H> {
    pub handle: H,
    pub _marker: PhantomData<T>
}

impl<T, H:Copy> Copy for Id<T, H> {}

impl<T, H:Copy> Clone for Id<T, H> { fn clone(&self) -> Id<T, H> { *self } }

impl<T, H:PartialEq> PartialEq for Id<T, H> {
    fn eq(&self, other: &Id<T,H>) -> bool { self.handle.eq(&other.handle) }
}

impl<T, H:Copy+Eq> Eq for Id<T, H> {}

impl<T, H:Copy> Id<T, H> {
    pub fn new(idx: H) -> Id<T, H> { Id { handle: idx, _marker: PhantomData } }
}

impl<T, H:IntegerHandle> Identifier for Id<T, H> {
    type Handle = H;
}

impl<T, H:ToIndex> ToIndex for Id<T, H> {
    fn to_index(&self) -> usize { self.handle.to_index() }
}

impl<T, H:Copy+FromIndex> FromIndex for Id<T, H> {
    fn from_index(idx: usize) -> Id<T, H> { Id::new(FromIndex::from_index(idx)) }
}


// ---------------------------------------------------------------------------------------- IdRange

#[derive(Debug)]
pub struct IdRange<T, H> {
    pub first: Id<T, H>,
    pub count: H,
}

impl<T, H:Copy> Copy for IdRange<T, H> {}

impl<T, H:Copy> Clone for IdRange<T, H> { fn clone(&self) -> IdRange<T, H> { *self } }

impl<T, H:PartialEq> PartialEq for IdRange<T, H> {
    fn eq(&self, other: &IdRange<T,H>) -> bool { self.first.eq(&other.first) && self.count.eq(&other.count) }
}

impl<T, H:Copy+Eq> Eq for IdRange<T, H> {}

impl<T, H:IntegerHandle> IdRange<T, H> {
    pub fn get(self, i: H) -> Id<T, H> {
        debug_assert!(i < self.count);
        return Id { handle: self.first.handle + i, _marker: PhantomData };
    }
}

impl<T, H:IntegerHandle> Iterator for IdRange<T, H> {
    type Item = Id<T, H>;
    fn next(&mut self) -> Option<Id<T, H>> {
        if self.count.to_index() == 0 {
            return None;
        }
        let res = self.first;
        self.count = FromIndex::from_index(self.count.to_index() - 1);
        self.first = FromIndex::from_index(self.first.to_index() + 1);
        return Some(res);
    }
}


// ------------------------------------------------------------------------------------------ GenId
// TODO: remove it or implement traits manually

#[derive(Copy, Clone, Debug)]
pub struct GenId<T, H:Copy, G> {
    pub id: Id<T, H>,
    pub gen: G,
}

impl<T, H:IntegerHandle, G: PartialEq> PartialEq for GenId<T, H, G> {
    fn eq(&self, other: &GenId<T, H, G>) -> bool { self.id == other.id && self.gen == other.gen }
}

impl<T, H:IntegerHandle, G> ToIndex for GenId<T, H, G> {
    fn to_index(&self) -> usize { self.id.to_index() }
}


// ----------------------------------------------------------------------------------------- traits

pub trait FromIndex {  fn from_index(idx: usize) -> Self; }

pub trait ToIndex { fn to_index(&self) -> usize; }

pub trait Generation { fn get_gen(&self) -> u32; }

pub trait IntegerHandle : Copy + Clone
                        + Sub<Output=Self>
                        + Add<Output=Self>
                        + PartialEq + PartialOrd
                        //+ Bounded
                        + FromIndex + ToIndex {}

pub trait Identifier: Copy + FromIndex + ToIndex + PartialEq {
    type Handle: IntegerHandle;
}


impl ToIndex for u8 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u16 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u32 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u64 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for usize { fn to_index(&self) -> usize { *self } }

impl FromIndex for u8 { fn from_index(idx: usize) -> u8 { idx as u8 } }
impl FromIndex for u16 { fn from_index(idx: usize) -> u16 { idx as u16 } }
impl FromIndex for u32 { fn from_index(idx: usize) -> u32 { idx as u32 } }
impl FromIndex for u64 { fn from_index(idx: usize) -> u64 { idx as u64 } }
impl FromIndex for usize { fn from_index(idx: usize) -> usize { idx } }

impl Generation for u8  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u16  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u32  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u64  { fn get_gen(&self) -> u32 { *self as u32 } }

impl<T, H:Copy, G:Generation> Generation for GenId<T, H, G>  {
    fn get_gen(&self) -> u32 { self.gen.get_gen() }
}

impl IntegerHandle for u8 {}
impl IntegerHandle for u16 {}
impl IntegerHandle for u32 {}
impl IntegerHandle for u64 {}
impl IntegerHandle for usize {}


// ------------------------------------------------------------------------------------------ tests

#[test]
fn test_copy_id() {
    #[derive(Debug)]
    struct Foo;

    // check that Id is Copy
    let a: Id<Foo, u32> = Id::new(0);
    let b = a;
    let c = a;
    assert_eq!(b, c);

    // check that IdRange is Copy
    let r1 = IdRange {
        first: a,
        count: 10,
    };

    let r2 = r1;
    let r3 = r1;
    assert_eq!(r2, r3);
}
