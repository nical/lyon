use loader::*;
use drive::{DriveLoader};
use identifier::*;
use std::collections::HashMap;

pub type Callback = Box<FnOnce(Vec<u8>)+'static>;
pub type CallbackMap = HashMap<ResourceId, Callback>;

pub struct Metrics {
    pub bytes_loaded: u64,
    pub resources_loaded: u64,
}

pub struct ResourceManager {
    drive_loader: ResourceLoader<DriveLoader>,
    drive_metrics: Metrics,

    callbacks: CallbackMap,
}

impl ResourceManager {
    pub fn new() -> ResourceManager {
        ResourceManager {
            drive_loader: ResourceLoader::new(),
            drive_metrics : Metrics {
                bytes_loaded: 0,
                resources_loaded: 0,
            },
            callbacks: HashMap::new(),
        }
    }

    pub fn load(&mut self, id: ResourceId, callback: Callback) {
        self.callbacks.insert(id, callback);
        self.drive_loader.load_resource(id);
    }

    pub fn update(&mut self) -> bool {
        match self.drive_loader.poll_response() {
            Some(Response::Loaded(id, data)) => {
                match self.callbacks.remove(&id) {
                    Some(cb) => {
                        //cb.call_once((data,));
                        return true;
                    }
                    None => { panic!("No callback registered for resource {:?}", id); }
                }
            }
            _ => { panic!("TODO"); }
        }
        return false;
    }
    pub fn shut_down(&self) {
        self.drive_loader.shut_down();
    }
}

