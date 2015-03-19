
use loader::*;
use identifier::*;

use std::io::{File};

pub struct DriveLoader;

// Handles loading content from the hard drive.
impl IoHandler for DriveLoader {
    fn load(&mut self, id: ResourceId) -> Result<Vec<u8>, LoaderError> {
        let path = Path::new(id.to_string());
        let mut file = match File::open(&path) {
            Ok(f) => { f }
            Err(_) => { return Err(LoaderError::IOError); }
        };

        return match file.read_to_end() {
            Ok(bytes) => { Ok(bytes) }
            Err(e) => { return Err(LoaderError::IOError); }
        };
    }

    fn new() -> Self { DriveLoader }

    fn shut_down(&mut self) {}
}

#[test]
fn load_text_from_drive() {

    let mut loader = ResourceLoader::<DriveLoader>::new();

    let id = resource_id(ResourceType::PlainText, Name::from_str("load-test").unwrap());

    loader.load_resource(id);

    match loader.wait_for_response().unwrap() {
        Response::Loaded(resource_id, resource) => {
            assert_eq!(iresource_d, buffer);
            match id.resource_type {
                ResourceType::PlainText => {
                    assert_eq!(
                        String::from_utf8(buffer).unwrap(),
                        "test load_text_from_drive in drive.rs\n"
                    );
                }
                _ => { panic!("Unexpected resource type"); }
            }
        }
        Response::Error(err) => {
            panic!("Error while loading: {:?}", err);
        }
        _ => {
            panic!("Unexpected response");
        }
    }

    loader.shut_down();
}