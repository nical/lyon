//! A simple module that helps with populating vertex and index buffers

use std::marker::PhantomData;


/// Structure that holds the vertex and index data.
///
/// Usually writen into though temporary VertexBuilder objects.
pub struct VertexBuffers<VertexType> {
    pub vertices: Vec<VertexType>,
    pub indices: Vec<u16>,
}

/// A trait that VertexBuilder implements exposing the methods that algorithms generating geometry
/// need, and hiding the generic parameters they should not care about.
pub trait VertexBufferBuilder<Input> {

    fn push_vertex(&mut self, p: Input) -> u16;

    fn push_indices(&mut self, a: u16, b: u16, c: u16);

    fn num_vertices(&self) -> usize;

    /// Set the vertex offset to the index of the next vertex to be pushed.
    ///
    /// Use this when a VertexBuilder is passed by reference to a succession of functions at the
    /// beginning of each of these functions (if they expect indices to start at zero).
    fn begin_geometry(&mut self);
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

    fn push_vertex(&mut self, p: VertexType) -> u16 {
        self.vertices.push(p);
        return self.vertices.len() as u16 - 1;
    }

    fn push_indices(&mut self, a: u16, b: u16, c: u16) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    fn num_vertices(&self) -> usize { self.vertices.len() }

    fn begin_geometry(&mut self) {
        self.vertices.clear();
        self.indices.clear();
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
    vertex_offset: u16,
    vertex_constructor: Ctor,
    _marker: PhantomData<Input>
}

impl<'l,
    VertexType,
    Input,
    Ctor: VertexConstructor<Input, VertexType>
> VertexBufferBuilder<Input> for VertexBuilder<'l, VertexType, Input, Ctor> {

    fn push_vertex(&mut self, p: Input) -> u16 {
        self.buffers.push_vertex(self.vertex_constructor.new_vertex(p)) - self.vertex_offset
    }

    fn push_indices(&mut self, a: u16, b: u16, c: u16) {
        self.buffers.push_indices(
            a + self.vertex_offset,
            b + self.vertex_offset,
            c + self.vertex_offset
        );
    }

    fn num_vertices(&self) -> usize { self.buffers.num_vertices() }

    fn begin_geometry(&mut self) { self.vertex_offset = self.buffers.vertices.len() as u16 }
}

/// Constructor
pub fn vertex_builder<'l,
    VertexType,
    Input,
    Ctor: VertexConstructor<Input, VertexType>
> (buffers: &'l mut VertexBuffers<VertexType>, ctor: Ctor) -> VertexBuilder<'l, VertexType, Input, Ctor> {
    let offset = buffers.num_vertices() as u16;
    VertexBuilder {
        buffers: buffers,
        vertex_offset: offset,
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
    let offset = buffers.num_vertices() as u16;
    VertexBuilder {
        buffers: buffers,
        vertex_offset: offset,
        vertex_constructor: Identity,
        _marker: PhantomData
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

// A typical "algortihm" that generates some geometry, in this case a simple axis-aligned quad.
#[cfg(test)]
fn add_quad<Builder: VertexBufferBuilder<[f32; 2]>>(
    top_left: [f32; 2],
    size:[f32; 2],
    mut out: Builder
) {
    out.begin_geometry();
    let a = out.push_vertex(top_left);
    let b = out.push_vertex([top_left[0]+size[0], top_left[1]]);
    let c = out.push_vertex([top_left[0]+size[0], top_left[1]+size[1]]);
    let d = out.push_vertex([top_left[0], top_left[1]+size[1]]);
    out.push_indices(a, b, c);
    out.push_indices(a, c, d);
    // offsets always start at zero after begin_geometry, regardless of where we are
    // in the actual vbo. Algorithms can rely on this property when generating indices.
    assert_eq!(a, 0);
    assert_eq!(b, 1);
    assert_eq!(c, 2);
    assert_eq!(d, 3);
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
