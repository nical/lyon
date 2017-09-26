# High level renderer

Still very much experimental, do not use.

# Rough plan so far

- Use the depth buffer
    - render opaque primitive (mostly) front to back (depth buffer read and write)
    - render transparent primitive back to front with (depth buffer read only)
- Automatically batch and sort primitives
- Keep the geometry in GPU memory, store primitive data in uniform buffers (or 1D texture).
- Differentiate between static and dynamic value in order to optimize transferring dynamic properties to the GPU and aid batching heuristics.
- Expose instanced rendering in the API.
