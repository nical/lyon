use crate::commands::FuzzCmd;
use lyon::algorithms::hatching::*;
use lyon::extra::debugging::find_reduced_test_case;
use lyon::geom::LineSegment;
use lyon::math::*;
use lyon::path::Path;
use lyon::tessellation::geometry_builder::NoOutput;
use lyon::tessellation::{FillTessellator, StrokeTessellator};
use std::cmp::{max, min};

fn random_point() -> Point {
    point(
        rand::random::<f32>() * 1000.0,
        rand::random::<f32>() * 1000.0,
    )
}

fn generate_path(cmd: &FuzzCmd, iteration: u64) -> Path {
    let mut path = Path::builder();

    let min_points = cmd.min_points.unwrap_or(5);
    let max_points = max(min_points, cmd.max_points.unwrap_or(5_000));
    let diff = max_points - min_points;

    let target = min_points + min(diff, (iteration / 5000) as u32);

    let mut num_points = 0;
    loop {
        let num_cmds = 3 + rand::random::<u32>() % (target - num_points);

        path.begin(random_point());
        num_points += 1;
        for _ in 0..num_cmds {
            path.line_to(random_point());
            num_points += 1;
        }
        path.close();

        if num_points >= target {
            break;
        }
    }

    path.build()
}

pub fn run(cmd: FuzzCmd) -> bool {
    let mut i: u64 = 0;
    println!("----");
    println!(
        "Fuzzing {} tessellation:",
        match (cmd.tess.fill, cmd.tess.stroke) {
            (Some(..), Some(..)) => "fill and stroke",
            (_, Some(..)) => "stroke",
            _ => "fill",
        }
    );
    if let Some(num) = cmd.min_points {
        println!("minimum number of points per path: {num}");
    }
    if let Some(num) = cmd.max_points {
        println!("maximum number of points per path: {num}");
    }
    println!("----");
    loop {
        let path = generate_path(&cmd, i);
        if let Some(options) = cmd.tess.fill {
            let status = ::std::panic::catch_unwind(|| {
                let result =
                    FillTessellator::new().tessellate(&path, &options, &mut NoOutput::new());
                if !cmd.ignore_errors {
                    result.unwrap();
                }
            });

            if status.is_err() {
                println!(" !! Error while tessellating");
                println!("    Path #{i}");
                find_reduced_test_case(path.as_slice(), &|path: Path| {
                    FillTessellator::new()
                        .tessellate(&path, &options, &mut NoOutput::new())
                        .is_err()
                });

                panic!("aborting");
            }
        }
        if let Some(options) = cmd.tess.stroke {
            StrokeTessellator::new()
                .tessellate(&path, &options, &mut NoOutput::new())
                .unwrap();
        }

        if let Some(ref hatch) = cmd.tess.hatch {
            let mut builder = Path::builder();
            let mut hatcher = Hatcher::new();
            hatcher.hatch_path(
                path.iter(),
                &hatch.options,
                &mut RegularHatchingPattern {
                    interval: hatch.spacing,
                    callback: &mut |segment: &HatchSegment| {
                        builder.add_line_segment(&LineSegment {
                            from: segment.a.position,
                            to: segment.b.position,
                        });
                    },
                },
            );
            let _hatched_path = builder.build();
        }

        if let Some(ref dots) = cmd.tess.dots {
            let mut builder = Path::builder();
            let mut hatcher = Hatcher::new();
            hatcher.dot_path(
                path.iter(),
                &dots.options,
                &mut RegularDotPattern {
                    row_interval: dots.spacing,
                    column_interval: dots.spacing,
                    callback: &mut |dot: &Dot| {
                        builder.add_point(dot.position);
                    },
                },
            );
            let _dotted_path = builder.build();
        }

        i += 1;
        if i % 500 == 0 {
            println!(" -- tested {i} paths");
        }
    }
}
