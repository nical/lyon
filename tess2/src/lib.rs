#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]
#![deny(bare_trait_objects)]

//! Alternative fill tessellation implementation using
//! [libtess2](https://github.com/memononen/libtess2).
//!
//! # Lyon libtess2 wrapper
//!
//! This crate provides an alternative path fill tessellator implemented
//! as a wrapper of the [libtess2](https://github.com/memononen/libtess2)
//! C library.
//!
//! The goal of this crate is to provide an alternative tessellator for
//! the potential cases where lyon_tessellation::FillTessellator is lacking
//! in features or robustness, and have something to compare the latter
//! against.
//!
//! ## Comparison with [lyon_tessellation::FillTessellator](https://docs.rs/lyon_tessellation/)
//!
//! Advantages:
//!
//! - Supports the `NonZero` fill rule.
//! - More robust against precision errors when paths have many self
//!   intersections very close to each other.
//!
//! Disadvantages:
//!
//! - About twice slower than lyon_tessellation's fill tessellator.
//! - Does not support computing vertex normals.
//! - Wrapper around a C library (as opposed to pure rust with no
//!   unsafe code).
//!
//! ## API
//!
//! In order to avoid any overhead, this crate introduces the
//! FlattenedPath type which stores already-flattened paths
//! in the memory layout expected by libtess2.
//! Instead of working with a `GeometryBuilder` like the tessellators
//! in `lyon_tessellation`, this tessellator uses a `GeometryReceiver`
//! trait that corresponds to the way libtess2 exposes its output.
//!
//! ## Example
//!
//! ```
//! extern crate lyon_tess2 as tess2;
//! use tess2::{FillTessellator, FillOptions};
//! use tess2::math::{Point, point};
//! use tess2::path::Path;
//! use tess2::path::builder::*;
//! use tess2::geometry_builder::*;
//!
//! fn main() {
//!     // Create a simple path.
//!     let mut path_builder = Path::builder();
//!     path_builder.begin(point(0.0, 0.0));
//!     path_builder.line_to(point(1.0, 2.0));
//!     path_builder.line_to(point(2.0, 0.0));
//!     path_builder.line_to(point(1.0, 1.0));
//!     path_builder.end(true);
//!     let path = path_builder.build();
//!
//!     // Create the destination vertex and index buffers.
//!     let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
//!
//!     {
//!         // Create the tessellator.
//!         let mut tessellator = FillTessellator::new();
//!
//!         // Compute the tessellation.
//!         let result = tessellator.tessellate(
//!             &path,
//!             &FillOptions::default(),
//!             &mut BuffersBuilder::new(&mut buffers, Positions)
//!         );
//!         assert!(result.is_ok());
//!     }
//!     println!("The generated vertices are: {:?}.", &buffers.vertices[..]);
//!     println!("The generated indices are: {:?}.", &buffers.indices[..]);
//!
//! }
//! ```

pub extern crate lyon_tessellation as tessellation;
pub extern crate tess2_sys;
pub use tessellation::geom;
pub use tessellation::math;
pub use tessellation::path;

pub mod flattened_path;
mod tessellator;

pub use crate::tessellation::FillOptions;
pub use crate::tessellator::FillTessellator;

pub mod geometry_builder {
    pub use crate::tessellation::geometry_builder::{Positions, NoOutput, VertexBuffers};
    pub use crate::tessellation::VertexId;
    use crate::math::Point;

    /// An interface with similar goals to `GeometryBuilder` for algorithms that pre-build
    /// the vertex and index buffers.
    ///
    /// This is primarily intended for efficient interaction with the libtess2 tessellator
    /// from the `lyon_tess2` crate.
    pub trait GeometryReceiver {
        fn set_geometry(&mut self, vertices: &[Point], indices: &[u32]);
    }

    /// A trait specifying how to create vertex values.
    pub trait BasicVertexConstructor<OutputVertex> {
        fn new_vertex(&mut self, point: Point) -> OutputVertex;
    }

    impl BasicVertexConstructor<Point> for Positions {
        fn new_vertex(&mut self, position: Point) -> Point {
            position
        }
    }

    impl<F, OutputVertex> BasicVertexConstructor<OutputVertex> for F
    where
        F: Fn(Point) -> OutputVertex,
    {
        fn new_vertex(&mut self, position: Point) -> OutputVertex {
            self(position)
        }
    }

    pub struct BuffersBuilder<'l, OutputVertex: 'l, OutputIndex: 'l, Ctor> {
        buffers: &'l mut VertexBuffers<OutputVertex, OutputIndex>,
        vertex_constructor: Ctor,
    }

    impl<'l, OutputVertex: 'l, OutputIndex: 'l, Ctor>
        BuffersBuilder<'l, OutputVertex, OutputIndex, Ctor>
    {
        pub fn new(buffers: &'l mut VertexBuffers<OutputVertex, OutputIndex>, ctor: Ctor) -> Self {
            BuffersBuilder {
                buffers,
                vertex_constructor: ctor,
            }
        }

        pub fn buffers<'a, 'b: 'a>(&'b self) -> &'a VertexBuffers<OutputVertex, OutputIndex> {
            self.buffers
        }
    }

    impl<'l, OutputVertex, OutputIndex, Ctor> GeometryReceiver
        for BuffersBuilder<'l, OutputVertex, OutputIndex, Ctor>
    where
        OutputIndex: From<VertexId>,
        Ctor: BasicVertexConstructor<OutputVertex>,
    {
        fn set_geometry(&mut self, vertices: &[Point], indices: &[u32]) {
            for v in vertices {
                let vertex = self.vertex_constructor.new_vertex(*v);
                self.buffers.vertices.push(vertex);
            }
            for idx in indices {
                self.buffers.indices.push(OutputIndex::from((*idx).into()));
            }
        }
    }

    impl GeometryReceiver for NoOutput {
        fn set_geometry(&mut self, _vertices: &[Point], _indices: &[u32]) {}
    }
}
