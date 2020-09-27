use std::collections::HashMap;
use std::num::NonZeroU32;

use glue::{PipelineKind, PipelineKey, PipelineFeatures};

pub struct Pipelines {
    kinds: HashMap<PipelineKind, PipelineKindEntry>,
    next_key: u32,
}

struct PipelineKindEntry {
    pipelines: Vec<Pipeline>,
}

struct Pipeline {
    handle: wgpu::RenderPipeline,
    features: PipelineFeatures,
}

impl Pipelines {
    pub fn new() -> Self {
        Pipelines {
            kinds: HashMap::new(),
            next_key: 1,
        }
    }

    pub fn get_compatible_pipeline(&self, key: PipelineKey) -> Option<(&wgpu::RenderPipeline, PipelineKey)> {
        let kind = self.kinds.get(&key.kind)?;

        for p in &kind.pipelines {
            if p.features & key.features == key.features {
                return Some((&p.handle, PipelineKey { features: p.features, .. key }));
            }
        }

        None
    }

    pub fn add_pipeline_kind(&mut self) -> PipelineKind {
        let key = PipelineKind(NonZeroU32::new(self.next_key).unwrap());

        self.next_key += 1;

        key
    }

    pub fn add_pipeline(&mut self, key: PipelineKey, handle: wgpu::RenderPipeline) {
        let kind = self.kinds
            .entry(key.kind)
            .or_insert(PipelineKindEntry { pipelines: Vec::new() });

        kind.pipelines.push(Pipeline {
            handle,
            features: key.features,
        });
    }

    pub fn remove_pipeline(&mut self, key: PipelineKey) {
        if let Some(kind) = self.kinds.get_mut(&key.kind) {
            kind.pipelines.retain(|p| p.features != key.features);
        }
    }

    pub fn remove_pipeline_kind(&mut self, key: PipelineKind) {
        self.kinds.remove(&key);
    }

    pub fn sort_kind_by(&mut self, key: PipelineKind, f: &dyn Fn(PipelineFeatures, PipelineFeatures) -> std::cmp::Ordering) {
        if let Some(kind) = self.kinds.get_mut(&key) {
            kind.pipelines.sort_by(|p1, p2| f(p1.features, p2.features));
        }
    }
}
