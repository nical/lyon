# Lyon
GPU-based 2D graphics rendering experiments in rust.

<img src="assets/lyon-logo.png" width=500 height=500 alt="Project logo">

# Goals

For now the goal is to provide efficient SVG-compliant path tessellation tools to help with rendering vector graphics on the GPU. If things go well the project could eventually grow into including a (partial) SVG renderer in a separate crate, but for now think of this library as a way to turn complex paths into triangles for use in your own rendering engine.

The intent is for this library to be useful in projects like [Servo](https://servo.org/) and games.

The project is split into small crates:
* lyon ([documentation](https://nical.github.io/lyon-doc/lyon/)): A meta-crate that imports the other crates.
* lyon_tessellator ([documentation](https://nical.github.io/lyon-doc/lyon_tessellator/)): The tessellation routines (where most of the focus is for now).
* lyon_path_iterator ([documentation](https://nical.github.io/lyon-doc/lyon_path_iterator/)): A set of iterator abstractions over vector paths.
* lyon_path_builder ([documentation](https://nical.github.io/lyon-doc/lyon_path_builder/)): Tools to build paths.
* lyon_path ([documentation](https://nical.github.io/lyon-doc/lyon_path/)): A simple vector path data structure provided for convenience, but not required by the other crates.
* lyon_bezier ([documentation](https://nical.github.io/lyon-doc/lyon_bezier/)): 2d quadratic and cubic bezier curve maths, including an efficient flattening algorithm.
* lyon_core ([documentation](https://nical.github.io/lyon-doc/lyon_core/)): Contains types common to most lyon crates.
* lyon_extra ([documentation](https://nical.github.io/lyon-doc/lyon_extra/)): various optional utilities.

## Documentation

* [Link to the documentation](nical.github.io/lyon-doc/lyon)
* The documentation can be generated locally by running ```cargo doc``` at the root of the repository.

## Status

The focus right now is on implementing a SVG compliant path tessellator (rather than an actual SVG render).

- path
  - [x] bezier curves (through path flattening)
  - [x] SVG 1.1
  - [x] builder API
  - [x] iterator API
- complex fill
  - [x] fill shape types
    - [x] concave shapes
    - [x] self-intersections
    - [x] holes
  - [ ] fill rule
    - [x] even-odd
    - [ ] non-zero
  - [ ] vertex-aa
  - [ ] clip rect
  - [ ] stable API
- complex stroke
  - [ ] line cap
    - [x] butt
    - [x] square
    - [ ] round
  - [ ] line join
    - [ ] miter
    - [ ] miter clip
    - [ ] round
    - [ ] bevel
    - [ ] arcs
  - [ ] vertex-aa
  - [ ] clip rect
  - [ ] stable API
- basic shapes
  - [ ] quad
    - [x] fill
    - [ ] stroke
  - [ ] rectangle
    - [x] fill
    - [ ] stroke
  - [ ] rounded rectangle
    - [ ] fill
    - [ ] stroke
  - [x] ellipsis
    - [x] fill
    - [ ] stroke
  - [ ] convex polygon
    - [ ] fill
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

