use crate::commands::{TessellateCmd, Tessellator};
use lyon::path::traits::*;
use lyon::path::Path;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{FillTessellator, StrokeTessellator};

pub fn reduce_testcase(cmd: TessellateCmd) {
    if let Some(options) = cmd.stroke {
        let mut flattener = Path::builder().flattened(options.tolerance);
        flattener.extend(cmd.path.iter());
        let path = flattener.build();

        lyon::extra::debugging::find_reduced_test_case(path.as_slice(), &|path: Path| {
            StrokeTessellator::new()
                .tessellate_path(&path, &options, &mut NoOutput::new())
                .is_err()
        });
    }

    if let Some(_) = cmd.hatch {
        unimplemented!();
    }

    if let Some(_) = cmd.dots {
        unimplemented!();
    }

    if let Some(options) = cmd.fill {
        let mut flattener = Path::builder().flattened(options.tolerance);
        flattener.extend(cmd.path.iter());
        let path = flattener.build();

        match cmd.tessellator {
            Tessellator::Default => {
                lyon::extra::debugging::find_reduced_test_case(path.as_slice(), &|path: Path| {
                    FillTessellator::new()
                        .tessellate_path(&path, &options, &mut NoOutput::new())
                        .is_err()
                });
            }
            Tessellator::Tess2 => {
                unimplemented!();
            }
        }
    }
}
