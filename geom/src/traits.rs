
use vodk_math::vector::{ Vector2D, Vector3D };

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PositionAttribute;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NormalAttribute;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ColorAttribute;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UvAttribute;


//pub trait Attribute<AttributeType, AttributeName> {
pub trait Attribute<AttributeType> {
    fn get<'l>(&'l self) -> &'l AttributeType;
    fn get_mut<'l>(&'l mut self) -> &'l mut AttributeType;
}

impl<T> Attribute<T> for T {
    fn get(&self) -> &T { self }
    fn get_mut(&mut self) -> &mut T { self }
}

// impl<U> Attribute<Vector2D<U>, PositionAttribute> for Vector2D<U> {
//     fn get(&self) -> &Vector2D<U> { &self }
//     fn get_mut(&mut self) -> &mut Vector2D<U> { &mut self }
// }
// 
// impl<U> Attribute<Vector3D<U>, PositionAttribute> for Vector3D<U> {
//     fn get(&self) -> &Vector3D<U> { &self }
//     fn get_mut(&mut self) -> &mut Vector3D<U> { &mut self }
// }

