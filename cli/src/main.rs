extern crate clap;
extern crate itertools;
extern crate lyon;
extern crate rand;
extern crate regex;

mod commands;
mod flatten;
mod fuzzing;
mod reduce;
mod show;
mod tessellate;

use clap::*;
use commands::*;

use lyon::algorithms::hatching::{DotOptions, HatchingOptions};
use lyon::extra::debugging::find_reduced_test_case;
use lyon::extra::parser::*;
use lyon::geom::euclid::Angle;
use lyon::path::Path;
use lyon::tessellation::{FillOptions, FillRule, LineCap, LineJoin, Orientation, StrokeOptions};
use std::fs::File;
use std::io::{stdout, Read, Write};

fn main() {
    env_logger::init();

    let matches = App::new("Lyon command-line interface")
        .version("0.1")
        .author("Nicolas Silva <nical@fastmail.com>")
        .about("Path tessellator")
        .subcommand(
            declare_tess_params(SubCommand::with_name("tessellate"), true)
                .about("Tessellates a path")
                .arg(
                    Arg::with_name("DEBUG")
                        .short('d')
                        .long("debug")
                        .help("Enable debugging"),
                )
                .arg(
                    Arg::with_name("COUNT")
                        .short('c')
                        .long("count")
                        .help("Prints the number of triangles and vertices"),
                )
                .arg(
                    Arg::with_name("OUTPUT")
                        .short('o')
                        .long("output")
                        .help("Sets the output file to use")
                        .value_name("FILE")
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::with_name("FLOAT_PRECISION")
                        .long("float-precision")
                        .help("Sets the floating point precision for the output")
                        .value_name("FLOAT_PRECISION")
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::with_name("FORMAT")
                        .long("format")
                        .help("Prints the output with the specified format")
                        .value_name("FORMAT_STRING")
                        .takes_value(true)
                        .required(false),
                ),
        )
        .subcommand(
            declare_input_path(SubCommand::with_name("path"))
                .about("Transforms an SVG path")
                .arg(
                    Arg::with_name("TOLERANCE")
                        .short('t')
                        .long("tolerance")
                        .help("Sets the tolerance threshold (0.5 by default)")
                        .value_name("TOLERANCE")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("FLATTEN")
                        .short('f')
                        .long("flatten")
                        .help("Approximates all curves with line segments"),
                )
                .arg(
                    Arg::with_name("COUNT")
                        .short('c')
                        .long("count")
                        .help("Prints the number of vertices"),
                )
                .arg(
                    Arg::with_name("OUTPUT")
                        .short('o')
                        .long("output")
                        .help("Sets the output file to use")
                        .value_name("FILE")
                        .takes_value(true)
                        .required(false),
                ),
        )
        .subcommand(
            declare_tess_params(SubCommand::with_name("fuzz"), false)
                .about("tessellates random paths in order to find potential bugs")
                .arg(
                    Arg::with_name("MAX_POINTS")
                        .long("max-points")
                        .help("Sets the maximum number of points per paths")
                        .value_name("MAX_POINTS")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("MIN_POINTS")
                        .long("min-points")
                        .help("Sets the minimum number of points per paths")
                        .value_name("MIN_POINTS")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("IGNORE_ERRORS")
                        .long("ignore-errors")
                        .help("Try to continue when encountering errors unless it is a panic."),
                ),
        )
        .subcommand(
            declare_tess_params(SubCommand::with_name("show"), true)
                .about("Renders a path in an interactive window")
                .arg(
                    Arg::with_name("ANTIALIASING")
                        .long("anti-aliasing")
                        .help("Sets the anti-aliasing method to use (msaa or none)")
                        .value_name("ANTIALIASING")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("BACKGROUND")
                        .long("background")
                        .value_name("Blue|Dark|Clear")
                        .help("Change the color of the window's background")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("DEBUGGER_2D")
                        .long("enable-debugger-2d")
                        .value_name("filter")
                        .help(
                            "Install a debugger 2d on the fill tessellator and display its output",
                        )
                        .takes_value(true),
                ),
        )
        .subcommand(
            declare_tess_params(SubCommand::with_name("reduce"), true)
                .about("Find a reduced testcase"),
        )
        .get_matches();

    if let Some(command) = matches.subcommand_matches("tessellate") {
        let output = get_output(command);
        let cmd = get_tess_command(command, true);
        let cmd_copy = cmd.clone();
        let float_precision = cmd.float_precision;

        let res = ::std::panic::catch_unwind(|| tessellate::tessellate_path(cmd));

        match res {
            Ok(Ok(buffers)) => {
                tessellate::write_output(
                    buffers,
                    command.is_present("COUNT"),
                    command.value_of("FORMAT"),
                    float_precision,
                    output,
                )
                .unwrap();
            }
            _ => {
                println!(" -- Error while tessellating");
                if command.is_present("DEBUG") {
                    println!(" -- Looking for a minimal test case...");
                    find_reduced_test_case(cmd_copy.path.as_slice(), &|path: Path| {
                        let cmd = TessellateCmd {
                            path,
                            ..cmd_copy.clone()
                        };
                        tessellate::tessellate_path(cmd).is_err()
                    });
                }
                panic!("aborting");
            }
        }
    }

    if let Some(command) = matches.subcommand_matches("path") {
        let cmd = PathCmd {
            path: get_path(command).expect("Need a path to transform"),
            output: get_output(command),
            tolerance: get_tolerance(command),
            count: command.is_present("COUNT"),
            flatten: command.is_present("FLATTEN"),
        };

        flatten::flatten(cmd).unwrap();
    }

    if let Some(fuzz_matches) = matches.subcommand_matches("fuzz") {
        fuzzing::run(FuzzCmd {
            tess: get_tess_command(fuzz_matches, false),
            min_points: fuzz_matches
                .value_of("MIN_POINTS")
                .and_then(|str_val| str_val.parse::<u32>().ok()),
            max_points: fuzz_matches
                .value_of("MAX_POINTS")
                .and_then(|str_val| str_val.parse::<u32>().ok()),
            ignore_errors: fuzz_matches.is_present("IGNORE_ERRORS"),
        });
    }

    if let Some(command) = matches.subcommand_matches("show") {
        let cmd = get_tess_command(command, true);
        let render_params = get_render_params(command);
        show::show_path(cmd, render_params);
    }

    if let Some(command) = matches.subcommand_matches("reduce") {
        let cmd = get_tess_command(command, true);
        reduce::reduce_testcase(cmd);
    }
}

fn declare_input_path(app: App) -> App {
    app.arg(
        Arg::with_name("PATH")
            .value_name("PATH")
            .help("An SVG path")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::with_name("INPUT")
            .short('i')
            .long("input")
            .help("Sets the input file to use")
            .value_name("FILE")
            .takes_value(true)
            .required(false),
    )
}

fn declare_tess_params(app: App, need_path: bool) -> App {
    if need_path {
        declare_input_path(app)
    } else {
        app
    }
    .arg(
        Arg::with_name("FILL")
            .short('f')
            .long("fill")
            .help("Fills the path"),
    )
    .arg(
        Arg::with_name("STROKE")
            .short('s')
            .long("stroke")
            .help("Strokes the path"),
    )
    .arg(
        Arg::with_name("HATCH")
            .long("hatch")
            .help(
                "Fill the path with a regular hatching pattern using the provided value as spacing",
            )
            .takes_value(true),
    )
    .arg(
        Arg::with_name("DOT")
            .long("dot")
            .help("Fill the path with a regular dot pattern using the provided value as spacing")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("TOLERANCE")
            .short('t')
            .long("tolerance")
            .help("Sets the tolerance threshold for flattening (0.5 by default)")
            .value_name("TOLERANCE")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("LINE_WIDTH")
            .short('w')
            .long("line-width")
            .help("The line width for strokes")
            .value_name("LINE_WIDTH")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("LINE_JOIN")
            .long("line-join")
            .help("The line-join type for strokes")
            .value_name("LINE_JOIN")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("LINE_CAP")
            .long("line-cap")
            .help("The line-cap type for strokes")
            .value_name("LINE_CAP")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("MITER_LIMIT")
            .long("miter-limit")
            .help("The miter limit for strokes")
            .value_name("MITER_LIMIT")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("FILL_RULE")
            .long("fill-rule")
            .help("Fill rule")
            .value_name("FILL_RULE")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("CUSTOM_ATTRIBUTES")
            .long("custom-attributes")
            .help("Custom attributes")
            .value_name("CUSTOM_ATTRIBUTES")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("VARIABLE_LINE_WIDTH")
            .long("variable-line-width")
            .help("Variable line width")
            .value_name("VARIABLE_LINE_WIDTH")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("SWEEP_ORIENTATION")
            .long("sweep-orientation")
            .help("Traverse the geometry vertically or horizontally.")
            .value_name("SWEEP_ORIENTATION")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("TESSELLATOR")
            .long("tessellator")
            .help("Select the tessellator to use")
            .value_name("TESSELLATOR")
            .takes_value(true),
    )
    .arg(
        Arg::with_name("HATCHING_ANGLE")
            .long("angle")
            .help("Angle between the hatching and the x axis")
            .takes_value(true),
    )
}

fn get_path(matches: &ArgMatches) -> Option<Path> {
    let mut path_str = matches.value_of("PATH").unwrap_or_default().to_string();

    if let Some(input_file) = matches.value_of("INPUT") {
        if let Ok(mut file) = File::open(input_file) {
            file.read_to_string(&mut path_str).unwrap();
        } else {
            eprintln!("Cannot open file {input_file}");
            return None;
        }
    }

    if path_str.is_empty() {
        return None;
    }

    let mut parser = PathParser::new();

    let mut options = ParserOptions::DEFAULT;
    if let Some(num_attributes_str) = matches.value_of("CUSTOM_ATTRIBUTES") {
        options.num_attributes = num_attributes_str.parse::<usize>().unwrap_or(0);
    }

    let mut builder = Path::builder_with_attributes(options.num_attributes);

    if let Err(e) = parser.parse(&options, &mut Source::new(path_str.chars()), &mut builder) {
        println!("Error while parsing path: {}", path_str);
        println!("{:?}", e);
    }

    Some(builder.build())
}

fn get_render_params(matches: &ArgMatches) -> RenderCmd {
    RenderCmd {
        aa: if let Some(aa) = matches.value_of("ANTIALIASING") {
            match aa {
                // wgpu currently only supports msaa 4 for portability reasons.
                "msaa" => AntiAliasing::Msaa(4),
                _ => AntiAliasing::None,
            }
        } else {
            AntiAliasing::Msaa(4)
        },
        background: get_background(matches),
        debugger: get_debugger(matches),
    }
}

fn get_tess_command(command: &ArgMatches, need_path: bool) -> TessellateCmd {
    let path = if need_path {
        get_path(command).expect("Need a path to tessellate")
    } else {
        Path::new()
    };

    let stroke = get_stroke(command);
    let hatch = get_hatching(command);
    let dots = get_dots(command);
    let fill_rule = get_fill_rule(command);
    let orientation = get_orientation(command);
    let fill =
        if command.is_present("FILL") || (stroke.is_none() && hatch.is_none() && dots.is_none()) {
            Some(
                FillOptions::tolerance(get_tolerance(command))
                    .with_fill_rule(fill_rule)
                    .with_sweep_orientation(orientation),
            )
        } else {
            None
        };

    let float_precision = if command.try_contains_id("FLOAT_PRECISION").is_ok() {
        command.value_of("FLOAT_PRECISION").map(|fp| {
            fp.parse::<usize>()
                .expect("Precision must be an integer")
                .min(7)
        })
    } else {
        None
    };

    TessellateCmd {
        path,
        fill,
        stroke,
        hatch,
        dots,
        float_precision,
    }
}

fn get_tolerance(matches: &ArgMatches) -> f32 {
    let default = 0.2;
    if let Some(tolerance_str) = matches.value_of("TOLERANCE") {
        tolerance_str.parse().unwrap_or(default)
    } else {
        default
    }
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
        if let Some(var_stroke_str) = matches.value_of("VARIABLE_LINE_WIDTH") {
            options.variable_line_width = var_stroke_str.parse::<usize>().ok()
        }

        Some(options)
    } else {
        None
    }
}

fn get_background(matches: &ArgMatches) -> Background {
    if let Some(name) = matches.value_of("BACKGROUND") {
        match name {
            "Blue" | "blue" => Background::Blue,
            "Dark" | "dark" => Background::Dark,
            "Clear" | "clear" => Background::Clear,
            _ => Background::Blue,
        }
    } else {
        Background::Blue
    }
}

fn get_debugger(matches: &ArgMatches) -> Option<u32> {
    if let Some(param) = matches.value_of("DEBUGGER_2D") {
        match param {
            "None" => Some(0),
            "all" => Some(0xfffff),
            other => other.parse().ok(),
        }
    } else {
        None
    }
}

fn get_hatching(matches: &ArgMatches) -> Option<HatchingParams> {
    if let Some(s) = matches.value_of("HATCH") {
        let spacing = match s.parse() {
            Ok(v) => v,
            Err(_) => {
                return None;
            }
        };

        let mut stroke = StrokeOptions::default();
        let cap = get_line_cap(matches);
        stroke.start_cap = cap;
        stroke.end_cap = cap;
        stroke.line_width = get_line_width(matches);
        stroke.line_join = get_line_join(matches);
        stroke.tolerance = get_tolerance(matches);

        let options = HatchingOptions::DEFAULT
            .with_tolerance(stroke.tolerance)
            .with_angle(get_hatching_angle(matches));

        Some(HatchingParams {
            options,
            stroke,
            spacing,
        })
    } else {
        None
    }
}

fn get_dots(matches: &ArgMatches) -> Option<DotParams> {
    if let Some(s) = matches.value_of("DOT") {
        let spacing = match s.parse() {
            Ok(v) => v,
            Err(_) => {
                return None;
            }
        };

        let mut stroke = StrokeOptions::default();
        let mut cap = get_line_cap(matches);
        if cap == LineCap::Butt {
            cap = LineCap::Square;
        }
        stroke.start_cap = cap;
        stroke.end_cap = cap;
        stroke.line_width = get_line_width(matches);
        stroke.tolerance = get_tolerance(matches);

        let options = DotOptions::DEFAULT
            .with_tolerance(stroke.tolerance)
            .with_angle(get_hatching_angle(matches));

        Some(DotParams {
            options,
            stroke,
            spacing,
        })
    } else {
        None
    }
}

fn get_line_join(matches: &ArgMatches) -> LineJoin {
    if let Some(stroke_str) = matches.value_of("LINE_JOIN") {
        match stroke_str {
            "Miter" => LineJoin::Miter,
            "MiterClip" => LineJoin::MiterClip,
            "Round" => LineJoin::Round,
            "Bevel" => LineJoin::Bevel,
            _ => LineJoin::Miter,
        }
    } else {
        LineJoin::Miter
    }
}

fn get_line_cap(matches: &ArgMatches) -> LineCap {
    if let Some(stroke_str) = matches.value_of("LINE_CAP") {
        match stroke_str {
            "Butt" => LineCap::Butt,
            "Square" => LineCap::Square,
            "Round" => LineCap::Round,
            _ => LineCap::Butt,
        }
    } else {
        LineCap::Butt
    }
}

fn get_fill_rule(matches: &ArgMatches) -> FillRule {
    if let Some(rule_str) = matches.value_of("FILL_RULE") {
        match rule_str {
            "NonZero" | "nonzero" => FillRule::NonZero,
            _ => FillRule::EvenOdd,
        }
    } else {
        FillRule::EvenOdd
    }
}

fn get_orientation(matches: &ArgMatches) -> Orientation {
    if let Some(orientation_str) = matches.value_of("SWEEP_ORIENTATION") {
        match orientation_str {
            "Horizontal" | "horizontal" | "h" => Orientation::Horizontal,
            _ => Orientation::Vertical,
        }
    } else {
        Orientation::Vertical
    }
}

fn get_miter_limit(matches: &ArgMatches) -> Option<f32> {
    if let Some(stroke_str) = matches.value_of("MITER_LIMIT") {
        if let Ok(val) = stroke_str.parse() {
            return Some(val);
        }
    }

    None
}

fn get_line_width(matches: &ArgMatches) -> f32 {
    if let Some(stroke_str) = matches.value_of("LINE_WIDTH") {
        if let Ok(val) = stroke_str.parse() {
            return val;
        }
    }

    1.0
}

fn get_hatching_angle(matches: &ArgMatches) -> Angle<f32> {
    if let Some(s) = matches.value_of("HATCHING_ANGLE") {
        if let Ok(val) = s.parse() {
            return Angle::radians(val);
        }
    }

    Angle::zero()
}

fn get_output(matches: &ArgMatches) -> Box<dyn Write> {
    if let Some(output_file) = matches.value_of("OUTPUT") {
        if let Ok(file) = File::create(output_file) {
            return Box::new(file);
        }
    }

    Box::new(stdout())
}
