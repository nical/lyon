use crate::flattened_path::FlattenedPath;
use crate::geometry_builder::GeometryReceiver;
use crate::math::*;
use crate::path::builder::*;
use crate::path::{Attributes, PathEvent, PathSlice};
use crate::tessellation::{Count, FillOptions, FillRule};

use std::os::raw::c_void;
use std::ptr;
use std::slice;
use tess2_sys::*;

/// A unit struct that represents any error occuring during tesselation.
#[derive(Debug)]
pub struct TessellationError;

impl std::fmt::Display for TessellationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an unknown error occurred during tessellation")
    }
}

impl std::error::Error for TessellationError {}

/// A fill tessellator implemented on top of [libtess2](https://github.com/memononen/libtess2).
///
/// When in doubt it is usually preferable to use
/// [lyon_tessellation](https://docs.rs/lyon_tessellation/)'s `FillTessellator`.
/// However in some cases, for example when the `NonZero` fill rule
/// is needed, This tessellator provides a good fallback.
pub struct FillTessellator {
    tess: *mut TESStesselator,
}

impl FillTessellator {
    pub fn new() -> Self {
        unsafe {
            FillTessellator {
                tess: tessNewTess(ptr::null_mut()),
            }
        }
    }

    /// Compute the tessellation from a path iterator.
    pub fn tessellate<Iter>(
        &mut self,
        it: Iter,
        options: &FillOptions,
        output: &mut dyn GeometryReceiver,
    ) -> Result<Count, TessellationError>
    where
        Iter: IntoIterator<Item = PathEvent>,
    {
        let mut builder = FlattenedPath::builder(options.tolerance);

        for evt in it {
            builder.path_event(evt, Attributes::NONE);
        }

        let flattened_path = builder.build();

        self.tessellate_flattened_path(&flattened_path, options, output)
    }

    /// Compute the tessellation from a pre-flattened path.
    pub fn tessellate_flattened_path(
        &mut self,
        path: &FlattenedPath,
        options: &FillOptions,
        output: &mut dyn GeometryReceiver,
    ) -> Result<Count, TessellationError> {
        self.prepare_path(path);

        if !self.do_tessellate(options) {
            return Err(TessellationError);
        }

        Ok(self.process_output(output))
    }

    /// Compute the tessellation from a path slice.
    pub fn tessellate_path<'l>(
        &'l mut self,
        path: impl Into<PathSlice<'l>>,
        options: &'l FillOptions,
        output: &mut dyn GeometryReceiver,
    ) -> Result<Count, TessellationError> {
        self.tessellate(path.into().iter(), options, output)
    }

    fn prepare_path(&mut self, path: &FlattenedPath) {
        unsafe {
            for sub_path in path.sub_paths() {
                let first_point = &sub_path.points()[0];
                let num_points = sub_path.points().len();
                tessAddContour(
                    self.tess,
                    2,
                    (&first_point.x as *const f32) as *const c_void,
                    8,
                    num_points as i32,
                );
            }
        }
    }

    fn do_tessellate(&mut self, options: &FillOptions) -> bool {
        unsafe {
            let winding_rule = match options.fill_rule {
                FillRule::EvenOdd => TessWindingRule::TESS_WINDING_ODD,
                FillRule::NonZero => TessWindingRule::TESS_WINDING_NONZERO,
            };

            let res = tessTesselate(
                self.tess,
                winding_rule,
                TessElementType::TESS_POLYGONS,
                3,
                2,
                ptr::null_mut(),
            );

            res == 1
        }
    }

    fn process_output(&mut self, output: &mut dyn GeometryReceiver) -> Count {
        unsafe {
            let num_indices = tessGetElementCount(self.tess) as usize * 3;
            let num_vertices = tessGetElementCount(self.tess) as usize;

            let vertices =
                slice::from_raw_parts(tessGetVertices(self.tess) as *const Point, num_vertices);
            let indices =
                slice::from_raw_parts(tessGetElements(self.tess) as *const u32, num_indices);

            output.set_geometry(vertices, indices);

            Count {
                vertices: num_indices as u32,
                indices: num_indices as u32,
            }
        }
    }
}

impl Drop for FillTessellator {
    fn drop(&mut self) {
        unsafe {
            tessDeleteTess(self.tess);
        }
    }
}

impl Default for FillTessellator {
    fn default() -> Self {
        Self::new()
    }
}
