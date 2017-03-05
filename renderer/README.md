# High level renderer

Still very much experimental, do not use.

# Rough plan so far

- Use the depth buffer
    - render opaque primitive (mostly) front to back (depth buffer read and write)
    - render transparent primitive back to front with (depth buffer read only)
- Automatically batch and sort primitives
- Keep the geometry in GPU memory, store primitive data in uniform buffers (or 1D texture).
- Differentiate between static and dynamic value in order to optimize transferring dynamic properties to the GPU and aid batching heuristics.

The API could maybe look something like that

```
let path_id = context.add_path(path, PropertyFlags::default());
let image_id = context.add_image(image, PropertyFlags::default());
let transform_id = context.add_transform(
    translation(100.0, 100.0),
    PropertyFlags::dynamic()
);

scene.render_shape(RenderNode {
    shape_id: ShapeId::Path(path_id),
    transform: transform_id,
    fill: Some(FillStyle{
        pattern: ImagePattern {
            image_id: image_id,
            image_rect: image.get_bounds(),
        },
        flags: RenderFlags::default(),
    }),
    stroke: None,
});

context.set_scene(scene);

context.render(&mut device);

// Change a transform and render the scene again.
context.update_transform(transform_id, translation(200.0, 100.0));
context.render(&mut device);

```
