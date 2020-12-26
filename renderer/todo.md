#TODO

 - SSBO resizes with copy
 - more efficient instancing (multiple quads per instance)
 - automatic bind group generation
 - something about multithreading command submission with the registries


# Resource management

It would be great to be able to both
 - register some resources that can use the automatic machinery for bind groups etc
 - store resources out of the the registry but still be able to use them in draw calls
 - have external code (for example embedder) be able to provide temporary access to their resources

Should buffers and other handles be stored as Arcs ?

How to make this multi-threaded ?
 - problem with automatic bind group generation and adding/removing resources



# Batching 

ideally: write instances directly into instance buffer instead of pushing into vectors and then assembling

problem: when starting a new batch we don't know how many instances compatible with previous batches we will encounter.

solution? could give some fixed headroom and seal the batch once its capacity is exhausted. having a separate instance buffer per pipeline kind would reduce the fragmentation

if culling is done on the gpu, each pipeline kind could write its instance buffer and the batcher would only generate instance ranges, splitting batches where other primitives interfere (although culling on cpu would help with avoiding some of these splits)


What about supporting custom instance formats (that the batcher doesn't know about and so can't build the instance buffer)?

would it be useful to deffer sub-resource allocation to during batching to avoid breaking batches in some cases ?
 - complicated



# GPU data storage

Separate buffer per type or serialized [f32; 4] / [u32; 4] storage (or a mix)?

float-only data: rects (4), transforms (8), img sources (8), 
uint-only data: instances, packed device rects

pros
 - can bundle allocations per group of primitives.
   - manage life-time per-bundle, do larger allocations (more efficient transfer)
 - fewer very large buffers, less bind groups to manage
 - 
cons:
 - larger address range (ids may not fit in u16 (max 65k items))
 - less "semantic", harder to debug in renderdoc
 - some things like rects would benefit from beeing stored contiguously to do gpu culling efficiently.
 - some systems can own their buffer and manage it completely.

data bundles can be collections of allocation handles to manually managed sub-buffer allocations to get some of the benefit of bundles without groups.


# Buffer sub-allocation

 use cases:
  - (A) long lived data, allocation and content is kept
  - (B) long lived allocation, data is updated every frame (probably same bucket as (A)).
  - (C) allocation and data change every frame

alloc lifetime:
 - manual
 - LRU cache


Tempting to have a separate agnostic LRU cache logic but if the LRU mechanism is part of the buffer allocator, we can do smarter things like only evicting when we actually need some room for new allocations.

Allocations:
 - buckets of fixed size (64?)
 - buckets of arbitrary size (rounded up to small bucket size)
 - single element ? (maybe start without support for that)
 - special single elements (hard coded handles like identity transform)


allocator:

large "pages" (1024 elts),
each page has chunks of a given size (16 elts)


# GPU uploads


 - pool of large-ish staging buffers split into chunks
 - pipe::Writer writing into a chunk
 - BufferWriter contains a pip::Writer + stagingBufferId, + BufferId + pointer to the gpu pipe

Gpu pipe contains an atomic linked list of mapped buffer with bump allocators. so that we only lock when
we need to allocate a new staging buffer.


# Batch submission

Either:
- (A) use a completely wgpu-agnostic data structure and tap into the registry
   - pipeline, ibo, vbos, bind_groups, buffer ranges, debug flags
- (B) or let the rendering systems register a callback
   - callback, + same parameters probably

(A) maybe more convenient because the batcher will spit out agnostic data already, we also need to keep track of already active resources anyway.

Is (A) too limiting ?

TODO: Replace BatchKey in the batchers with a binding ID + pipeline features. It should make batching a tad cheaper and perhaps more generic (not dependent on what fits in the batch key).
Also maybe use that to deal with the lifetime of bind groups instead of marking them as used for each primitive.
TODO: right now it takes the pipeline features in the key which it shouldn't do.


# Mesh format:

could split the vertices in fixed size chunks (meshlets) and store them in a SSBO instead of VBO. each instance would know a certain range of vertices to fetch.
 - very complex scenes in few draw calls
 - allows finer-grained occlusion culling
 - on the other hand we need to have an indirection to fetch vertices vertices, don't use indices and lose the benefits of the vertex cache.
 - can remove the index buffer and have sequences of triangle coordinates directly (simpler, takes more storage)
 
Mesh renderer:
 - vertex: x, y, sub_mesh_index   (submesh_index = sub_mesh_index + instance.sub_mesh_offset
 - SubMeshData (SSBO): transform, img_src, img_dst_rect, user_data
 - instance: transform, z, rect?, user_data, sub_mesh_offset

Meshlet renderer:
 - vertex: (nothing)
 - index: (nothing)
 - vtx_geometry (SSBO) x, y
 - meshlet data (SSBO): geom_offset, transform, img_src, imd_dst_rect, user_data,
 - 1 instance = 1 meshlet, vtx_addr = meshlet.geom_offset + vertex_index