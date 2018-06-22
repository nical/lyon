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
extern crate lyon;
use lyon::math::point;
use lyon::path::default::Path;
use lyon::path::builder::*;
use lyon::tessellation::*;

fn main() {
    // Build a Path.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
    builder.cubic_bezier_to(point(1.0, 1.0), point(0.0, 1.0), point(0.0, 0.0));
    builder.close();
    let path = builder.build();

    // Let's use our own custom vertex type instead of the default one.
    #[derive(Copy, Clone, Debug)]
    struct MyVertex { position: [f32; 2], normal: [f32; 2] };

    // Will contain the result of the tessellation.
    let mut geometry: VertexBuffers<MyVertex, u16> = VertexBuffers::new();

    let mut tessellator = FillTessellator::new();

    {
        // Compute the tessellation.
        tessellator.tessellate_path(
            path.path_iter(),
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, |vertex : FillVertex| {
                MyVertex {
                    position: vertex.position.to_array(),
                    normal: vertex.normal.to_array(),
                }
            }),
        ).unwrap();
    }

    // The tessellated geometry is ready to be uploaded to the GPU.
    println!(" -- {} vertices {} indices",
        geometry.vertices.len(),
        geometry.indices.len()
    );
}
```

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
