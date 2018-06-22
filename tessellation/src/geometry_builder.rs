//! Tools to help with generating vertex and index buffers.
//!
//! ## Overview
//!
//! While it would be possible for the tessellation algorithms to manually generate vertex
//! and index buffers with a certain layout, it would mean that most code using the tessellators
//! would need to copy and convert all generated vertices in order to have their own vertex
//! layout, or even several vertex layouts, which is a very common use-case.
//!
//! In order to provide flexibility with the generation of geometry, this module provides with
//! the [`GeometryBuilder`](trait.GeometryBuilder.html) and its extension the
//! [`BezierGeometryBuilder`](trait.BezierGeometryBuilder.html) trait. The former exposes
//! the methods to facilitate adding vertices and triangles. The latter adds a method to
//! specifically handle quadratic bezier curves. Quadratic bezier curves have interesting
//! properties that make them a lot easier to render than most types of curves and we want
//! to have the option to handle them separately in the renderer.
//!
//! See the [Rendering curves](https://github.com/nical/lyon/wiki/Experiments#rendering-curves)
//! section in the project's wiki for more details about the advantages of handling quadratic
//! bezier curves separately in the tessellator and the renderer.
//!
//! This modules provides with a basic implementation of these traits through the following types:
//!
//! * The struct [`VertexBuffers<T>`](struct.VertexBuffers.html) is a simple pair of vectors of u32
//!   indices and T (generic parameter) vertices.
//! * The struct [`BuffersBuilder`](struct.BuffersBuilder.html) which implements
//!   [`BezierGeometryBuilder`](trait.BezierGeometryBuilder.html) and writes into a
//!   [`VertexBuffers`](struct.VertexBuffers.html).
//! * The trait [`VertexConstructor`](trait.VertexConstructor.html) used by
//!   [`BuffersBuilder`](struct.BuffersBuilder.html) in order to generate any vertex type. In the
//!   example below, a struct `WithColor` implements the `VertexConstructor` trait in order to
//!   create vertices composed of a 2d position and a color value from an input 2d position.
//!   This separates the construction of vertex values from the assembly of the vertex buffers.
//!   Another, simpler example of vertex constructor is the [`Identity`](struct.Identity.html)
//!   constructor which just returns its input, untransformed.
//!   `VertexConstructor<Input, Ouput>` is implemented for all closures `Fn(Input) -> Output`.
//!
//! Geometry builders are a practical way to add one last step to the tessellation pipeline,
//! such as applying a transform or clipping the geometry.
//!
//! While this is module designed to facilitate the generation of vertex buffers and index
//! buffers, nothing prevents a given GeometryBuilder implementation to only generate a
//! vertex buffer without indices, or write into a completely different format.
//! These builder traits are at the end of the tessellation pipelines and are meant for
//! users of this crate to be able to adapt the output of the tessellators to their own
//! needs.
//!
//! ## Examples
//!
//! ### Generating custom vertices
//!
//! The exampe below implements the `VertexConstructor` trait in order to use a custom
//! vertex type `MyVertex` (containing position and color), storing the tessellation in a
//! `VertexBuffers<MyVertex, u16>`, and tessellates two shapes with different colors.
//!
//! ```
//! extern crate lyon_tessellation as tess;
//! use tess::{VertexConstructor, VertexBuffers, BuffersBuilder, FillVertex, FillOptions};
//! use tess::basic_shapes::fill_circle;
//! use tess::math::point;
//!
//! // Our custom vertex.
//! #[derive(Copy, Clone, Debug)]
//! pub struct MyVertex {
//!   position: [f32; 2],
//!   color: [f32; 4],
//! }
//!
//! // The vertex constructor. This is the object that will be used to create the custom
//! // verticex from the information provided by the tessellators.
//! struct WithColor([f32; 4]);
//!
//! impl VertexConstructor<FillVertex, MyVertex> for WithColor {
//!     fn new_vertex(&mut self, vertex: FillVertex) -> MyVertex {
//!         // FillVertex also provides normals but we don't need it here.
//!         MyVertex {
//!             position: [
//!                 vertex.position.x,
//!                 vertex.position.y,
//!             ],
//!             color: self.0,
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let mut output: VertexBuffers<MyVertex, u16> = VertexBuffers::new();
//!     // Tessellate a red and a green circle.
//!     fill_circle(
//!         point(0.0, 0.0),
//!         10.0,
//!         &FillOptions::tolerance(0.05),
//!         &mut BuffersBuilder::new(
//!             &mut output,
//!             WithColor([1.0, 0.0, 0.0, 1.0])
//!         ),
//!     );
//!     fill_circle(
//!         point(10.0, 0.0),
//!         5.0,
//!         &FillOptions::tolerance(0.05),
//!         &mut BuffersBuilder::new(
//!             &mut output,
//!             WithColor([0.0, 1.0, 0.0, 1.0])
//!         ),
//!     );
//!
//!     println!(" -- {} vertices, {} indices", output.vertices.len(), output.indices.len());
//! }
//! ```
//!
//! ### Generating a completely custom output
//!
//! Using `VertexBuffers<T>` is convenient and probably fits a lot of use cases, but
//! what if we do not want to write the geometry in a pair of vectors?
//! Perhaps we want to write the geometry in a different data structure or directly
//! into gpu-accessible buffers mapped on the CPU?
//!
//! ```
//! extern crate lyon_tessellation as tess;
//! use tess::{GeometryBuilder, StrokeOptions, Count};
//! use tess::geometry_builder::VertexId;
//! use tess::basic_shapes::stroke_polyline;
//! use tess::math::point;
//! use std::fmt::Debug;
//!
//! // A geometry builder that writes the result of the tessellation to stdout instead
//! // of filling vertex and index buffers.
//! pub struct ToStdOut {
//!     vertices: u32,
//!     indices: u32,
//! }
//!
//! impl ToStdOut {
//!      pub fn new() -> Self { ToStdOut { vertices: 0, indices: 0 } }
//! }
//!
//! // This one takes any vertex type that implements Debug, so it will work with both
//! // FillVertex and StrokeVertex.
//! impl<Vertex: Debug> GeometryBuilder<Vertex> for ToStdOut {
//!     fn begin_geometry(&mut self) {
//!         // Reset the vertex in index counters.
//!         self.vertices = 0;
//!         self.indices = 0;
//!         println!(" -- begin geometry");
//!     }
//!
//!     fn end_geometry(&mut self) -> Count {
//!         println!(" -- end geometry, {} vertices, {} indices", self.vertices, self.indices);
//!         Count {
//!             vertices: self.vertices,
//!             indices: self.indices,
//!         }
//!     }
//!
//!     fn add_vertex(&mut self, vertex: Vertex) -> VertexId {
//!         println!("vertex {:?}", vertex);
//!         self.vertices += 1;
//!         VertexId(self.vertices as u32 - 1)
//!     }
//!
//!     fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
//!         println!("triangle ({}, {}, {})", a.offset(), b.offset(), c.offset());
//!         self.indices += 3;
//!     }
//!
//!     fn abort_geometry(&mut self) {
//!         println!(" -- oops!");
//!     }
//! }
//!
//! fn main() {
//!     let mut output = ToStdOut::new();
//!     stroke_polyline(
//!         [point(0.0, 0.0), point(10.0, 0.0), point(5.0, 5.0)].iter().cloned(),
//!         true,
//!         &StrokeOptions::default(),
//!         &mut output,
//!     );
//! }
//! ```
//!
//! ### Writing a tessellator
//!
//! The example below is the implementation of `basic_shapes::fill_rectangle`.
//!
//! ```
//! use lyon_tessellation::geometry_builder::*;
//! use lyon_tessellation::FillVertex;
//! use lyon_tessellation::math::{Rect, vector};
//!
//! // A tessellator that generates an axis-aligned quad.
//! // Returns a structure containing the number of vertices and number of indices allocated
//! // during the execution of this method.
//! pub fn fill_rectangle<Output>(rect: &Rect, output: &mut Output) -> Count
//! where
//!     Output: GeometryBuilder<FillVertex>
//! {
//!     output.begin_geometry();
//!     // Create the vertices...
//!     let a = output.add_vertex(
//!         FillVertex { position: rect.origin, normal: vector(-1.0, -1.0) }
//!     );
//!     let b = output.add_vertex(
//!         FillVertex { position: rect.top_right(), normal: vector(1.0, -1.0) }
//!     );
//!     let c = output.add_vertex(
//!         FillVertex { position: rect.bottom_right(), normal: vector(1.0, 1.0) }
//!     );
//!     let d = output.add_vertex(
//!         FillVertex { position: rect.bottom_left(), normal: vector(-1.0, 1.0) }
//!     );
//!     // ...and create triangle form these points. a, b, c, and d are relative offsets in the
//!     // vertex buffer.
//!     output.add_triangle(a, b, c);
//!     output.add_triangle(a, c, d);
//!
//!     output.end_geometry()
//! }
//! ```


use std::marker::PhantomData;
use std::ops::Add;
use std::convert::From;

pub type Index = u32;

/// A virtual vertex offset in a geometry.
///
/// The `VertexId`s are only valid between `GeometryBuilder::begin_geometry` and
/// `GeometryBuilder::end_geometry`. `GeometryBuilder` implementations typically be translate
/// the ids internally so that first `VertexId` after `begin_geometry` is zero.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VertexId(pub Index);

impl VertexId {
    pub fn offset(&self) -> Index { self.0 }
}

impl Add<u32> for VertexId {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        VertexId(self.0 + rhs)
    }
}

impl From<u16> for VertexId {
    fn from(v: u16) -> Self { VertexId(v as Index) }
}
impl From<u32> for VertexId {
    fn from(v: u32) -> Self { VertexId(v) }
}
impl From<i32> for VertexId {
    fn from(v: i32) -> Self { VertexId(v as Index) }
}

impl From<VertexId> for u16 {
    fn from(v: VertexId) -> Self { v.0 as u16 }
}
impl From<VertexId> for u32 {
    fn from(v: VertexId) -> Self { v.0 }
}
impl From<VertexId> for i32 {
    fn from(v: VertexId) -> Self { v.0 as i32 }
}

/// An interface separating tessellators and other geometry generation algorithms from the
/// actual vertex construction.
///
/// See the [`geometry_builder`](index.html) module documentation for more detailed explanation.
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


/// An interface with similar goals to `GeometryBuilder` for algorithms that pre-build
/// the vertex and index buffers.
///
/// This is primarily intended for efficient interaction with the libtess2 tessellator
/// from the `lyon_tess2` crate.
pub trait GeometryReceiver<Vertex> {

    fn set_geometry(
        &mut self,
        vertices: &[Vertex],
        indices: &[u32]
    );
}

/// Structure that holds the vertex and index data.
///
/// Usually writen into though temporary `BuffersBuilder` objects.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct VertexBuffers<VertexType, IndexType> {
    pub vertices: Vec<VertexType>,
    pub indices: Vec<IndexType>,
}

impl<VertexType, IndexType> VertexBuffers<VertexType, IndexType> {
    /// Constructor
    pub fn new() -> Self { VertexBuffers::with_capacity(512, 1024) }

    /// Constructor
    pub fn with_capacity(num_vertices: usize, num_indices: usize) -> Self {
        VertexBuffers {
            vertices: Vec::with_capacity(num_vertices),
            indices: Vec::with_capacity(num_indices),
        }
    }
}

/// A temporary view on a `VertexBuffers` object which facilitate the population of vertex and index
/// data.
///
/// `BuffersBuilders` record the vertex offset from when they are created so that algorithms using
/// them don't need to worry about offsetting indices if some geometry was added beforehand. This
/// means that from the point of view of a `BuffersBuilder` user, the first added vertex is at always
/// offset at the offset 0 and `VertexBuilfer` takes care of translating indices adequately.
///
/// Often, algorithms are built to generate vertex positions without knowledge of eventual other
/// vertex attributes. The `VertexConstructor` does the translation from generic `Input` to `VertexType`.
/// If your logic generates the actual vertex type directly, you can use the `SimpleBuffersBuilder`
/// convenience typedef.
pub struct BuffersBuilder<'l, VertexType: 'l, IndexType:'l, Input, Ctor> {
    buffers: &'l mut VertexBuffers<VertexType, IndexType>,
    vertex_offset: Index,
    index_offset: Index,
    vertex_constructor: Ctor,
    _marker: PhantomData<Input>,
}

impl<'l, VertexType: 'l, IndexType:'l, Input, Ctor> BuffersBuilder<'l, VertexType, IndexType, Input, Ctor> {
    pub fn new(
        buffers: &'l mut VertexBuffers<VertexType, IndexType>,
        ctor: Ctor,
    ) -> Self {
        let vertex_offset = buffers.vertices.len() as Index;
        let index_offset = buffers.indices.len() as Index;
        BuffersBuilder {
            buffers,
            vertex_offset,
            index_offset,
            vertex_constructor: ctor,
            _marker: PhantomData,
        }
    }

    pub fn buffers<'a, 'b: 'a>(&'b self) -> &'a VertexBuffers<VertexType, IndexType> {
        self.buffers
    }
}

/// Creates a `BuffersBuilder`.
pub fn vertex_builder<VertexType, IndexType, Input, Ctor>(
    buffers: &mut VertexBuffers<VertexType, IndexType>,
    ctor: Ctor,
) -> BuffersBuilder<VertexType, IndexType, Input, Ctor>
where
    Ctor: VertexConstructor<Input, VertexType>
{
    BuffersBuilder::new(buffers, ctor)
}

/// A trait specifying how to create vertex values.
pub trait VertexConstructor<Input, VertexType> {
    fn new_vertex(&mut self, input: Input) -> VertexType;
}

/// A dummy vertex constructor that just forwards its inputs.
pub struct Identity;
impl<T> VertexConstructor<T, T> for Identity {
    fn new_vertex(&mut self, input: T) -> T { input }
}

impl<F, Input, VertexType> VertexConstructor<Input, VertexType> for F
    where F: Fn(Input) -> VertexType
{
    fn new_vertex(&mut self, vertex: Input) -> VertexType {
        self(vertex)
    }
}

/// A `BuffersBuilder` that takes the actual vertex type as input.
pub type SimpleBuffersBuilder<'l, VertexType> = BuffersBuilder<'l, VertexType, u16, VertexType, Identity>;

/// Creates a `SimpleBuffersBuilder`.
pub fn simple_builder<VertexType>(buffers: &mut VertexBuffers<VertexType, u16>)
    -> SimpleBuffersBuilder<VertexType> {
    let vertex_offset = buffers.vertices.len() as Index;
    let index_offset = buffers.indices.len() as Index;
    BuffersBuilder {
        buffers,
        vertex_offset,
        index_offset,
        vertex_constructor: Identity,
        _marker: PhantomData,
    }
}

/// Number of vertices and indices added during the tessellation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Count {
    pub vertices: u32,
    pub indices: u32,
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

impl<'l, VertexType, IndexType, Input, Ctor> GeometryBuilder<Input>
    for BuffersBuilder<'l, VertexType, IndexType, Input, Ctor>
where
    VertexType: 'l + Clone,
    IndexType: Add + From<VertexId>,
    Ctor: VertexConstructor<Input, VertexType>,
{
    fn begin_geometry(&mut self) {
        self.vertex_offset = self.buffers.vertices.len() as Index;
        self.index_offset = self.buffers.indices.len() as Index;
    }

    fn end_geometry(&mut self) -> Count {
        Count {
            vertices: self.buffers.vertices.len() as u32 - self.vertex_offset,
            indices: self.buffers.indices.len() as u32 - self.index_offset,
        }
    }

    fn add_vertex(&mut self, v: Input) -> VertexId {
        self.buffers.vertices.push(self.vertex_constructor.new_vertex(v));
        VertexId(self.buffers.vertices.len() as Index - 1 - self.vertex_offset)
    }

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        self.buffers.indices.push((a + self.vertex_offset).into());
        self.buffers.indices.push((b + self.vertex_offset).into());
        self.buffers.indices.push((c + self.vertex_offset).into());
    }

    fn abort_geometry(&mut self) {
        self.buffers.vertices.truncate(self.vertex_offset as usize);
        self.buffers.indices.truncate(self.index_offset as usize);
    }
}

impl<'l, VertexType, IndexType, InputVertex, Ctor> GeometryReceiver<InputVertex>
    for BuffersBuilder<'l, VertexType, IndexType, InputVertex, Ctor>
where
    VertexType: 'l + Clone,
    IndexType: From<VertexId>,
    Ctor: VertexConstructor<InputVertex, VertexType>,
    InputVertex: Clone,
{
    fn set_geometry(
        &mut self,
        vertices: &[InputVertex],
        indices: &[u32]
    ) {
        for v in vertices {
            let vertex = self.vertex_constructor.new_vertex(v.clone());
            self.buffers.vertices.push(vertex);
        }
        for idx in indices {
            self.buffers.indices.push(IndexType::from(idx.clone().into()));
        }
    }
}

/// A geometry builder that does not output any geometry.
///
/// Mostly useful for testing.
pub struct NoOutput {
    count: Count,
}

impl NoOutput {
    pub fn new() -> Self {
        NoOutput { count: Count { vertices: 0, indices: 0 } }
    }
}

impl<T> GeometryBuilder<T> for NoOutput
{
    fn begin_geometry(&mut self) {
        self.count.vertices = 0;
        self.count.indices = 0;
    }

    fn add_vertex(&mut self, _: T) -> VertexId {
        self.count.vertices += 1;
        VertexId(self.count.vertices as Index - 1)
    }

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        debug_assert!(a != b);
        debug_assert!(a != c);
        debug_assert!(b != c);
        self.count.indices += 3;
    }

    fn end_geometry(&mut self) -> Count { self.count }
    fn abort_geometry(&mut self) {}
}

impl<V> GeometryReceiver<V> for NoOutput {
    fn set_geometry(&mut self, _vertices: &[V], _indices: &[u32]) {}
}

// /// An extension to GeometryBuilder that can handle quadratic bézier segments.
// pub trait BezierGeometryBuilder<Input>: GeometryBuilder<Input> {
//     /// Insert a quadratic bezier curve.
//     /// The interrior is on the right side of the curve.
//     ///
//     /// This method can only be called between `begin_geometry` and `end_geometry`.
//     fn add_quadratic_bezier(&mut self, from: VertexId, to: VertexId, ctrl: Input);
// }
// impl<'l, VertexType, Input, Ctor> BezierGeometryBuilder<Input>
//     for BuffersBuilder<'l, VertexType, Input, Ctor>
// where
//     VertexType: 'l + Clone,
//     Ctor: VertexConstructor<Input, VertexType>,
// {
//     fn add_quadratic_bezier(&mut self, _from: VertexId, _to: VertexId, _ctrl: Input) {
//         unimplemented!();
//     }
// }

#[test]
fn test_simple_quad() {
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
                color: self.0,
            }
        }
    }

    // A typical "algortihm" that generates some geometry, in this case a simple axis-aligned quad.
    fn add_quad<Builder: GeometryBuilder<[f32; 2]>>(
        top_left: [f32; 2],
        size: [f32; 2],
        mut out: Builder,
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

        count
    }


    let mut buffers: VertexBuffers<Vertex2d, u32> = VertexBuffers::new();
    let red = [1.0, 0.0, 0.0, 1.0];
    let green = [0.0, 1.0, 0.0, 1.0];

    add_quad([0.0, 0.0], [1.0, 1.0], vertex_builder(&mut buffers, WithColor(red)));

    assert_eq!(
        buffers.vertices[0],
        Vertex2d {
            position: [0.0, 0.0],
            color: red,
        }
    );
    assert_eq!(
        buffers.vertices[1],
        Vertex2d {
            position: [1.0, 0.0],
            color: red,
        }
    );
    assert_eq!(
        buffers.vertices[3],
        Vertex2d {
            position: [0.0, 1.0],
            color: red,
        }
    );
    assert_eq!(
        buffers.vertices[2],
        Vertex2d {
            position: [1.0, 1.0],
            color: red,
        }
    );
    assert_eq!(&buffers.indices[..], &[0, 1, 2, 0, 2, 3]);

    add_quad([10.0, 10.0], [1.0, 1.0], vertex_builder(&mut buffers, WithColor(green)));

    assert_eq!(
        buffers.vertices[4],
        Vertex2d {
            position: [10.0, 10.0],
            color: green,
        }
    );
    assert_eq!(
        buffers.vertices[5],
        Vertex2d {
            position: [11.0, 10.0],
            color: green,
        }
    );
    assert_eq!(
        buffers.vertices[6],
        Vertex2d {
            position: [11.0, 11.0],
            color: green,
        }
    );
    assert_eq!(
        buffers.vertices[7],
        Vertex2d {
            position: [10.0, 11.0],
            color: green,
        }
    );
    assert_eq!(&buffers.indices[..], &[0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
}

#[test]
fn test_closure() {
    use math::{Point, point, vector};

    let translation = vector(1.0, 0.0);

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    {
        // A builder that just translates all vertices by `translation`.
        let mut builder = vertex_builder(&mut buffers, |position| {
            position + translation
        });

        builder.begin_geometry();
        let a = builder.add_vertex(point(0.0, 0.0));
        let b = builder.add_vertex(point(1.0, 0.0));
        let c = builder.add_vertex(point(1.0, 1.0));
        let d = builder.add_vertex(point(0.0, 1.0));
        builder.add_triangle(a, b, c);
        builder.add_triangle(a, c, d);
        builder.end_geometry();
    }

    assert_eq!(buffers.vertices, vec![
        point(1.0, 0.0),
        point(2.0, 0.0),
        point(2.0, 1.0),
        point(1.0, 1.0),
    ]);
}
