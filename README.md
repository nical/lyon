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
</p>

# Motivation

For now the goal is to provide efficient SVG-compliant path tessellation tools to help with rendering vector graphics on the GPU. If things go well the project could eventually grow into including a (partial) SVG renderer in a separate crate, but for now think of this library as a way to turn complex paths into triangles for use in your own rendering engine.

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
## Documentation

* [Link to the documentation](https://nical.github.io/lyon-doc/lyon/index.html)
* The documentation can be generated locally by running ```cargo doc``` at the root of the repository.

## Structure

The project is split into small crates:
* lyon ([documentation](https://nical.github.io/lyon-doc/lyon/index.html)): A meta-crate that imports the other crates.
* lyon_tessellator ([documentation](https://nical.github.io/lyon-doc/lyon_tessellator/index.html)): The tessellation routines (where most of the focus is for now).
* lyon_path_iterator ([documentation](https://nical.github.io/lyon-doc/lyon_path_iterator/index.html)): A set of iterator abstractions over vector paths.
* lyon_path_builder ([documentation](https://nical.github.io/lyon-doc/lyon_path_builder/index.html)): Tools to build paths.
* lyon_path ([documentation](https://nical.github.io/lyon-doc/lyon_path/)): A simple vector path data structure provided for convenience, but not required by the other crates.
* lyon_bezier ([documentation](https://nical.github.io/lyon-doc/lyon_bezier/index.html)): 2d quadratic and cubic bezier curve maths, including an efficient flattening algorithm.
* lyon_core ([documentation](https://nical.github.io/lyon-doc/lyon_core/index.html)): Contains types common to most lyon crates.
* lyon_extra ([documentation](https://nical.github.io/lyon-doc/lyon_extra/index.html)): various optional utilities.

There is also a toy [command-line tool](cli) exposing to tessellate SVG path from your favorite terminal.

Have a look at the [gfx-rs example](examples/gfx_logo) to see how integrating the tessellators in a renderer can look like.

## Status

The focus right now is on implementing a SVG compliant path tessellator (rather than an actual SVG render).

- path
  - [x] bezier curves (through path flattening)
  - [x] SVG 1.1
  - [x] builder API
  - [x] iterator API
- complex fills
  - [x] fill shape types
    - [x] concave shapes
    - [x] self-intersections
    - [x] holes
  - [ ] fill rules
    - [x] even-odd
    - [ ] non-zero
  - [ ] vertex-aa
  - [ ] clip rect
  - [ ] stable API
- complex strokes
  - [ ] line caps
    - [x] butt
    - [x] square
    - [ ] round
  - [ ] line joins
    - [ ] miter
    - [ ] miter clip
    - [ ] round
    - [ ] bevel
    - [ ] arcs
  - [ ] vertex-aa
  - [ ] clip rect
  - [ ] stable API
- basic shapes
  - [x] quad
    - [x] fill
    - [x] stroke
  - [x] rectangle
    - [x] fill
    - [x] stroke
  - [ ] rounded rectangle
    - [ ] fill
    - [ ] stroke
  - [x] ellipsis
    - [x] fill
    - [ ] stroke
  - [ ] convex polygons
    - [x] fill
    - [ ] stroke
  - [ ] nine-patch
- path flattening
  - [x] builder
  - [x] iterator
- testing
  - [x] fill
    - [x] test suite
    - [x] automatic test-case reduction
    - [ ] reference testing
    - [ ] fuzzing
  - [ ] stroke
  - [ ] basic shapes


## TODO

There are the unticked items above as well as a [rough list of things to do](https://github.com/nical/lyon/wiki/TODO). If you are interested in [contributing](https://github.com/nical/lyon/wiki/Contribute), please let me know on twitter ([@nicalsilva](https://twitter.com/nicalsilva)) or by e-mail.


## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

