## Change log

### (next)
  - [tessellation] implement fill tessellation for rounded rectangles.
  - [svg] Bump svgparser dependency from 0.0.3 t0 0.4.0.
  - [lyon] Bump euclid dependency from 0.10.1 to 0.13.
  - [bezier] Fix a bug (issue #19) in the cubic bezier flattening code.
  - [bezier] Expose a method to find the inflection points of a cubic bezier curve.
  - [bezier] Expose a method to compute the length of bezier segments.
  - [doc] Various doc updates.
  - [doc] Add CHANGLOG.md
  - [examples] rename the gfx_logo example into gfx_advanced and add a simpler gfx_basic example.

### v0.4.1 (2016-05-02)
  - [doc] Make the documentation easier to use in docs.rs.
  - [tessellation] Work around a floating point precision issue in the stroke tessellator.

### v0.4.0 (2016-02-28)
  - [tessellation] Allow applying stroke width outside of the tessellator.
  - [examples] Improve the gfx_logo example.
  - [tests] Setup travis ci.
  - [tessellation] Improve the performance of the fill tessellator.
  - [svg] add the lyon_svg crate.

### v0.3.2 (2016-09-22)
  - [doc] improve the documentation.

### v0.3.1 (2016-09-21)
  - [doc] Add a lot of documentation.

...