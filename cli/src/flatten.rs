use commands::PathCmd;
use lyon::path::iterator::*;
use lyon::path::FlattenedEvent;
use std::io;

#[derive(Debug)]
pub enum FlattenError {
    Io(io::Error),
}

impl ::std::convert::From<::std::io::Error> for FlattenError {
    fn from(err: io::Error) -> Self { FlattenError::Io(err) }
}

pub fn flatten(mut cmd: PathCmd) -> Result<(), FlattenError> {
    if !cmd.flatten {
        // TODO: implement more transformations.
        return Ok(());
    }
    if cmd.count {
        // TODO: when flatten is false we should count vertices, curves, etc.
        let mut num_paths = 0;
        let mut num_vertices = 0;
        for event in cmd.path.iter().flattened(cmd.tolerance) {
            match event {
                FlattenedEvent::Begin { .. } => {
                    num_vertices += 1;
                    num_paths += 1;
                }
                FlattenedEvent::Line { .. } => {
                    num_vertices += 1;
                }
                FlattenedEvent::End { .. } => {}
            }
        }

        try!{ writeln!(&mut *cmd.output, "vertices: {}", num_vertices) };
        try!{ writeln!(&mut *cmd.output, "paths: {}", num_paths) };

        return Ok(());
    }

    for event in cmd.path.iter().flattened(cmd.tolerance) {
        match event {
            FlattenedEvent::Begin { at } => {
                try!{ write!(&mut *cmd.output, "M {} {} ", at.x, at.y) };
            }
            FlattenedEvent::Line { to, .. } => {
                try!{ write!(&mut *cmd.output, "L {} {} ", to.x, to.y) };
            }
            FlattenedEvent::End { close: true, .. } => {
                try!{ write!(&mut *cmd.output, "Z") };
            }
            _=> {}
        }
    }
    try!{ writeln!(&mut *cmd.output, "") };

    Ok(())
}
