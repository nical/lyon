use std::default::Default;

pub trait FromIndex {
    fn from_index(idx: usize) -> Self;
}

pub trait ToIndex {
    fn to_index(&self) -> usize;
}

pub trait Generation {
    fn get_gen(&self) -> u32;
}

pub trait Invalid {
    fn is_valid(&self) -> bool;
}

#[derive(Clone, Show)]
pub struct GenId<T, H, G> {
    pub handle: H,
    pub gen: G,
}

#[derive(Clone, Show)]
pub struct Id<T, H> {
    pub handle: H,
}

impl<T, H, G: Generation> Invalid for GenId<T, H, G> {
    fn is_valid(&self) -> bool { self.get_gen() != 0 }
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

impl ToIndex for u8 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u16 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u32 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u64 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for usize { fn to_index(&self) -> usize { *self } }

impl<T, H:ToIndex, G> ToIndex for GenId<T, H, G> {
    fn to_index(&self) -> usize { self.handle.to_index() }
}

impl<T, H:ToIndex> ToIndex for Id<T, H> {
    fn to_index(&self) -> usize { self.handle.to_index() }
}

impl FromIndex for u8 { fn from_index(idx: usize) -> u8 { idx as u8 } }
impl FromIndex for u16 { fn from_index(idx: usize) -> u16 { idx as u16 } }
impl FromIndex for u32 { fn from_index(idx: usize) -> u32 { idx as u32 } }
impl FromIndex for u64 { fn from_index(idx: usize) -> u64 { idx as u64 } }
impl FromIndex for usize { fn from_index(idx: usize) -> usize { idx } }

impl<T, H:FromIndex> FromIndex for Id<T, H> {
    fn from_index(idx: usize) -> Id<T, H> { Id { handle: FromIndex::from_index(idx) } }
}

impl Generation for u8  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u16  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u32  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u64  { fn get_gen(&self) -> u32 { *self as u32 } }

impl<T, H, G: Generation> Generation for GenId<T, H, G>  { fn get_gen(&self) -> u32 { self.gen.get_gen() } }

//impl<T, H> Generation for GenId<T, H, u8>  { fn get_gen(&self) -> u32 { self.gen as u32 } }
//impl<T, H> Generation for GenId<T, H, u16> { fn get_gen(&self) -> u32 { self.gen as u32 } }
//impl<T, H> Generation for GenId<T, H, u32> { fn get_gen(&self) -> u32 { self.gen as u32 } }
//impl<T, H> Generation for GenId<T, H, u64> { fn get_gen(&self) -> u32 { self.gen as u32 } }
