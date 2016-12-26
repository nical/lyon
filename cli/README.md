# Lyon command-line tool

A simple program that exposes lyon's tessellators to the terminal.

```
USAGE:
    lyon [OPTIONS] [<PATH>] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i, --input <FILE>     Sets the input file to use
    -o, --output <FILE>    Sets the output file to use

ARGS:
    <PATH>    A path

SUBCOMMANDS:
    flatten       Flatten a path
    help          Prints this message or the help of the given subcommand(s)
    tessellate    Tessellate a path
```

## The ```tessellate``` subcommand

Tessellates an SVG path. The output is a vertex buffer and an index buffer in text representation.
Run ```$> lyon tessellate --help``` for more details.

## The ```flatten``` subcommand

Flattens an SVG path, turning curve segments into successions of line segments for a given threshold. The output is an SVG path.
Run ```$> lyon flatten --help``` for more details.

# Examples


```
$> lyon "M 0 0 L 10 0  10 10 L 0 10 z" tessellate
```

This command prints to stdout:

```
vertices: [(0, 0), (10, 0), (0, 10), (10, 10)]
indices: [0, 1, 2, 2, 1, 3]
```

To read and write from files instead, see the ```-i <FILE>``` and ```-o <FILE>```.

```
$> echo "M 0 0 L 10 0  10 10 L 0 10 z" > input.path
$> lyon -i input.path -o output.txt tessellate
$> cat output.txt
vertices: [(0, 0), (10, 0), (0, 10), (10, 10)]
indices: [0, 1, 2, 2, 1, 3]
```

The flag ```-c``` prints some stats instead of the actual tessellation:

```
$> lyon "M 0 0 L 10 0  10 10 L 0 10 z" tessellate -c
vertices: 4
indices: 6
triangles: 2
```
