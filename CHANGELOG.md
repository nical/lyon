## Change log

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