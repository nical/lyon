
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;
use std::collections::HashMap;

use identifier::*;

#[derive(Copy, Show, PartialEq)]
pub enum LoaderError {
    InvalidFormat,
    InvalidId,
    IOError,
    UnsupportedResourceType,
    UnknownError,
}

pub trait IoHandler {
    fn load(&mut self, id: ResourceId) -> Result<Vec<u8>, LoaderError>;
    fn new() -> Self;
    fn shut_down(&mut self);
}

#[derive(Copy, Show)]
pub enum Request {
    Load(ResourceId),
    Stop,
}

pub enum Response {
    Loaded(ResourceId, Vec<u8>),
    Error(LoaderError),
    Stopped,
}

pub struct ResourceLoader<T:IoHandler> {
    sender: Sender<Request>,
    receiver: Receiver<Response>,
}

fn loader_thread<T: IoHandler>(receiver: Receiver<Request>, sender: Sender<Response>) {
    let mut handler: T = IoHandler::new();
    loop {
        match receiver.recv().unwrap() {
            Request::Load(resource_id) => {
                let data = handler.load(resource_id);
                match data {
                    Ok(buffer) => {
                        sender.send(Response::Loaded(resource_id, buffer));
                    }
                    Err(e) => {
                        sender.send(Response::Error(e));
                    }
                }
            }
            Request::Stop => { break; }
        }
    }
    handler.shut_down();
    sender.send(Response::Stopped);
}


impl<T: IoHandler> ResourceLoader<T> {

    pub fn new() -> ResourceLoader<T> {
        let (ms, lr) = channel();
        let (ls, mr) = channel();

        Thread::spawn(move|| { loader_thread::<T>(lr, ls) });

        return ResourceLoader {
            sender: ms,
            receiver: mr,
        };
    }

    pub fn load_resource(&self, resource: ResourceId) {
        self.sender.send(Request::Load(resource));
    }

    pub fn poll_response(&self) -> Option<Response> {
        return match self.receiver.try_recv() {
            Ok(response) => { Some(response) }
            Err(_) => { None }
        };
    }

    pub fn wait_for_response(&self) -> Option<Response> {
        return match self.receiver.recv() {
            Ok(response) => { Some(response) }
            Err(_) => { None }
        };
    }

    pub fn shut_down(&self) {
        self.sender.send(Request::Stop);
        loop {
            match self.receiver.recv() {
                Ok(response) => {
                    match response {
                        Response::Stopped => { break; }
                        _ => {}
                    }
                }
                _ => { panic!("Error during the loader shutdown"); }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use identifier::*;

    struct DummyIoHandler { print: bool }

    impl IoHandler for DummyIoHandler {

        fn load(&mut self, id: ResourceId) -> Result<Vec<u8>, LoaderError> {
            if self.print { println!(" -- received request {:?}", id.name); }
            return Err(LoaderError::UnsupportedResourceType);
        }
        fn new() -> DummyIoHandler {
            let print = false;
            if print { println!(" -- starting up loader"); }
            return DummyIoHandler { print: print };
        }
        fn shut_down(&mut self) {
            if self.print { println!(" -- shutting down loader"); }
        }
    }

    #[test]
    fn test_loader() {
        let mut loader = ResourceLoader::<DummyIoHandler>::new();

        let mut num_requests: u64 = 100;

        for i in (0 .. num_requests) {
            loader.load_resource(resource_id(ResourceType::Image, Name{ hash: i }));
        }

        loop {
            match loader.poll_response() {
                Some(r) => {
                    match r {
                        Response::Error(LoaderError::UnsupportedResourceType) => {}
                        _ => { panic!("unexpected response"); }
                    }
                    num_requests -= 1;
                }
                None => {}
            }
            if num_requests < 1 { break; }
        }

        loader.shut_down();
    }
}

