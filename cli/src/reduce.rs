use crate::commands::TessellateCmd;
use lyon::path::Path;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{FillTessellator, StrokeTessellator};

pub fn reduce_testcase(cmd: TessellateCmd) {
    if let Some(options) = cmd.stroke {
        let mut flattener = Path::builder().flattened(options.tolerance);
        for evt in cmd.path.iter() {
            flattener.path_event(evt);
        }
        let path = flattener.build();

        lyon::extra::debugging::find_reduced_test_case(path.as_slice(), &|path: Path| {
            StrokeTessellator::new()
                .tessellate_path(&path, &options, &mut NoOutput::new())
                .is_err()
        });
    }

    if cmd.hatch.is_some() {
        unimplemented!();
    }

    if cmd.dots.is_some() {
        unimplemented!();
    }

    if let Some(options) = cmd.fill {
        let mut flattener = Path::builder().flattened(options.tolerance);
        for evt in cmd.path.iter() {
            flattener.path_event(evt);
        }
        let path = flattener.build();

        lyon::extra::debugging::find_reduced_test_case(path.as_slice(), &|path: Path| {
            FillTessellator::new()
                .tessellate_path(&path, &options, &mut NoOutput::new())
                .is_err()
        });
    }
}
