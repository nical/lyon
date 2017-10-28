extern crate clap;
extern crate lyon;
extern crate lyon_extra;
extern crate rand;
#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;

mod commands;
mod tessellate;
mod fuzzing;
mod flatten;
mod show;

use clap::*;
use commands::*;

use std::fs::File;
use std::io::{Read, Write, stdout, stderr};
use lyon::svg::parser::build_path;
use lyon::path::Path;
use lyon::tessellation::{FillOptions, StrokeOptions, LineJoin, LineCap};
use lyon_extra::debugging::find_reduced_test_case;

fn main() {
    let matches = App::new("Lyon command-line interface")
        .version("0.1")
        .author("Nicolas Silva <nical@fastmail.com>")
        .about("Path tessellator")
        .subcommand(
            declare_tess_params(SubCommand::with_name("tessellate"))
            .about("Tessellates a path")
            .arg(Arg::with_name("DEBUG")
                .short("d")
                .long("debug")
                .help("Enable debugging")
            )
            .arg(Arg::with_name("COUNT")
                .short("c")
                .long("count")
                .help("Prints the number of triangles and vertices")
            )
            .arg(Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .help("Sets the output file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
            )
        )
        .subcommand(
            declare_input_path(SubCommand::with_name("path"))
            .about("Transforms an SVG path")
            .arg(Arg::with_name("TOLERANCE")
                .short("t")
                .long("tolerance")
                .help("Sets the tolerance threshold (0.5 by default)")
                .value_name("TOLERANCE")
                .takes_value(true)
            )
            .arg(Arg::with_name("FLATTEN")
                .short("f")
                .long("flatten")
                .help("Approximates all curves with line segments")
            )
            .arg(Arg::with_name("COUNT")
                .short("c")
                .long("count")
                .help("Prints the number of vertices")
            )
            .arg(Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .help("Sets the output file to use")
                .value_name("FILE")
                .takes_value(true)
                .required(false)
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
        .subcommand(
            declare_tess_params(SubCommand::with_name("show"))
            .about("Renders a path in an interactive window")
        )
        .get_matches();

    if let Some(command) = matches.subcommand_matches("tessellate") {
        let output = get_output(&command);
        let cmd = get_tess_command(&command);
        let cmd_copy = cmd.clone();

        let res = ::std::panic::catch_unwind(|| {
            tessellate::tessellate_path(cmd)
        });

        match res {
            Ok(Ok(buffers)) => {
                tessellate::write_output(buffers, command.is_present("COUNT"), output).unwrap();
            }
            _ => {
                println!(" -- Error while tessellating");
                if command.is_present("DEBUG") {
                    println!(" -- Looking for a minimal test case...");
                    find_reduced_test_case(
                        cmd_copy.path.as_slice(),
                        &|path: Path| {
                            let cmd = TessellateCmd {
                                path,
                                ..cmd_copy.clone()
                            };
                            tessellate::tessellate_path(cmd).is_err()
                        },
                    );
                }
                panic!("aborting");
            }
        }

    }

    if let Some(command) = matches.subcommand_matches("path") {
        let cmd = PathCmd {
            path: get_path(&command).expect("Need a path to transform"),
            output: get_output(command),
            tolerance: get_tolerance(&command),
            count: command.is_present("COUNT"),
            flatten: command.is_present("FLATTEN"),
        };

        flatten::flatten(cmd).unwrap();
    }

    if let Some(fuzz_matches) = matches.subcommand_matches("fuzz") {
        fuzzing::run(FuzzCmd {
            fill: fuzz_matches.is_present("FILL"),
            stroke: fuzz_matches.is_present("STROKE"),
            min_points: fuzz_matches.value_of("MIN_POINTS").and_then(|str_val| str_val.parse::<u32>().ok()),
            max_points: fuzz_matches.value_of("MAX_POINTS").and_then(|str_val| str_val.parse::<u32>().ok()),
        });
    }

    if let Some(command) = matches.subcommand_matches("show") {
        let cmd = get_tess_command(command);
        show::show_path(cmd);
    }
}

fn declare_input_path<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.arg(Arg::with_name("PATH")
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
}

fn declare_tess_params<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    declare_input_path(app)
    .arg(Arg::with_name("FILL")
        .short("f")
        .long("fill")
        .help("Fills the path")
    )
    .arg(Arg::with_name("STROKE")
        .short("s")
        .long("stroke")
        .help("Strokes the path")
    )
    .arg(Arg::with_name("TOLERANCE")
        .short("t")
        .long("tolerance")
        .help("Sets the tolerance threshold for flattening (0.5 by default)")
        .value_name("TOLERANCE")
        .takes_value(true)
    )
    .arg(Arg::with_name("LINE_WIDTH")
        .short("w")
        .long("line-width")
        .help("The line width for strokes")
        .value_name("LINE_WIDTH")
        .takes_value(true)
    )
    .arg(Arg::with_name("LINE_JOIN")
        .long("line-join")
        .help("The line-join type for strokes")
        .value_name("LINE_JOIN")
        .takes_value(true)
    )
    .arg(Arg::with_name("LINE_CAP")
        .long("line-cap")
        .help("The line-cap type for strokes")
        .value_name("LINE_CAP")
        .takes_value(true)
    )
    .arg(Arg::with_name("MITER_LIMIT")
        .long("miter-limit")
        .help("The miter limit for strokes")
        .value_name("MITER_LIMIT")
        .takes_value(true)
    )
}

fn get_path(matches: &ArgMatches) -> Option<Path> {
    let mut path_str = matches.value_of("PATH").unwrap_or(&"").to_string();

    if let Some(input_file) = matches.value_of("INPUT") {
        if let Ok(mut file) = File::open(input_file) {
            file.read_to_string(&mut path_str).unwrap();
        } else {
            write!(&mut stderr(), "Cannot open file {}", input_file).unwrap();
            return None;
        }
    }

    if path_str.is_empty() {
        return None;
    }


    return match build_path(Path::builder().with_svg(), &path_str) {
        Ok(path) => { Some(path) }
        Err(e) => {
            println!("Error while parsing path: {}", path_str);
            println!("{}", e);
            None
        }
    }
}

fn get_tess_command(command: &ArgMatches) -> TessellateCmd {
    let path = get_path(command).expect("Need a path to tessellate");
    let stroke_cmd = get_stroke(command);
    let fill = if command.is_present("FILL") || !stroke_cmd.is_some() {
        Some(FillOptions::tolerance(get_tolerance(&command)))
    } else {
        None
    };

    TessellateCmd {
        path: path.clone(),
        fill: fill,
        stroke: stroke_cmd,
    }
}

fn get_tolerance(matches: &ArgMatches) -> f32 {
    let default = 0.2;
    if let Some(tolerance_str) = matches.value_of("TOLERANCE") {
        return tolerance_str.parse().unwrap_or(default);
    }
    return default;
}

fn get_stroke(matches: &ArgMatches) -> Option<StrokeOptions> {
    if matches.is_present("STROKE") {
        let mut options = StrokeOptions::default();
        let cap = get_line_cap(matches);
        options.start_cap = cap;
        options.end_cap = cap;
        options.line_width = get_line_width(matches);
        options.line_join = get_line_join(matches);
        options.tolerance = get_tolerance(matches);
        if let Some(limit) = get_miter_limit(matches) {
            options.miter_limit = limit;
        }
        return Some(options);
    }
    return None;
}

fn get_line_join(matches: &ArgMatches) -> LineJoin {
    if let Some(stroke_str) = matches.value_of("LINE_JOIN") {
        return match stroke_str {
            "Miter" => LineJoin::Miter,
            "MiterClip" => LineJoin::MiterClip,
            "Round" => LineJoin::Round,
            "Bevel" => LineJoin::Bevel,
            _ => LineJoin::Miter,
        }
    }
    return LineJoin::Miter;
}

fn get_line_cap(matches: &ArgMatches) -> LineCap {
    if let Some(stroke_str) = matches.value_of("LINE_CAP") {
        return match stroke_str {
            "Butt" => LineCap::Butt,
            "Square" => LineCap::Square,
            "Round" => LineCap::Round,
            _ => LineCap::Butt,
        }
    }
    return LineCap::Butt;
}

fn get_miter_limit(matches: &ArgMatches) -> Option<f32> {
    if let Some(stroke_str) = matches.value_of("MITER_LIMIT") {
        if let Ok(val) = stroke_str.parse() {
            return Some(val);
        }
    }
    return None;
}

fn get_line_width(matches: &ArgMatches) -> f32 {
    if let Some(stroke_str) = matches.value_of("LINE_WIDTH") {
        if let Ok(val) = stroke_str.parse() {
            return val;
        }
    }
    return 1.0;
}

fn get_output(matches: &ArgMatches) -> Box<Write> {
    let mut output: Box<Write> = Box::new(stdout());
    if let Some(output_file) = matches.value_of("OUTPUT") {
        if let Ok(file) = File::create(output_file) {
            output = Box::new(file);
        }
    }
    return output;
}
