use commands::{TessellateCmd, Tessellator};
use lyon::path::Path;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{FillTessellator, StrokeTessellator};

pub fn reduce_testcase(cmd: TessellateCmd) {
    if let Some(options) = cmd.stroke {
        lyon::extra::debugging::find_reduced_test_case(cmd.path.as_slice(), &|path: Path| {
            StrokeTessellator::new().tessellate_path(
                &path,
                &options,
                &mut NoOutput::new(),
            ).is_err()
        });
    }

    if let Some(_) = cmd.hatch {
        unimplemented!();
    }

    if let Some(_) = cmd.dots {
        unimplemented!();
    }

    if let Some(options) = cmd.fill {
        match cmd.tessellator {
            Tessellator::Default => {
                lyon::extra::debugging::find_reduced_test_case(cmd.path.as_slice(), &|path: Path| {
                    FillTessellator::new().tessellate_path(
                        &path,
                        &options,
                        &mut NoOutput::new(),
                    ).is_err()
                });
            }
            Tessellator::Tess2 => {
                unimplemented!();
            }
        }
    }
}
