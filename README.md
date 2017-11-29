# Lyon
A path tessellation library written in rust for GPU-based 2D graphics rendering.

<p align="center">
<img src="https://nical.github.io/lyon-doc/lyon-logo.svg" alt="Project logo">
</p>

<p align="center">
  <a href="https://crates.io/crates/lyon">
      <img src="http://meritbadge.herokuapp.com/lyon" alt="crates.io">
  </a>
  <a href="https://travis-ci.org/nical/lyon">
      <img src="https://img.shields.io/travis/nical/lyon/master.svg" alt="Travis Build Status">
  </a>
  <a href="https://docs.rs/lyon">
      <img src="https://docs.rs/lyon/badge.svg" alt="documentation">
  </a>

  <a href="https://gitter.im/lyon-rs/Lobby">
    <img src="https://img.shields.io/badge/GITTER-join%20chat-green.svg" alt="Gitter Chat">
  </a>

</p>

# Motivation

For now the goal is to provide efficient SVG-compliant path tessellation tools to help with rendering vector graphics on the GPU. For now think of this library as a way to turn complex paths into triangles for use in your own rendering engine.

The intent is for this library to be useful in projects like [Servo](https://servo.org/) and games.

## Example

```rust
    // Build a Path.
    let mut builder = SvgPathBuilder::new(Path::builder());
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
    builder.cubic_bezier_to(point(1.0, 1.0), point(0.0, 1.0), point(0.0, 0.0));
    builder.close();
    let path = builder.build();

    // Will contain the result of the tessellation.
    let mut geometry_cpu: VertexBuffers<Vec2> = VertexBuffers::new();

    let mut tessellator = FillTessellator::new();

    {
        // The simple builder uses the tessellator's vertex type.
        // You can implement the GeometryBuilder trait to create custom vertices.
        let mut vertex_builder = simple_builder(&mut geometry_cpu);

        // Compute the tessellation.
        tessellator.tessellate_path(
            path.path_iter(),
            &FillOptions::default(),
            &mut vertex_builder
        ).unwrap();
    }

    // The tessellated geometry is ready to be uploaded to the GPU.
    println!(" -- {} vertices {} indices",
        geometry_cpu.vertices.len(),
        geometry_cpu.indices.len()
    );
```

## Structure

The project is split into several crates:

* [![crate](http://meritbadge.herokuapp.com/lyon)](https://crates.io/crates/lyon)
  [![doc](https://docs.rs/lyon/badge.svg)](https://docs.rs/lyon) -
  **lyon** - A meta-crate that reexports the crates below for convenience.
* [![crate](http://meritbadge.herokuapp.com/lyon_tessellation)](https://crates.io/crates/lyon_tessellation)
  [![doc](https://docs.rs/lyon_tessellation/badge.svg)](https://docs.rs/lyon_tessellation) -
  **lyon_tessellation** - Path tessellation routines.
* [![crate](http://meritbadge.herokuapp.com/lyon_path)](https://crates.io/crates/lyon_path)
  [![doc](https://docs.rs/lyon_path/badge.svg)](https://docs.rs/lyon_path) -
  **lyon_path** - Tools to build and iterate over paths.
* [![crate](http://meritbadge.herokuapp.com/lyon_geom)](https://crates.io/crates/lyon_geom)
  [![doc](https://docs.rs/lyon_geom/badge.svg)](https://docs.rs/lyon_geom) -
  **lyon_geom** - Cubic and quadratic 2d bézier math.
* [![crate](http://meritbadge.herokuapp.com/lyon_svg)](https://crates.io/crates/lyon_svg)
  [![doc](https://docs.rs/lyon_svg/badge.svg)](https://docs.rs/lyon_svg) -
  **lyon_svg** - Create paths using SVG's path syntax.
* [![crate](http://meritbadge.herokuapp.com/lyon_extra)](https://crates.io/crates/lyon_extra)
  [![doc](https://docs.rs/lyon_extra/badge.svg)](https://docs.rs/lyon_extra) -
  **lyon_extra** - Additional testing and debugging tools.
* [![crate](http://meritbadge.herokuapp.com/lyon_core)](https://crates.io/crates/lyon_core)
  [![doc](https://docs.rs/lyon_core/badge.svg)](https://docs.rs/lyon_core) -
  **lyon_core** - Common types to most lyon crates (mostly for internal use, reexported by the other crates).

There is also a toy [command-line tool](cli) to tessellate SVG path from your favorite terminal.

Have a look at the [basic](examples/gfx_basic) and [advanced](examples/gfx_advanced) gfx-rs examples to see how integrating the tessellators in a renderer can look like.

## FAQ

### In a nutshell, what is a tessellator?

Tessellators such as the ones provided by lyon take complex shapes as input and generate geometry made of triangles that can be easily consumed by graphics APIs such as OpenGL, Vulkan or D3D.

### How do I render an SVG file with lyon?

Lyon is *not* an SVG renderer. For now lyon mainly provides primitives to tessellate complex path fills and strokes in a way that is convenient to use with GPU APIs such as gfx-rs, glium, OpenGL, D3D, etc. How the tessellated geometry is rendered is completely up to the user of this crate.

### How do I render the output of the tessellators?

Although the format of the output of the tessellators is customizable, the algorithms are designed to generate a vertex and an index buffer. See the [lyon::tessellation documentaton](https://docs.rs/lyon_tessellation/0.7.4/lyon_tessellation/#the-output-geometry-builders) for more details.

### Is anti-aliasing supported?

There is currently no built-in support for anti-aliasing in the tessellators. Anti-aliasing can still be achieved by users of this crate using techniques commonly employed in video games (msaa, taa, fxaa, etc.).

### What is left to do before lyon 1.0?

See the [1.0 milestone](https://github.com/nical/lyon/milestone/2) on the github repository.

### I need help!

Don't hesitate to [file an issue](https://github.com/nical/lyon/issues/new), ask questions on [gitter](https://gitter.im/lyon-rs/Lobby), or contact [@nical](https://github.com/nical) by e-mail.

### How can I help?

See [CONTRIBUTING.md](https://github.com/nical/lyon/blob/master/CONTRIBUTING.md).

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 * [Mozilla Public License 2.0](https://www.mozilla.org/en-US/MPL/2.0/)

at your option.

Dual MIT/Apache2 is strictly more permissive

### Contribution

There is useful information for contributors in the [contribution guidelines](https://github.com/nical/lyon/blob/master/CONTRIBUTING.md).
