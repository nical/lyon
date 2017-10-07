extern crate clap;
extern crate lyon;
extern crate lyon_extra;
extern crate rand;

mod commands;
mod tessellate;
mod fuzzing;
mod flatten;

use clap::*;
use commands::*;

use std::fs::File;
use std::io::{Read, Write, stdout, stderr};
use lyon::svg::parser::build_path;
use lyon::path::Path;
use lyon::path_builder::*;
use lyon_extra::debugging::find_reduced_test_case;

fn main() {
    let matches = App::new("Lyon command-line interface")
        .version("0.1")
        .author("Nicolas Silva <nical@fastmail.com>")
        .about("Path tessellator")
        .subcommand(SubCommand::with_name("tessellate")
            .about("Tessellates a path")
            .arg(Arg::with_name("DEBUG")
                .short("d")
                .long("debug")
                .help("Enable debugging")
            )
            .arg(Arg::with_name("FILL")
                .short("f")
                .long("fill")
                .help("Fills the path")
            )
            .arg(Arg::with_name("STROKE")
                .short("s")
                .long("stroke")
                .help("Strokes the path")
                .value_name("STROKE_WIDTH")
                .takes_value(true)
            )
            .arg(Arg::with_name("TOLERANCE")
                .short("t")
                .long("tolerance")
                .help("Sets the tolerance threshold for flattening (0.5 by default)")
                .value_name("TOLERANCE")
                .takes_value(true)
            )
            .arg(Arg::with_name("COUNT")
                .short("c")
                .long("count")
                .help("Prints the number of triangles and vertices")
            )
        )
        .subcommand(SubCommand::with_name("flatten")
            .about("Flattens a path")
            .arg(Arg::with_name("TOLERANCE")
                .short("t")
                .long("tolerance")
                .help("Sets the tolerance threshold (0.5 by default)")
                .value_name("TOLERANCE")
                .takes_value(true)
            )
            .arg(Arg::with_name("COUNT")
                .short("c")
                .long("count")
                .help("Prints the number of vertices")
            )
        )
        .subcommand(SubCommand::with_name("fuzz")
            .about("tessellates random paths in order to find potential bugs")
            .arg(Arg::with_name("FILL")
                .short("f")
                .long("fill")
                .help("Fills the path")
            )
            .arg(Arg::with_name("STROKE")
                .short("s")
                .long("stroke")
                .help("Strokes the path")
                .value_name("STROKE_WIDTH")
                .takes_value(true)
            )
            .arg(Arg::with_name("MAX_POINTS")
                .long("max-points")
                .help("Sets the maximum number of points per paths")
                .value_name("MAX_POINTS")
                .takes_value(true)
            )
            .arg(Arg::with_name("MIN_POINTS")
                .long("min-points")
                .help("Sets the minimum number of points per paths")
                .value_name("MIN_POINTS")
                .takes_value(true)
            )
        )
        .arg(Arg::with_name("PATH")
            .value_name("PATH")
            .help("An SVG path")
            .takes_value(true)
            .required(false)
        )
        .arg(Arg::with_name("INPUT")
            .short("i")
            .long("input")
            .help("Sets the input file to use")
            .value_name("FILE")
            .takes_value(true)
            .required(false)
        )
        .arg(Arg::with_name("OUTPUT")
            .short("o")
            .long("output")
            .help("Sets the output file to use")
            .value_name("FILE")
            .takes_value(true)
            .required(false)
        )
        .get_matches();

    let mut input_buffer = matches.value_of("PATH").unwrap_or(&"").to_string();

    if let Some(input_file) = matches.value_of("INPUT") {
        if let Ok(mut file) = File::open(input_file) {
            file.read_to_string(&mut input_buffer).unwrap();
        } else {
            write!(&mut stderr(), "Cannot open file {}", input_file).unwrap();
            return;
        }
    }

    let mut output: Box<Write> = Box::new(stdout());

    if let Some(output_file) = matches.value_of("OUTPUT") {
        if let Ok(file) = File::create(output_file) {
            output = Box::new(file);
        }
    }

    if let Some(tess_matches) = matches.subcommand_matches("tessellate") {
        let fill_cmd = tess_matches.is_present("FILL");
        let stroke_cmd = get_stroke(tess_matches);
        let path = build_path(Path::builder().with_svg(), &input_buffer).unwrap();
        let fill = fill_cmd || (!fill_cmd && !stroke_cmd.is_some());
        let tolerance = get_tolerance(&tess_matches);


        let cmd = TessellateCmd {
            path: path,
            fill: fill,
            stroke: stroke_cmd,
            tolerance: tolerance,
        };

        let res = ::std::panic::catch_unwind(|| {
            tessellate::tessellate_path(cmd)
        });

        match res {
            Ok(Ok(buffers)) => {
                tessellate::write_output(buffers, tess_matches.is_present("COUNT"), output).unwrap();
            }
            _ => {
                println!(" -- Error while tessellating");
                let path = build_path(Path::builder().flattened(tolerance).with_svg(), &input_buffer).unwrap();
                if tess_matches.is_present("DEBUG") {
                    println!(" -- Looking for a minimal test case...");
                    find_reduced_test_case(
                        path.as_slice(),
                        &|path: Path| {
                            tessellate::tessellate_path(TessellateCmd {
                                path: path,
                                fill: fill,
                                stroke: stroke_cmd,
                                tolerance: tolerance,
                            }).is_err()
                        },
                    );
                }
                panic!("aborting");
            }
        }

    } else if let Some(flatten_matches) = matches.subcommand_matches("flatten") {
        let cmd = FlattenCmd {
            input: input_buffer,
            output: output,
            tolerance: get_tolerance(&flatten_matches),
            count: flatten_matches.is_present("COUNT"),
        };

        flatten::flatten(cmd).unwrap();
    } else if let Some(fuzz_matches) = matches.subcommand_matches("fuzz") {
        fuzzing::run(FuzzCmd {
            fill: fuzz_matches.is_present("FILL"),
            stroke: fuzz_matches.is_present("STROKE"),
            min_points: fuzz_matches.value_of("MIN_POINTS").and_then(|str_val| str_val.parse::<u32>().ok()),
            max_points: fuzz_matches.value_of("MAX_POINTS").and_then(|str_val| str_val.parse::<u32>().ok()),
        });
    }
}

fn get_tolerance(matches: &ArgMatches) -> f32 {
    let default = 0.5;
    if let Some(tolerance_str) = matches.value_of("TOLERANCE") {
        return tolerance_str.parse().unwrap_or(default);
    }
    return default;
}

fn get_stroke(matches: &ArgMatches) -> Option<f32> {
    if let Some(stroke_str) = matches.value_of("STROKE") {
        if let Ok(val) = stroke_str.parse() {
            return Some(val);
        }
    }
    return None;
}