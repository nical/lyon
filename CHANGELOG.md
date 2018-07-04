## Change log

### v0.11.0 (2018-07-05)
  - [tessellation] Move the VertexId representation from u16 to u32.
  - [tessellation] Fix a circle tessellation bug with large tolerance values.
  - [tessellation] Add a fast path for ellipses when the radii are equal.
  - [algorithms] Added the lyon_algorithms crate.
  - [algorithms] Implement a hatching pattern fill algorithm.
  - [algorithms] Implement a dotted pattern fill algorithm.
  - [algorithms] Implement path bounding rectangles.
  - [algorithms] Implement rectangle fitting transform computation.
  - [algorithms] Move path walking to the algorithms crate.
  - [geom] Implement callback based iteration over the monotonic parts of an arc.
  - [geom] Add LineSegment::set_length.
  - [geom] Fix an elliptic arc bug.
  - [geom] Implement precise elliptic arc bounding rectangle.
  - [cli] Add support for custom formatting in the tessellate command.
  - [cli] Allow changing the background in the show command.
  - [cli] Automatically position the view in the show command.
  - [examples] Add a simple SVG rendering example.
  - [misc] Update usvg and euclid dependencies.

### v0.10.0 (2018-02-28)
  - [geom] Fix several arc bugs.
  - [geom] Implement a much better cubic to quadratic bézier approximation.
  - [geom] Implement iterating over the monotonic parts of a bézier curve.
  - [geom] A few API changes.
  - [lyon] Make serde optional for all crates ("serialization" feature flags).
  - [tessellation] Implement better error handling.
  - [extra] Revive the toy software rasterizer.

### v0.9.1 (2018-01-14)
  - [tessellation] Fix missing vertices when normals are disabled.
  - [tess2] Add an alternative tessellator based on libtess2.
  - [cli] Expose the tess2 tessellator in the app.

### v0.9.0 (2018-01-08)
  - [lyon] Simplify the carte structure:
    - Rename the `lyon_bezier` crate into `lyon_geom`.
    - Merge `lyon_path_iterator` and `lyon_path_builder` into the `lyon_path` crate.
    - Remove the `lyon_core` crate.
  - [geom] Implement new bézier intersection methods.
  - [geom] Make geometrix types generic over float types.
  - [geom] Rename `Vec2` into `Vector`.
  - [geom] Implement new cubic to quadratic bézier approximations.
  - [path] Support arcs in `PathEvent`.
  - [path] Implement walking along a path at constant speed.
  - [tessellation] Fix some fill tessellation bugs found by the fuzzer.
  - [tessellation] Use trait objects instead of generics when using `GeometryBuilder` in the API.
  - [tessellation] Fix incorrect rounded rectangle tessellation.
  - [svg] Bump svgparser dependecy to 0.6.x.

### v0.8.8 (2018-01-14)
  - [tessellation] Fix missing vertices when normals are disabled.
  - [tessellation] Fix incorrect rounded rectangle tessellation.

### v0.8.5 (2017-11-05)
  - [tessellation] Fix several fill tessellation bugs found by the fuzzer.
  - [tessellation] Implement Vertex normals in the fill tessellator.
  - [tessellation] Make the triangle winding order consistent.
  - [tessellation] Implement stroke miter limit.
  - [tessellation] Fix incorrect tessellation in fill_convex_polyline.
  - [tessellation] Add constants to FillOptions and StrokeOptions.
  - [bezier] Fix some precision issues in the curve flattening code.
  - [cli] Add an interactive path viewer.
  - [cli] Improve the interface of the command line app.
  - [examples] Update the glutin dependency.

### v0.8.4 (2017-10-18)
  - [tessellation] Fix several fill tessellation bugs found by the fuzzer.
  - [tessellation] Fix a stroke tessellation bug.
  - [bezier] Fix a flattening bug.
  - [cli] Add a simple fuzzer.

### v0.8.2 (2017-10-07)
  - [tessellation] Fix a floating point precision bug in the fill tessellator with almost-overlapping edges. All tests are now passing.

### v0.8.0 (2017-09-29)
  - [tessellation] Performance improvements to the fill and stroke path tessellators.
  - [tessellation] Stroke path tessellator bug fixes.
  - [tessellation] Fix a bug in the tessellation of circle strokes.
  - [tessellation] Implement bevel line joins in the stroke tessellator.
  - [tessellation] Default to bevel joins when the miter length exceeds the limit.
  - [path_iterator] Rename some iterators to comply with the rust API guidelines.
  - [tessellation] Minor API changes.

### v0.7.3 (2017-08-20)
  - [tessellation] Fill tessellator bug fix (#150).
  - [tessellation] Import tests from Mapbox's earcut tessellator.
  - [svg] Add path builder that writes a string using the SVG path syntax.

### v0.7.1 (2017-08-02)
  - [bezier] Fix broken conversion between arc and quadratic béziers.
  - [tessellation] Fix bug in circle tessellation when center is not the origin.
  - [tessellation] (re-)implement `basic_shapes::fill_ellipse`.
  - [tessellation] Implement `basic_shapes::stroke_ellipse`.
  - [tessellation] Minor doc improvements.

### v0.7.0 (2017-07-31)
  - [tessellation] Various API improvements.
  - [tessellation] Implement `basic_shapes::fill_polyline`.
  - [tessellation] Implement round stroke caps.
  - [tessellation] Fix bug causing generated stroke width to be half of what it should be.

### v 0.6.2 (2017-07-28)
  - [tessellation] Improve numerical stability in the stroke tessellator.

### v 0.6.1 (2017-07-08)
  - [bezier] Intersection between a bézier segment and a line or line segment.
  - [bezier] Bézier flattening bug fixes.
  - [tessellation] Implement a stroke tessellator for rounded rectangles.
  - [tessellation] Bug fixes in the fill tessellator.
  - [tessellation] Bug fixes in the stroke tessellator.

### v 0.6.0 (2017-07-04)
  - [svg] Add a helper to build paths from the SVG path syntax.
  - [bezier] Allow applying transforms to all geometric types.
  - [bezier] Added Triangle, Line, LineSegment and monotone bézier types.
  - [bezier] Compute the x/y extremeums of bézier segments.
  - [bezier] Compute the conservative and minimum bounding rects of bézier segments.
  - [tessellation] Support for round line joins in the stroke tessellator.
  - [tessellation] The stroke tessellator applies stroke width by default (optional).
  - [tessellation] fill_convext_polyline now properly compute normals.
  - [tessellation] Implement a stroke tessellator for circles.
  - [tessellation] Arcs to bézier convertion refactored, bugs fixed.
  - [tessellation] Measure distance along strokes.
  - [tessellation] Bug fixes in the fill tessellator.
  - [tessellation] Bug fixes in the stroke tessellator.
  - [lyon] Bump euclid dependency to 0.15.1.
  - [cli] The command line tool can be set to find minimal test cases on errors.

### v 0.5.0 (2017-05-23)
  - [tessellation] Implement fill tessellation for rounded rectangles.
  - [tessellation] Implement fill tessellation for circles.
  - [svg] Bump svgparser dependency from 0.0.3 t0 0.4.0.
  - [lyon] Bump euclid dependency from 0.10.1 to 0.13.
  - [bezier] Fix a bug (issue #19) in the cubic bézier flattening code.
  - [bezier] Expose a method to find the inflection points of a cubic bézier curve.
  - [bezier] Expose a method to compute the length of bézier segments.
  - [doc] Improve the crate documentations.
  - [doc] Add CHANGLOG.md
  - [examples] rename the gfx_logo example into gfx_advanced and add a simpler gfx_basic example.

### v0.4.1 (2017-05-02)
  - [doc] Make the documentation easier to use in docs.rs.
  - [tessellation] Work around a floating point precision issue in the stroke tessellator.

### v0.4.0 (2017-02-28)
  - [tessellation] Allow applying stroke width outside of the tessellator.
  - [examples] Improve the gfx_logo example.
  - [tests] Setup travis ci.
  - [tessellation] Improve the performance of the fill tessellator.
  - [svg] add the lyon_svg crate.

### v0.3.2 (2016-09-22)
  - [doc] Improve the documentation.

### v0.3.1 (2016-09-21)
  - [doc] Add a lot of documentation.

...