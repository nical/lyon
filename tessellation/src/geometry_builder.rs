//! # Geometry builder
//!
//! Tools to help with populating vertex and index buffers.
//!
//! ## Overview
//!
//! TODO
//!
//! ## Example
//!
//! ```
//! // This example sets up a simple function that generates the vertices and indices for
//! // colored quads, using some of the tools provided in this crate.
//! // Note that for simplicity in this example we use [f32; 2] to represent positions,
//! // while most of the more advanced tessellator code use euclid points.
//! use lyon_tessellation::geometry_builder::*;
//!
//! // Define our vertex type.
//! #[derive(Copy, Clone, PartialEq, Debug)]
//! struct Vertex2d {
//!   position: [f32; 2],
//!   color: [f32; 4],
//! }
//!
//! // The vertex constructor. This is the object that will be used to create vertices from
//! // a position provided by the geometry builder. In this specific case the vertex constructor
//! // stores a constant color which will be applied to all vertices.
//! struct WithColor([f32; 4]);
//!
//! // Implement the VertexConstructor trait accordingly. WithColor takes a [f32; 2] position as
//! // input and returns a Vertex2d.
//! impl VertexConstructor<[f32; 2], Vertex2d> for WithColor {
//!     fn new_vertex(&mut self, pos: [f32; 2]) -> Vertex2d {
//!         Vertex2d {
//!             position: pos,
//!             color: self.0
//!         }
//!     }
//! }
//!
//! // A typical "algortihm" that generates some geometry, in this case a simple axis-aligned quad.
//! // Returns a structure containing the number of vertices and number of indices allocated during
//! // the execution of this method.
//! fn make_quad<Builder: GeometryBuilder<[f32; 2]>>(
//!     top_left: [f32; 2],
//!     size: [f32; 2],
//!     builder: &mut Builder
//! ) -> Count {
//!     builder.begin_geometry();
//!     // Create the vertices...
//!     let a = builder.add_vertex(top_left);
//!     let b = builder.add_vertex([top_left[0] + size[0], top_left[1]]);
//!     let c = builder.add_vertex([top_left[0] + size[0], top_left[1] + size[1]]);
//!     let d = builder.add_vertex([top_left[0], top_left[1] + size[1]]);
//!     // ...and create triangle form these points. a, b, c, and d are relative offsets in the
//!     // vertex buffer.
//!     builder.add_triangle(a, b, c);
//!     builder.add_triangle(a, c, d);
//!     return builder.end_geometry();
//! }
//!
//! // Allocate a vertex buffer and an index buffer. This is typically what we would want to
//! // send to the GPU for rendering.
//! let mut buffers: VertexBuffers<Vertex2d> = VertexBuffers::new();
//!
//! // Finally, generate the geometry using the function we created above to make a red square...
//! let red = [1.0, 0.0, 0.0, 1.0];
//! make_quad([0.0, 0.0], [1.0, 1.0], &mut vertex_builder(&mut buffers, WithColor(red)));
//!
//! // ...an a green one.
//! let green = [0.0, 1.0, 0.0, 1.0];
//! make_quad([2.0, 0.0], [1.0, 1.0], &mut vertex_builder(&mut buffers, WithColor(green)));
//!
//! println!("The generated vertices are: {:?}.", &buffers.vertices[..]);
//! println!("The generated indices are: {:?}.", &buffers.indices[..]);
//! ```


use std::marker::PhantomData;
use std::ops::Add;

pub type Index = u16;

/// A virtual vertex offset in a geometry.
///
/// The VertexIds are only valid between GeometryBuilder::begin_geometry and
/// GeometryBuilder::end_geometry. GeometryBuilder implementations typically be translate
/// the ids internally so that first VertexId after begin_geometry is zero.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VertexId(pub u16);

impl VertexId {
    pub fn offset(&self) -> u16 { self.0 }
}

/// An interface separating tessellators and other geometry generation algorthms from the
/// actual vertex construction.
pub trait GeometryBuilder<Input> {
    /// Called at the beginning of a generation.
    ///
    /// end_geometry must be called before begin_geometry is called again.
    fn begin_geometry(&mut self);

    /// Called at the end of a generation.
    /// Returns the number of vertices and indices added since the last time begin_geometry was
    /// called.
    fn end_geometry(&mut self) -> Count;

    /// Inserts a vertex, providing its position, and optionally a normal.
    /// Retuns a vertex id that is only valid between begin_geometry and end_geometry.
    ///
    /// This method can only be called between begin_geometry and end_geometry.
    fn add_vertex(&mut self, vertex: Input) -> VertexId;

    /// Insert a triangle made of vertices that were added after the last call to begin_geometry.
    ///
    /// This method can only be called between begin_geometry and end_geometry.
    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId);

    /// abort_geometry is called instead of end_geometry if an error occured while producing
    /// the geometry and we won't be able to finish.
    ///
    /// The implementation is expected to discard the geometry that was generated since the last
    /// time begin_geometry was called, and to remain in a usable state.
   fn abort_geometry(&mut self);
}

/// An extension to GeometryBuilder that can handle quadratic bezier segments.
pub trait BezierGeometryBuilder<Input> : GeometryBuilder<Input> {
    /// Insert a quadratic bezier curve.
    /// The interrior is on the right side of the curve.
    ///
    /// This method can only be called between begin_geometry and end_geometry.
    fn add_quadratic_bezier(&mut self, from: VertexId, to: VertexId, ctrl: Input);
}

/// Structure that holds the vertex and index data.
///
/// Usually writen into though temporary BuffersBuilder objects.
pub struct VertexBuffers<VertexType> {
    pub vertices: Vec<VertexType>,
    pub indices: Vec<Index>,
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

/// A temporary view on a VertexBuffers object which facilitate the population of vertex and index
/// data.
///
/// BuffersBuilders record the vertex offset from when they are created so that algorithms using
/// them don't need to worry about offsetting indices if some geometry was added beforehand. This
/// means that from the point of view of a BuffersBuilder user, the first added vertex is at always
/// offset at the offset 0 and VertexBuilfer takes care of translating indices adequately.
///
/// Often, algorithms are built to generate vertex positions without knowledge of eventual other
/// vertex attributes. The VertexConstructor does the translation from generic Input to VertexType.
/// If your logic generates the actual vertex type directly, you can use the SimpleBuffersBuilder
/// convenience typedef.
pub struct BuffersBuilder<'l,
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

impl<'l, VertexType: 'l, Input, Ctor: VertexConstructor<Input, VertexType>>
BuffersBuilder<'l, VertexType, Input, Ctor> {
    pub fn new(buffers: &'l mut VertexBuffers<VertexType>, ctor: Ctor) -> BuffersBuilder<'l, VertexType, Input, Ctor> {
        let vertex_offset = buffers.vertices.len() as Index;
        let index_offset = buffers.indices.len() as Index;
        BuffersBuilder {
            buffers: buffers,
            vertex_offset: vertex_offset,
            index_offset: index_offset,
            vertex_constructor: ctor,
            _marker: PhantomData
        }
    }
}

/// Creates a BuffersBuilder.
pub fn vertex_builder<'l,
    VertexType,
    Input,
    Ctor: VertexConstructor<Input, VertexType>
> (buffers: &'l mut VertexBuffers<VertexType>, ctor: Ctor) -> BuffersBuilder<'l, VertexType, Input, Ctor> {
    let vertex_offset = buffers.vertices.len() as Index;
    let index_offset = buffers.indices.len() as Index;
    BuffersBuilder {
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

/// A BuffersBuilder that takes the actual vertex type as input.
pub type SimpleBuffersBuilder<'l, VertexType> = BuffersBuilder<'l, VertexType, VertexType, Identity>;

/// Creates a SimpleBuffersBuilder.
pub fn simple_builder<'l, VertexType> (buffers: &'l mut VertexBuffers<VertexType>) -> SimpleBuffersBuilder<'l, VertexType> {
    let vertex_offset = buffers.vertices.len() as Index;
    let index_offset = buffers.indices.len() as Index;
    BuffersBuilder {
        buffers: buffers,
        vertex_offset: vertex_offset,
        index_offset: index_offset,
        vertex_constructor: Identity,
        _marker: PhantomData
    }
}

/// Number of vertices and indices added during the tessellation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

impl<'l, VertexType, Input, Ctor> GeometryBuilder<Input>
for BuffersBuilder<'l, VertexType, Input, Ctor>
where VertexType:'l + Clone, Ctor: VertexConstructor<Input, VertexType> {

    fn begin_geometry(&mut self) {
        self.vertex_offset = self.buffers.vertices.len() as Index;
        self.index_offset = self.buffers.indices.len() as Index;
    }

    fn end_geometry(&mut self) -> Count {
        return Count {
            vertices: self.buffers.vertices.len() as u32 - self.vertex_offset as u32,
            indices: self.buffers.indices.len() as u32 - self.index_offset as u32
        };
    }

    fn add_vertex(&mut self, v: Input) -> VertexId {
        self.buffers.vertices.push(self.vertex_constructor.new_vertex(v));
        return VertexId(self.buffers.vertices.len() as Index - 1 - self.vertex_offset)
    }

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        self.buffers.indices.push(a.offset() + self.vertex_offset);
        self.buffers.indices.push(b.offset() + self.vertex_offset);
        self.buffers.indices.push(c.offset() + self.vertex_offset);
    }

    fn abort_geometry(&mut self) {
        self.buffers.vertices.truncate(self.vertex_offset as usize);
        self.buffers.indices.truncate(self.index_offset as usize);
    }
}


impl<'l, VertexType, Input, Ctor> BezierGeometryBuilder<Input>
for BuffersBuilder<'l, VertexType, Input, Ctor>
where VertexType:'l + Clone, Ctor: VertexConstructor<Input, VertexType> {
    fn add_quadratic_bezier(&mut self, _from: VertexId, _to: VertexId, _ctrl: Input) {
        unimplemented!();
    }
}

#[test]
fn test_simple_quad() {
    // Same as the example from the documentation with some assertions.

    #[derive(Copy, Clone, PartialEq, Debug)]
    struct Vertex2d {
      position: [f32; 2],
      color: [f32; 4],
    }

    struct WithColor([f32; 4]);

    impl VertexConstructor<[f32; 2], Vertex2d> for WithColor {
        fn new_vertex(&mut self, pos: [f32; 2]) -> Vertex2d {
            Vertex2d {
                position: pos,
                color: self.0
            }
        }
    }

    // A typical "algortihm" that generates some geometry, in this case a simple axis-aligned quad.
    fn add_quad<Builder: GeometryBuilder<[f32; 2]>>(
        top_left: [f32; 2],
        size: [f32; 2],
        mut out: Builder
    ) -> Count {
        out.begin_geometry();
        let a = out.add_vertex(top_left);
        let b = out.add_vertex([top_left[0] + size[0], top_left[1]]);
        let c = out.add_vertex([top_left[0] + size[0], top_left[1] + size[1]]);
        let d = out.add_vertex([top_left[0], top_left[1] + size[1]]);
        out.add_triangle(a, b, c);
        out.add_triangle(a, c, d);
        let count = out.end_geometry();
        // offsets always start at zero after begin_geometry, regardless of where we are
        // in the actual vbo. Algorithms can rely on this property when generating indices.
        assert_eq!(a.offset(), 0);
        assert_eq!(b.offset(), 1);
        assert_eq!(c.offset(), 2);
        assert_eq!(d.offset(), 3);
        assert_eq!(count.vertices, 4);
        assert_eq!(count.indices, 6);

        return count;
    }


    let mut buffers: VertexBuffers<Vertex2d> = VertexBuffers::new();
    let red = [1.0, 0.0, 0.0, 1.0];
    let green = [0.0, 1.0, 0.0, 1.0];

    add_quad([0.0, 0.0], [1.0, 1.0], vertex_builder(&mut buffers, WithColor(red)));

    assert_eq!(buffers.vertices[0], Vertex2d { position: [0.0, 0.0], color: red });
    assert_eq!(buffers.vertices[1], Vertex2d { position: [1.0, 0.0], color: red });
    assert_eq!(buffers.vertices[3], Vertex2d { position: [0.0, 1.0], color: red });
    assert_eq!(buffers.vertices[2], Vertex2d { position: [1.0, 1.0], color: red });
    assert_eq!(&buffers.indices[..], &[0, 1, 2, 0, 2, 3]);

    add_quad([10.0, 10.0], [1.0, 1.0], vertex_builder(&mut buffers, WithColor(green)));

    assert_eq!(buffers.vertices[4], Vertex2d { position: [10.0, 10.0], color: green });
    assert_eq!(buffers.vertices[5], Vertex2d { position: [11.0, 10.0], color: green });
    assert_eq!(buffers.vertices[6], Vertex2d { position: [11.0, 11.0], color: green });
    assert_eq!(buffers.vertices[7], Vertex2d { position: [10.0, 11.0], color: green });
    assert_eq!(&buffers.indices[..], &[0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
}
