# Lyon command-line tool

A simple program that exposes lyon's tessellators to the terminal.

```
USAGE:
    lyon [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    fuzz          tessellates random paths in order to find potential bugs
    help          Prints this message or the help of the given subcommand(s)
    path          Transforms an SVG path
    show          Renders a path in an interactive window
    tessellate    Tessellates a path
```

# Commands

## ```tessellate```

Tessellates an SVG path. The output is a vertex buffer and an index buffer in text representation.

Run ```$> lyon tessellate --help``` for more details.

### examples

```
$> lyon tessellate "M 0 0 L 10 0  10 10 L 0 10 z"
```

This command prints to stdout:

```
vertices: [(0, 0), (10, 0), (0, 10), (10, 10)]
indices: [0, 1, 2, 2, 1, 3]
```

To read and write from files instead, see the ```-i <FILE>``` and ```-o <FILE>```.

```
$> echo "M 0 0 L 10 0  10 10 L 0 10 z" > input.path
$> lyon tessellate -i input.path -o output.txt
$> cat output.txt
vertices: [(0, 0), (10, 0), (0, 10), (10, 10)]
indices: [0, 1, 2, 2, 1, 3]
```

The flag ```-c``` prints some stats instead of the actual tessellation:

```
$> lyon tessellate "M 0 0 L 10 0  10 10 L 0 10 z" -c
vertices: 4
indices: 6
triangles: 2
```

## ```show```

Opens a window with an interactive path viewer.

Run ```$> lyon show --help``` for more details.

## ```fuzz```

This command runs the built-in fuzzer. The fuzzer will generate random paths and tessellate them until it finds a path that trigers an error. Once an error is found, the fuzzer tries to reduce the test case and prints the reduced test case to stdout.

Run ```$> lyon fuzz --help``` for more details.

### example

```
$> lyon fuzz --max-points 12
```

## ```path```

Performs some given transformations on an SVG path.
At the moment the only transformation implemented is flattening (approximating curve segments with successions of line segments).

Run ```$> lyon path --help``` for more details.
