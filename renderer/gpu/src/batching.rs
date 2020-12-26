use crate::{GpuInstance};
use glue::units::*;
use glue::{BufferId, PipelineKey, PipelineFeatures, BindGroupId};

use std::collections::HashMap;


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BatchDescriptor {
    pub pipeline: PipelineKey,
    pub ibo: BufferId,
    pub vbos: [Option<BufferId>; 4],
    pub bind_groups: [Option<BindGroupId>; 4],
    pub index_range: (u32, u32),
    pub base_vertex: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BatchKey {
    index: u32,
    pub features: PipelineFeatures,
}

impl BatchKey {
    fn is_compatible_with(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

/// Converts a large-ish batch descriptor into a simple ID valid for only one frame.
///
/// Also convenient to build the list of existing batch descriptors (without pipeline features).
pub struct BatchKeys {
    keys: HashMap<BatchDescriptor, u32>,
    descriptors: Vec<BatchDescriptor>,
    cached_descriptor: Option<BatchDescriptor>,
    cached_index: u32,
    next_index: u32,
}

impl BatchKeys {
    pub fn new() -> Self {
        BatchKeys {
            keys: HashMap::new(),
            descriptors: Vec::new(),
            cached_descriptor: None,
            cached_index: 0,
            next_index: 0,
        }
    }

    pub fn reset(&mut self) {
        self.keys.clear();
        self.descriptors.clear();
        self.cached_descriptor = None;
        self.cached_index = 0;
        self.next_index = 0;
    }

    pub fn get(&mut self, mut descriptor: BatchDescriptor) -> BatchKey {
        let features = descriptor.pipeline.features;
        descriptor.pipeline.features = PipelineFeatures::default();

        if self.cached_descriptor == Some(descriptor) {
            return BatchKey {
                index: self.cached_index,
                features,
            };
        }

        let next_index = &mut self.next_index;
        let descriptors = &mut self.descriptors;
        let idx = *self.keys.entry(descriptor).or_insert_with(|| {
            let idx = *next_index;
            *next_index += 1;
            descriptors.push(descriptor);

            idx
        });

        self.cached_descriptor = Some(descriptor);
        self.cached_index = idx;

        BatchKey {
            index: idx,
            features,
        }
    }

    pub fn get_batch_descriptor(&self, key: BatchKey) -> BatchDescriptor {
        let mut descriptor = self.descriptors[key.index as usize];
        descriptor.pipeline.features = key.features;

        descriptor
    }

    pub fn batch_descriptors(&self) -> &[BatchDescriptor] {
        &self.descriptors
    }
}


/// Some options to tune the behavior of the batching phase.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BatchingConfig {
    /// If false, all batches are ordered, including opaque ones.
    pub enable_order_independent_pass: bool,
    /// Don't aggressively merge batches until the number of batches is larger
    /// than this threshold.
    pub ideal_batch_count: usize,

    /// Don't aggressively merge batches if the sum of their cost is larger
    /// than this threshold.
    ///
    /// This allows reducing the likely hood that we'll use a very expensive
    /// shader for a very large amount of pixels.
    pub max_merge_cost: f32,

    /// Maximum amount of batches to go through when looking for a batch to
    /// assign a primitive to.
    pub max_lookback: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Stats {
    pub num_batches: u32,
    pub num_instances: u32,
    pub hit_lookback_limit: u32,
}

impl Stats {
    pub fn combine(&self, other: &Self) -> Self {
        Stats {
            num_batches: self.num_batches + other.num_batches,
            num_instances: self.num_instances + other.num_instances,
            hit_lookback_limit: self.hit_lookback_limit + other.hit_lookback_limit,
        }
    }
}

struct BatchRects {
    batch: LayerBox,
    items: Vec<LayerBox>,
}

impl BatchRects {
    fn new(rect: &LayerBox) -> Self {
        BatchRects {
            batch: *rect,
            items: Vec::new(),
        }
    }

    fn add_rect(&mut self, rect: &LayerBox) {
        let union = self.batch.union(rect);

        if !self.items.is_empty() {
            self.items.push(*rect);
        } else if self.batch.area() + rect.area() > union.area() {
            self.items.reserve(16);
            self.items.push(self.batch);
            self.items.push(*rect);
        }

        self.batch = union;
    }

    fn intersects(&mut self, rect: &LayerBox) -> bool {
        if !self.batch.intersects(rect) {
            return false;
        }

        if self.items.is_empty() {
            true
        } else {
            self.items.iter().any(|item| item.intersects(rect))
        }
    }
}

/// A list of batches that preserve the ordering of overlapping primitives. 
pub struct OrderedBatchList {
    batches: Vec<Batch>,
    rects: Vec<BatchRects>,
    max_lookback: usize,
    hit_lookback_limit: u32,
}

pub struct Batch {
    pub instances: Vec<GpuInstance>,
    pub key: BatchKey,
    pub cost: f32,
}

impl OrderedBatchList {
    pub fn new(config: &BatchingConfig) -> Self {
        OrderedBatchList {
            batches: Vec::new(),
            rects: Vec::new(),
            max_lookback: config.max_lookback,
            hit_lookback_limit: 0,
        }
    }

    pub fn add_instances(&mut self, key: &BatchKey, instances: &[GpuInstance], rect: &LayerBox) {
        let mut intersected = false;
        for (batch_index, batch) in self.batches.iter_mut().enumerate().rev().take(self.max_lookback) {
            let compatible = batch.key.is_compatible_with(key);
            if compatible {
                batch.key.features |= key.features;
                batch.instances.extend_from_slice(instances);
                self.rects[batch_index].add_rect(rect);
                batch.cost += rect.area();
                return;
            }

            if self.rects[batch_index].intersects(rect) {
                intersected = true;
                break;
            }
        }

        if !intersected && self.batches.len() > self.max_lookback {
            self.hit_lookback_limit += 1;
        }

        self.batches.push(Batch {
            key: *key,
            instances: instances.to_vec(),
            cost: rect.area(),
        });
        self.rects.push(BatchRects::new(rect));
    }

    pub fn add_batch(&mut self, key: &BatchKey, instances: &[GpuInstance], rect: &LayerBox) {
        self.batches.push(Batch {
            instances: instances.to_vec(),
            key: *key,
            cost: rect.area(),
        });
        self.rects.push(BatchRects::new(rect));
    }

    pub fn stats(&self) -> Stats {
        Stats {
            hit_lookback_limit: self.hit_lookback_limit,
            num_batches: self.batches.len() as u32,
            num_instances: self.batches.iter().fold(
                0,
                |count, batch| count + batch.instances.len() as u32,
            ),
        }
    }

    pub fn clear(&mut self) {
        self.batches.clear();
        self.rects.clear();
    }
}

/// A list of batches that don't preserve ordering.
///
/// Typically useful for fully opaque primitives when the depth-buffer is used for
/// occlusion culling.
pub struct OrderIndependentBatchList {
    batches: Vec<Batch>,
}

impl OrderIndependentBatchList {
    pub fn new(_config: &BatchingConfig) -> Self {
        OrderIndependentBatchList {
            batches: Vec::new(),
        }
    }

    pub fn add_instances(&mut self, key: &BatchKey, instances: &[GpuInstance], rect: &LayerBox) {
        for batch in self.batches.iter_mut().rev() {
            if batch.key.is_compatible_with(key) {
                batch.key.features |= key.features;
                batch.instances.extend_from_slice(instances);
                batch.cost += rect.area();
                return;
            }
        }

        self.batches.push(Batch {
            key: *key,
            instances: instances.to_vec(),
            cost: rect.area(),
        });
    }

    pub fn clear(&mut self) {
        self.batches.clear();
    }

    pub fn stats(&self) -> Stats {
        Stats {
            hit_lookback_limit: 0,
            num_batches: self.batches.len() as u32,
            num_instances: self.batches.iter().fold(
                0,
                |count, batch| count + batch.instances.len() as u32,
            ),
        }
    }
}

pub struct Batcher {
    enable_order_independent_pass: bool,
    order_independent: OrderIndependentBatchList,
    ordered: OrderedBatchList,
}


impl Batcher {
    pub fn new(config: &BatchingConfig) -> Self {
        Batcher {
            enable_order_independent_pass: config.enable_order_independent_pass,
            order_independent: OrderIndependentBatchList::new(config),
            ordered: OrderedBatchList::new(config),
        }
    }

    pub fn add_instances(&mut self, key: &BatchKey, instances: &[GpuInstance], rect: &LayerBox) {
        if self.enable_order_independent_pass && key.features.is_opaque() {
            self.order_independent.add_instances(key, instances, rect);
        } else {
            self.ordered.add_instances(key, instances, rect);
        }
    }

    pub fn add_instance(&mut self, key: &BatchKey, instance: GpuInstance, rect: &LayerBox) {
        self.add_instances(key, &[instance], rect);
    }

    pub fn order_independent_batches(&self) -> &[Batch] {
        &self.order_independent.batches
    }

    pub fn ordered_batches(&self) -> &[Batch] {
        &self.ordered.batches
    }

    pub fn clear(&mut self) {
        self.order_independent.clear();
        self.ordered.clear();
    }

    pub fn stats(&self) -> Stats {
        self.ordered.stats().combine(&self.order_independent.stats())
    }
}

use crate::{Registry, DrawState};

fn submit_batches<'l>(
    batches: &[Batch],
    batch_keys: &BatchKeys,
    registry: &'l Registry,
    state: &mut DrawState,
    pass: &mut wgpu::RenderPass<'l>,
) {
    for batch in batches {
        // TODO: copy instances to a gpu buffer and get the real instance range.
        let instance_range = 0..(batch.instances.len() as u32);

        let descriptor = batch_keys.get_batch_descriptor(batch.key);
        state.submit_batch(pass, registry, &descriptor, instance_range);
    }
}