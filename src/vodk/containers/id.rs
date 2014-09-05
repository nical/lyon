
pub trait Id {
    fn to_index(&self) -> uint;
    fn from_index(idx: uint) -> Self;
}
