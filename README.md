# Lyon
GPU-based 2D graphics rendering in rust.

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
        tessellator.tessellate_flattened_path(
            path.path_iter().flattened(0.1),
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

The project is split into small crates:

* [![doc](https://docs.rs/lyon_tessellation/badge.svg)](https://docs.rs/lyon_tessellation) - [lyon_tessellation](https://crates.io/crates/lyon_tessellation) - Path tessellation routines.
* [![doc](https://docs.rs/lyon_path_builder/badge.svg)](https://docs.rs/lyon_path_builder) - [lyon_path_builder](https://crates.io/crates/lyon_path_builder) - Tools to facilitate building paths.
* [![doc](https://docs.rs/lyon_path_iterator/badge.svg)](https://docs.rs/lyon_path_iterator) - [lyon_path_iterator](https://crates.io/crates/lyon_path_iterator) - Tools to facilitate iteratring over paths.
* [![doc](https://docs.rs/lyon_path/badge.svg)](https://docs.rs/lyon_path) - [lyon_path](https://crates.io/crates/lyon_path) - A simple optional path data structure, provided for convenience.
* [![doc](https://docs.rs/lyon_bezier/badge.svg)](https://docs.rs/lyon_bezier) - [lyon_bezier](https://crates.io/crates/lyon_bezier) - Cubic and quadratic 2d bezier math.
* [![doc](https://docs.rs/lyon_svg/badge.svg)](https://docs.rs/lyon_svg) - [lyon_svg](https://crates.io/crates/lyon_svg) - Create paths using SVG's path syntax.
* [![doc](https://docs.rs/lyon_extra/badge.svg)](https://docs.rs/lyon_extra) - [lyon_extra](https://crates.io/crates/lyon_extra) - Additional testing and debugging tools.
* [![doc](https://docs.rs/lyon_core/badge.svg)](https://docs.rs/lyon_path_core) - [lyon_core](https://crates.io/crates/lyon_core) - Common types to most lyon crates.

There is also a toy [command-line tool](cli) to tessellate SVG path from your favorite terminal.

Have a look at the [basic](examples/gfx_basic) and [advanced](examples/gfx_advanced) gfx-rs examples to see how integrating the tessellators in a renderer can look like.

## TODO

The [github issues](https://github.com/nical/lyon/issues) is the most up to date list of things to do in the near future. Some of these issues have the ["help wanted" label](https://github.com/nical/lyon/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) which indicates that they are approachable for people who are not already familiar with the code. The ones with the [easy label](https://github.com/nical/lyon/issues?q=is%3Aissue+is%3Aopen+label%3Aeasy) are good places to start for anyone looking to [contribute](https://github.com/nical/lyon/blob/master/CONTRIBUTING.md) for the first time.

Need help working on or with lyon? [File an issue](https://github.com/nical/lyon/issues/new), ask questions on [gitter](https://gitter.im/lyon-rs/Lobby), or contact [@nical](https://github.com/nical) by e-mail.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

There is useful information for contributors in the [contribution guidelines](https://github.com/nical/lyon/blob/master/CONTRIBUTING.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
