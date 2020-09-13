use std::collections::HashMap;

pub type PipelineFeatures = u32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    family: PipelineFamilyKey,
    features: PipelineFeatures,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PipelineFamilyKey {
    key: u32,
}

impl PipelineFamilyKey {
    pub fn with_features(&self, features: PipelineFeatures) -> PipelineKey {
        PipelineKey {
            family: *self,
            features,
        }
    }
}

pub struct Pipelines {
    families: HashMap<u32, PipelineFamily>,
    next_key: u32,
}

struct PipelineFamily {
    pipelines: Vec<Pipeline>,
}

struct Pipeline {
    handle: wgpu::RenderPipeline,
    features: PipelineFeatures,
}

impl Pipelines {
    pub fn get_compatible_pipeline(&self, key: PipelineKey) -> Option<&wgpu::RenderPipeline> {
        let family = self.families.get(&key.family.key)?;

        for p in &family.pipelines {
            if p.features & key.features == key.features {
                return Some(&p.handle);
            }
        }

        None
    }

    pub fn add_pipeline_family(&mut self) -> PipelineFamilyKey {
        let key = PipelineFamilyKey {
            key: self.next_key,
        };

        self.next_key += 1;

        key
    }

    pub fn add_pipeline(&mut self, handle: wgpu::RenderPipeline, key: PipelineKey) {
        let family = self.families
            .entry(key.family.key)
            .or_insert(PipelineFamily { pipelines: Vec::new() });

        family.pipelines.push(Pipeline {
            handle,
            features: key.features,
        });
    }

    pub fn remove_pipeline(&mut self, key: PipelineKey) {
        if let Some(family) = self.families.get_mut(&key.family.key) {
            family.pipelines.retain(|p| p.features != key.features);
        }
    }

    pub fn remove_pipeline_family(&mut self, key: PipelineFamilyKey) {
        self.families.remove(&key.key);
    }

    pub fn sort_family_by(&mut self, key: PipelineFamilyKey, f: &dyn Fn(PipelineFeatures, PipelineFeatures) -> std::cmp::Ordering) {
        if let Some(family) = self.families.get_mut(&key.key) {
            family.pipelines.sort_by(|p1, p2| f(p1.features, p2.features));
        }
    }
}
