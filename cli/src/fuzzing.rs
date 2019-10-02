use lyon::math::*;
use lyon::path::Path;
use lyon::tessellation::geometry_builder::NoOutput;
use lyon::tessellation::{
    StrokeOptions, StrokeTessellator,
    FillOptions, FillTessellator,
    OnError,
};
use lyon::extra::debugging::find_reduced_test_case;
use rand;
use commands::{FuzzCmd, Tessellator};
use std::cmp::{min, max};
use lyon::tess2;
#[cfg(feature="experimental")]
use lyon::tessellation::experimental;

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

        path.move_to(random_point());
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
    return path.build();
}

pub fn run(cmd: FuzzCmd) -> bool {
    let mut i: u64 = 0;
    println!("----");
    println!(
        "Fuzzing {} tessellation:",
        match (cmd.fill, cmd.stroke) {
            (true, true) => "fill and stroke",
            (_, true) => "stroke",
            _ => "fill",
        }
    );
    if let Some(num) = cmd.min_points {
        println!("minimum number of points per path: {}", num);
    }
    if let Some(num) = cmd.max_points {
        println!("maximum number of points per path: {}", num);
    }
    println!("----");
    loop {
        let path = generate_path(&cmd, i);
        if cmd.fill || !cmd.stroke {
            let status = ::std::panic::catch_unwind(|| {
                let options = FillOptions::default().on_error(
                    if cmd.ignore_errors {
                        OnError::Recover
                    } else {
                        OnError::Panic
                    }
                );
                match cmd.tessellator {
                    Tessellator::Default => {
                        let result = FillTessellator::new().tessellate_path(
                            &path,
                            &options,
                            &mut NoOutput::new()
                        );
                        if !cmd.ignore_errors {
                            result.unwrap();
                        }
                    }
                    Tessellator::Tess2 => {
                        let result = tess2::FillTessellator::new().tessellate_path(
                            &path,
                            &options,
                            &mut NoOutput::new()
                        );
                        if !cmd.ignore_errors {
                            result.unwrap();
                        }
                    }
                    Tessellator::Experimental => {
                        #[cfg(feature="experimental")]
                        let result = experimental::FillTessellator::new().tessellate_path(
                            &path,
                            &options,
                            &mut NoOutput::new()
                        );
                    }
                }
            });

            if status.is_err() {
                println!(" !! Error while tessellating");
                println!("    Path #{} containing {} points", i, path.points().len());
                find_reduced_test_case(
                    path.as_slice(),
                    &|path: Path| {
                        FillTessellator::new().tessellate_path(
                            &path,
                            &FillOptions::default(),
                            &mut NoOutput::new()
                        ).is_err()
                    },
                );

                panic!("aborting");
            }
        }
        if cmd.stroke {
            StrokeTessellator::new().tessellate_path(
                &path,
                &StrokeOptions::default(),
                &mut NoOutput::new()
            ).unwrap();
        }
        i += 1;
        if i % 500 == 0 {
            println!(" -- tested {} paths (~{} points per path)", i, path.points().len());
        }
    }
}
