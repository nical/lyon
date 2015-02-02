
#[derive(Copy, Clone, Show, PartialEq, Eq)]
pub enum Format {
    R8G8B8A8,
    R8G8B8X8,
    B8G8R8A8,
    B8G8R8X8,
    A8,
    L8A8,
    A32,
}

pub struct Image {
    pub data: Vec<u8>,
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

pub struct ImageView<'l> {
    pub data: &'l [u8],
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}
