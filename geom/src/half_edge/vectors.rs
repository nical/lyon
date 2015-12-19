
pub static X: usize = 0;
pub static Y: usize = 1;
pub static Z: usize = 2;
pub static W: usize = 3;

pub static U: usize = 0;
pub static V: usize = 1;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];

pub fn vec2_add(a: Vec2, b: Vec2) -> Vec2 { [a[X]+b[X], a[Y]+b[Y]] }
pub fn vec2_sub(a: Vec2, b: Vec2) -> Vec2 { [a[X]-b[X], a[Y]-b[Y]] }

//pub trait Attribute<AttributeType, AttributeName> {
pub trait Attribute<AttributeType> {
    fn get<'l>(&'l self) -> &'l AttributeType;
    fn get_mut<'l>(&'l mut self) -> &'l mut AttributeType;
}

impl<T> Attribute<T> for T {
    fn get(&self) -> &T { self }
    fn get_mut(&mut self) -> &mut T { self }
}

pub struct MtlId { pub handle: u32 }

pub trait MaterialId {
    fn mtl(&self) -> MtlId;
    //fn mtl_mut(&mut self) -> &mut MtlId;
}

pub trait Position2D {
    fn position(&self) -> [f32; 2];
    //fn position_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn x(&self) -> f32 { self.position()[X] }
    fn y(&self) -> f32 { self.position()[Y] }
}

pub trait Position3D {
    fn position(&self) -> [f32; 3];
    //fn position_mut(&mut self) -> &mut Vector3D<Self::Unit>;
    fn x(&self) -> f32 { self.position()[X] }
    fn y(&self) -> f32 { self.position()[Y] }
    fn z(&self) -> f32 { self.position()[Z] }
}

pub trait Position4D {
    fn position(&self) -> [f32; 4];
    //fn position_mut(&mut self) -> &mut Vector4D<Self::Unit>;
    fn x(&self) -> f32 { self.position()[X] }
    fn y(&self) -> f32 { self.position()[Y] }
    fn z(&self) -> f32 { self.position()[Z] }
    fn w(&self) -> f32 { self.position()[W] }
}

pub trait Normal2D {
    fn normal(&self) -> [f32; 2];
    //fn normal_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal()[X] }
    fn ny(&self) -> f32 { self.normal()[Y] }
}

pub trait Normal3D {
    fn normal(&self) -> [f32; 3];
    //fn normal_mut(&mut self) -> &mut Vector3D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal()[X] }
    fn ny(&self) -> f32 { self.normal()[Y] }
    fn nz(&self) -> f32 { self.normal()[Z] }
}

pub trait Normal4D {
    fn normal(&self) -> [f32; 4];
    //fn normal_mut(&mut self) -> &mut Vector4D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal()[X] }
    fn ny(&self) -> f32 { self.normal()[Y] }
    fn nz(&self) -> f32 { self.normal()[Z] }
    fn nw(&self) -> f32 { self.normal()[W] }
}

pub trait TextureCoordinates {
    fn uv(&self) -> [f32; 2];
    //fn uv_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn u(&self) -> f32 { self.uv()[X] }
    fn v(&self) -> f32 { self.uv()[Y] }
}

#[derive(Copy, Clone, Debug)]
pub struct Rgba<T> { r: T, g: T, b: T, a: T }

pub trait Color {
    type ScalarType: Copy;
    fn rgba(&self) -> &Rgba<Self::ScalarType>;
    //fn rgba_mut(&mut self) -> &mut Rgba<Self::ScalarType>;
    fn r(&self) -> Self::ScalarType { self.rgba().r }
    fn g(&self) -> Self::ScalarType { self.rgba().g }
    fn b(&self) -> Self::ScalarType { self.rgba().b }
    fn a(&self) -> Self::ScalarType { self.rgba().a }
}

impl Position2D for Vec2 { fn position(&self) -> Vec2 { *self } }

impl Position3D for Vec3 { fn position(&self) -> Vec3 { *self } }

impl Position4D for Vec4 { fn position(&self) -> Vec4 { *self } }

impl TextureCoordinates for Vec2 { fn uv(&self) -> Vec2 { *self } }
