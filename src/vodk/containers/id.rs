use std::default::Default;

pub trait FromIndex {
    fn from_index(idx: uint) -> Self;
}

pub trait ToIndex {
    fn to_index(&self) -> uint;
}

#[deriving(Clone, Show)]
pub struct GenId<T, H, G> {
    pub handle: H,
    pub gen: G,
}

#[deriving(Clone, Show)]
pub struct Id<T, H> {
    pub handle: H,
}

impl<T, H: PartialEq, G: PartialEq> PartialEq for GenId<T, H, G> {
    fn eq(&self, other: &GenId<T, H, G>) -> bool { self.handle == other.handle && self.gen == other.gen }
    fn ne(&self, other: &GenId<T, H, G>) -> bool { self.handle != other.handle || self.gen != other.gen }
}

impl<T, H: PartialEq> PartialEq for Id<T, H> {
    fn eq(&self, other: &Id<T, H>) -> bool { self.handle == other.handle }
    fn ne(&self, other: &Id<T, H>) -> bool { self.handle != other.handle }
}

impl<T, H: Default, G: Default> Default for GenId<T, H, G> {
    fn default() -> GenId<T,H,G> { GenId { handle: Default::default(), gen: Default::default() } }
}

impl<T, H: Default> Default for Id<T, H> {
    fn default() -> Id<T,H> { Id { handle: Default::default() } }
}

impl ToIndex for u8 { fn to_index(&self) -> uint { *self as uint } }
impl ToIndex for u16 { fn to_index(&self) -> uint { *self as uint } }
impl ToIndex for u32 { fn to_index(&self) -> uint { *self as uint } }
impl ToIndex for u64 { fn to_index(&self) -> uint { *self as uint } }
impl ToIndex for uint { fn to_index(&self) -> uint { *self } }

impl<T, H:ToIndex, G> ToIndex for GenId<T, H, G> {
    fn to_index(&self) -> uint { self.handle.to_index() }
}

impl<T, H:ToIndex> ToIndex for Id<T, H> {
    fn to_index(&self) -> uint { self.handle.to_index() }
}

impl FromIndex for u8 { fn from_index(idx: uint) -> u8 { idx as u8 } }
impl FromIndex for u16 { fn from_index(idx: uint) -> u16 { idx as u16 } }
impl FromIndex for u32 { fn from_index(idx: uint) -> u32 { idx as u32 } }
impl FromIndex for u64 { fn from_index(idx: uint) -> u64 { idx as u64 } }
impl FromIndex for uint { fn from_index(idx: uint) -> uint { idx } }

impl<T, H:FromIndex> FromIndex for Id<T, H> {
    fn from_index(idx: uint) -> Id<T, H> { Id { handle: FromIndex::from_index(idx) } }
}
