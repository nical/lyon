# Lyon
2D graphics rendering experiments in rust.

<img src="assets/screenshot.png" width=500 height=500 alt="Screenshot of the Rust logo (svg) tesselated">

For now the goal is to provide efficient path tesselation tools to help with rendering vector graphics on the GPU.

The project is split into small crates:
* lyon: A meta-crate that imports the other crates.
* lyon_core: Contains types common to most lyon crates.
* lyon_tesselator: The tesselation routines (where most of the focus is for now).
* lyon_extra: various optional utilities.

TODO:
* lyon_renderer: A scene-graph API to render complex 2d graphics.
* lyon_glium: A glium backend for lyon_renderer.
* other backends ?

Rendering fonts is out of scope for now.

## Status

The tesselator can currently only operate on flattened paths. It is able to handle some complex cases including self intersecting paths, but there are still some bugs that need to be found and fixed. The API is not stable at all.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

