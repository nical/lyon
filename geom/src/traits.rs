
use vodk_math::vector::{ Vector2D, Vector3D, Vector4D };

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
    fn mtl_mut(&mut self) -> &mut MtlId;
}

pub trait Position2D {
    type Unit;
    fn position(&self) -> &Vector2D<Self::Unit>;
    fn position_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn x(&self) -> f32 { self.position().x }
    fn y(&self) -> f32 { self.position().y }
}

pub trait Position3D {
    type Unit;
    fn position(&self) -> &Vector3D<Self::Unit>;
    fn position_mut(&mut self) -> &mut Vector3D<Self::Unit>;
    fn x(&self) -> f32 { self.position().x }
    fn y(&self) -> f32 { self.position().y }
    fn z(&self) -> f32 { self.position().z }
}

pub trait Position4D {
    type Unit;
    fn position(&self) -> &Vector4D<Self::Unit>;
    fn position_mut(&mut self) -> &mut Vector4D<Self::Unit>;
    fn x(&self) -> f32 { self.position().x }
    fn y(&self) -> f32 { self.position().y }
    fn z(&self) -> f32 { self.position().z }
    fn w(&self) -> f32 { self.position().w }
}

pub trait Normal2D {
    type Unit;
    fn normal(&self) -> &Vector2D<Self::Unit>;
    fn normal_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal().x }
    fn ny(&self) -> f32 { self.normal().y }
}

pub trait Normal3D {
    type Unit;
    fn normal(&self) -> &Vector3D<Self::Unit>;
    fn normal_mut(&mut self) -> &mut Vector3D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal().x }
    fn ny(&self) -> f32 { self.normal().y }
    fn nz(&self) -> f32 { self.normal().z }
}

pub trait Normal4D {
    type Unit;
    fn normal(&self) -> &Vector4D<Self::Unit>;
    fn normal_mut(&mut self) -> &mut Vector4D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal().x }
    fn ny(&self) -> f32 { self.normal().y }
    fn nz(&self) -> f32 { self.normal().z }
    fn nw(&self) -> f32 { self.normal().w }
}

pub trait TextureCoordinates {
    type Unit;
    fn uv(&self) -> &Vector2D<Self::Unit>;
    fn uv_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn u(&self) -> f32 { self.uv().x }
    fn v(&self) -> f32 { self.uv().y }
}

#[derive(Copy, Clone, Debug)]
pub struct Rgba<T> { r: T, g: T, b: T, a: T }

pub trait Color {
    type ScalarType: Copy;
    fn rgba(&self) -> &Rgba<Self::ScalarType>;
    fn rgba_mut(&mut self) -> &mut Rgba<Self::ScalarType>;
    fn r(&self) -> Self::ScalarType { self.rgba().r }
    fn g(&self) -> Self::ScalarType { self.rgba().g }
    fn b(&self) -> Self::ScalarType { self.rgba().b }
    fn a(&self) -> Self::ScalarType { self.rgba().a }
}

impl<U> Position2D for Vector2D<U> {
    type Unit = U;
    fn position(&self) -> &Vector2D<U> { self }
    fn position_mut(&mut self) -> &mut Vector2D<U> { self }
}

impl<U> Position3D for Vector3D<U> {
    type Unit = U;
    fn position(&self) -> &Vector3D<U> { self }
    fn position_mut(&mut self) -> &mut Vector3D<U> { self }
}

impl<U> Position4D for Vector4D<U> {
    type Unit = U;
    fn position(&self) -> &Vector4D<U> { self }
    fn position_mut(&mut self) -> &mut Vector4D<U> { self }
}

impl<U> TextureCoordinates for Vector2D<U> {
    type Unit = U;
    fn uv(&self) -> &Vector2D<U> { self }
    fn uv_mut(&mut self) -> &mut Vector2D<U> { self }
}

