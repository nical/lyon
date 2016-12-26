extern crate clap;
extern crate lyon;

mod commands;
mod tessellate;
mod flatten;

use clap::*;
use commands::*;

use std::fs::File;
use std::io::{Write, stdout, stderr};
use std::io::prelude::*;

fn main() {
    let matches = App::new("Lyon command-line interface")
        .version("0.1")
        .author("Nicolas Silva <nical@fastmail.com>")
        .about("Path tessellator")
        .subcommand(SubCommand::with_name("tessellate")
            .about("Tessellates a path")
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
        .arg(Arg::with_name("PATH")
            .value_name("PATH")
            .help("An SVG path")
            .takes_value(true)
            .required(false)
        )
        .arg(Arg::with_name("INPUT")
            .help("Sets the input file to use")
            .short("i")
            .long("input")
            .value_name("FILE")
            .takes_value(true)
            .required(false)
        )
        .arg(Arg::with_name("OUTPUT")
            .help("Sets the output file to use")
            .value_name("FILE")
            .short("o")
            .long("output")
            .takes_value(true)
            .required(false)
        )
        .get_matches();

    let mut input_buffer = matches.value_of("PATH").unwrap_or(&"").to_string();

    String::new();

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
        let stroke_cmd = tess_matches.is_present("STROKE");
        let cmd = TessellateCmd {
            input: input_buffer,
            output: output,
            fill: fill_cmd || (!fill_cmd && !stroke_cmd),
            stroke: stroke_cmd,
            tolerance: get_tolerance(&tess_matches),
            count: tess_matches.is_present("COUNT"),
        };

        tessellate::tessellate(cmd).unwrap();

    } else if let Some(flatten_matches) = matches.subcommand_matches("flatten") {
        let cmd = FlattenCmd {
            input: input_buffer,
            output: output,
            tolerance: get_tolerance(&flatten_matches),
            count: flatten_matches.is_present("COUNT"),
        };

        flatten::flatten(cmd).unwrap();
    }
}

fn get_tolerance(matches: &ArgMatches) -> f32 {
    let default = 0.5;
    if let Some(tolerance_str) = matches.value_of("TOLERANCE") {
        return tolerance_str.parse().unwrap_or(default);
    }
    return default;
}
