//! Tools to help with populating vertex and index buffers

use std::marker::PhantomData;
use std::ops::Add;
use math::{ Point, Vec2 };
use super::{ VertexId, vertex_id };

pub type Index = u16;

/// Structure that holds the vertex and index data.
///
/// Usually writen into though temporary VertexBuilder objects.
pub struct VertexBuffers<VertexType> {
    pub vertices: Vec<VertexType>,
    pub indices: Vec<Index>,
}

/// A trait that VertexBuilder implements exposing the methods that algorithms generating geometry
/// need, and hiding the generic parameters they should not care about.
pub trait VertexBufferBuilder<Input> {

    fn push_vertex(&mut self, p: Input) -> Index;

    fn push_indices(&mut self, a: Index, b: Index, c: Index);

    fn num_vertices(&self) -> usize;

    /// Set the vertex offset to the index of the next vertex to be pushed.
    ///
    /// Use this when a VertexBuilder is passed by reference to a succession of functions at the
    /// beginning of each of these functions (if they expect indices to start at zero).
    ///
    /// Return the offsets of the first vertex and th first index.
    fn begin(&mut self);

    /// Return the ranges of vertices and indices added since we last called begin.
    fn end(&mut self) -> (Range, Range);
}

impl<VertexType> VertexBuffers<VertexType> {
    /// Constructor
    pub fn new() -> VertexBuffers<VertexType> { VertexBuffers::with_capacity(512, 1024) }

    /// Constructor
    pub fn with_capacity(num_vertices: usize, num_indices: usize) -> VertexBuffers<VertexType> {
        VertexBuffers {
            vertices: Vec::with_capacity(num_vertices),
            indices: Vec::with_capacity(num_indices),
        }
    }
}

impl<VertexType> VertexBufferBuilder<VertexType> for VertexBuffers<VertexType> {

    fn push_vertex(&mut self, p: VertexType) -> Index {
        self.vertices.push(p);
        return self.vertices.len() as Index - 1;
    }

    fn push_indices(&mut self, a: Index, b: Index, c: Index) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    fn num_vertices(&self) -> usize { self.vertices.len() }

    fn begin(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    fn end(&mut self) -> (Range, Range) {
        return (
            Range { first: 0, count: self.vertices.len() as Index },
            Range { first: 0, count: self.indices.len() as Index },
        );
    }
}

/// A temporary view on a VertexBuffers object which facilitate the population of vertex and index
/// data.
///
/// VertexBuilders record the vertex offset from when they are created so that algorithms using
/// them don't need to worry about offsetting indices if some geometry was added beforehand. This
/// means that from the point of view of a VertexBuilder user, the first added vertex is at always
/// offset at the offset 0 and VertexBuilfer takes care of translating indices adequately.
///
/// Often, algorithms are built to generate vertex positions without knowledge of eventual other
/// vertex attributes. The VertexConstructor does the translation from generic Input to VertexType.
/// If your logic generates the actual vertex type directly, you can use the SimpleVertexBuilder
/// convenience typedef.
pub struct VertexBuilder<'l,
    VertexType: 'l,
    Input,
    Ctor: VertexConstructor<Input, VertexType>
> {
    buffers: &'l mut VertexBuffers<VertexType>,
    vertex_offset: Index,
    index_offset: Index,
    vertex_constructor: Ctor,
    _marker: PhantomData<Input>
}

impl<'l,
    VertexType,
    Input,
    Ctor: VertexConstructor<Input, VertexType>
> VertexBufferBuilder<Input> for VertexBuilder<'l, VertexType, Input, Ctor> {

    fn push_vertex(&mut self, p: Input) -> Index {
        self.buffers.push_vertex(self.vertex_constructor.new_vertex(p)) - self.vertex_offset
    }

    fn push_indices(&mut self, a: Index, b: Index, c: Index) {
        self.buffers.push_indices(
            a + self.vertex_offset,
            b + self.vertex_offset,
            c + self.vertex_offset
        );
    }

    fn num_vertices(&self) -> usize { self.buffers.num_vertices() }

    fn begin(&mut self) {
        self.vertex_offset = self.buffers.vertices.len() as Index;
        self.index_offset = self.buffers.indices.len() as Index;
    }

    fn end(&mut self) -> (Range, Range) {
        let ranges = (
            Range { first: self.vertex_offset, count: self.buffers.vertices.len() as Index - self.vertex_offset },
            Range { first: self.index_offset, count: self.buffers.indices.len() as Index - self.index_offset }
        );
        return ranges;
    }
}

/// Constructor
pub fn vertex_builder<'l,
    VertexType,
    Input,
    Ctor: VertexConstructor<Input, VertexType>
> (buffers: &'l mut VertexBuffers<VertexType>, ctor: Ctor) -> VertexBuilder<'l, VertexType, Input, Ctor> {
    let vertex_offset = buffers.num_vertices() as Index;
    let index_offset = buffers.indices.len() as Index;
    VertexBuilder {
        buffers: buffers,
        vertex_offset: vertex_offset,
        index_offset: index_offset,
        vertex_constructor: ctor,
        _marker: PhantomData
    }
}

/// Creates vertex values
///
/// Typically will take a vertex position as Input and will build a full vertex value from it, swee
/// the test example at the bottom of this file.
pub trait VertexConstructor<Input, VertexType> {
    fn new_vertex(&mut self, input: Input) -> VertexType;
}

/// A dummy vertex constructor that just forwards its inputs.
pub struct Identity;
impl<T> VertexConstructor<T, T> for Identity {
    fn new_vertex(&mut self, input: T) -> T { input }
}

/// A VertexBuilder that takes the actual vertex type as input.
pub type SimpleVertexBuilder<'l, VertexType> = VertexBuilder<'l, VertexType, VertexType, Identity>;

/// Constructor
pub fn simple_vertex_builder<'l, VertexType> (buffers: &'l mut VertexBuffers<VertexType>) -> SimpleVertexBuilder<'l, VertexType> {
    let vertex_offset = buffers.num_vertices() as Index;
    let index_offset = buffers.indices.len() as Index;
    VertexBuilder {
        buffers: buffers,
        vertex_offset: vertex_offset,
        index_offset: index_offset,
        vertex_constructor: Identity,
        _marker: PhantomData
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Range {
    pub first: Index,
    pub count: Index,
}

impl Range {
    pub fn new(first: Index, count: Index) -> Range { Range { first: first, count: count } }

    pub fn contains(&self, other: &Range) -> bool {
        self.first <= other.first && self.first + self.count >= other.first + other.count
    }
    pub fn intersects(&self, other: &Range) -> bool {
        self.first <= other.first + self.count && self.first + self.count >= other.first
    }
    pub fn shrink_left(&mut self, amount: Index) {
        self.count -= amount;
        self.first += amount;
    }
    pub fn shrink_right(&mut self, amount: Index) {
        self.count -= amount;
    }
    pub fn expand_left(&mut self, amount: Index) {
        self.count += amount;
        self.first -= amount;
    }
    pub fn expand_right(&mut self, amount: Index) {
        self.count += amount;
    }
    pub fn is_left_adjacent_to(&self, other: &Range) -> bool {
        self.first + self.count == other.first
    }
    pub fn is_right_adjacent_to(&self, other: &Range) -> bool {
        other.is_left_adjacent_to(self)
    }
    pub fn is_adjacent_to(&self, other: &Range) -> bool {
        self.is_left_adjacent_to(other) || other.is_right_adjacent_to(other)
    }

    pub fn is_left_of(&self, other: &Range) -> bool {
        self.first < other.first
    }

    pub fn right_most(&self) -> Index {
        self.first + self.count
    }
}

#[cfg(test)]
#[derive(PartialEq, Debug)]
struct Vertex2d {
  position: [f32; 2],
  color: [f32; 4],
}

#[cfg(test)]
struct Vertex2dConstructor {
    color: [f32; 4]
}

#[cfg(test)]
impl VertexConstructor<[f32; 2], Vertex2d> for Vertex2dConstructor {
    fn new_vertex(&mut self, pos: [f32; 2]) -> Vertex2d {
        Vertex2d {
            position: pos,
            color: self.color
        }
    }
}

/// A slightly higher level interface than VertexBufferBuilder that is tailored for
/// the tessellator's use-case.
///
/// It may end up replacing VertexBufferBuilder completely.
pub trait GeometryBuilder {
    fn begin_geometry(&mut self);
    fn end_geometry(&mut self) -> Count;
    fn add_vertex(&mut self, pos: Point, normal: Option<Vec2>) -> VertexId;
    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId);
    fn add_quadratic_curve(&mut self, from: VertexId, to: VertexId, ctrl: Point);
}

/// Number of vertices and indices added during the tesselation.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Count {
    vertices: u32,
    indices: u32,
}

impl Add for Count {
    type Output = Count;
    fn add(self, other: Count) -> Count {
        Count {
            vertices: self.vertices + other.vertices,
            indices: self.indices + other.indices,
        }
    }
}

impl<'l, VertexType:'l, Ctor: VertexConstructor<Point, VertexType>>
GeometryBuilder for VertexBuilder<'l, VertexType, Point, Ctor> {
    fn begin_geometry(&mut self) {
        self.begin();
    }

    fn end_geometry(&mut self) -> Count {
        let (vertices, indices) = self.end();
        return Count { vertices: vertices.count as u32, indices: indices.count as u32 };
    }

    fn add_vertex(&mut self, pos: Point, _normal: Option<Vec2>) -> VertexId {
        vertex_id(self.push_vertex(pos))
    }

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        self.push_indices(a.handle, b.handle, c.handle);
    }

    fn add_quadratic_curve(&mut self, _from: VertexId, _to: VertexId, _ctrl: Point) {
        unimplemented!();
    }
}

// A typical "algortihm" that generates some geometry, in this case a simple axis-aligned quad.
#[cfg(test)]
fn add_quad<Builder: VertexBufferBuilder<[f32; 2]>>(
    top_left: [f32; 2],
    size:[f32; 2],
    mut out: Builder
) -> (Range, Range) {
    out.begin();
    let a = out.push_vertex(top_left);
    let b = out.push_vertex([top_left[0]+size[0], top_left[1]]);
    let c = out.push_vertex([top_left[0]+size[0], top_left[1]+size[1]]);
    let d = out.push_vertex([top_left[0], top_left[1]+size[1]]);
    out.push_indices(a, b, c);
    out.push_indices(a, c, d);
    let (vertices, indices) = out.end();
    // offsets always start at zero after begin_geometry, regardless of where we are
    // in the actual vbo. Algorithms can rely on this property when generating indices.
    assert_eq!(a, 0);
    assert_eq!(b, 1);
    assert_eq!(c, 2);
    assert_eq!(d, 3);
    assert_eq!(vertices.count, 4);
    assert_eq!(indices.count, 6);

    return (vertices, indices);
}

#[test]
fn test_simple_quad() {
    let mut buffers: VertexBuffers<Vertex2d> = VertexBuffers::new();
    let red = [1.0, 0.0, 0.0, 1.0];
    let green = [0.0, 1.0, 0.0, 1.0];

    add_quad([0.0, 0.0], [1.0, 1.0], vertex_builder(&mut buffers, Vertex2dConstructor { color: red }));

    assert_eq!(buffers.vertices[0], Vertex2d { position: [0.0, 0.0], color: red });
    assert_eq!(buffers.vertices[1], Vertex2d { position: [1.0, 0.0], color: red });
    assert_eq!(buffers.vertices[3], Vertex2d { position: [0.0, 1.0], color: red });
    assert_eq!(buffers.vertices[2], Vertex2d { position: [1.0, 1.0], color: red });
    assert_eq!(&buffers.indices[..], &[0, 1, 2, 0, 2, 3]);

    add_quad([10.0, 10.0], [1.0, 1.0], vertex_builder(&mut buffers, Vertex2dConstructor { color: green }));

    assert_eq!(buffers.vertices[4], Vertex2d { position: [10.0, 10.0], color: green });
    assert_eq!(buffers.vertices[5], Vertex2d { position: [11.0, 10.0], color: green });
    assert_eq!(buffers.vertices[6], Vertex2d { position: [11.0, 11.0], color: green });
    assert_eq!(buffers.vertices[7], Vertex2d { position: [10.0, 11.0], color: green });
    assert_eq!(&buffers.indices[..], &[0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
}
