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

### example

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

To specify the output format use ```--format <FORMAT_STRING>```.

There are 3 format markers, one for *vertices*, *indices* and for *triangles* (triplets of indices). Each format marker starts with ```@```, has a separator block (```{sep=...}```), a format block (```{fmt=...}``` and it ends with ```@```. In the blocks ```{``` and ```}``` characters have to be escaped!


#### list of format variables
- ```@vertices```
	* ```{position.x}``` or ```{pos.x}```
	* ```{position.y}``` or ```{pos.y}```
- ```@indices```
	* ```{index}``` or ```{i}```
- ```@triangles```
	* ```{index0}``` or ```{i0}```
	* ```{index1}``` or ```{i1}```
	* ```{index2}``` or ```{i2}```

#### examples

```
$> lyon tessellate --format "vertices: [@vertices{sep=, }{fmt=({position.x}, {position.y})}@]" "M 0 0 L 10 0  10 10 L 0 10 z"
vertices: [(0, 0), (10, 0), (0, 10), (10, 10)]

$> lyon tessellate --format "@indices{sep=, }{fmt=[{index}]}@" "M 0 0 L 10 0  10 10 L 0 10 z"
[0], [1], [2], [2], [1], [3]

$> lyon tessellate --format '\{\n "triangles": \{\n@triangles{sep=,\n}{fmt=  "triangle": \{\n   "{i0}",\n   "{i1}",\n   "{i2}"\n  \}}@\n \}\n\}' "M 0 0 L 10 0  10 10 L 0 10 z"
{
 "triangles": {
  "triangle": {
   "1",
   "0",
   "2"
  },
  "triangle": {
   "1",
   "2",
   "3"
  }
 }
}
```

## ```show```

Opens a window with an interactive path viewer.

Run ```$> lyon show --help``` for more details.

### example

```
$> lyon show -i assets/logo.path --fill --stroke --tolerance 0.05 --line-join Round --line-width 1.5
```

## ```fuzz```

This command runs the built-in fuzzer. The fuzzer will generate random paths and tessellate them until it finds a path that triggers an error. Once an error is found, the fuzzer tries to reduce the test case and prints the reduced test case to stdout.

Run ```$> lyon fuzz --help``` for more details.

### example

```
$> lyon fuzz --max-points 12
```

## ```path```

Performs some given transformations on an SVG path.
At the moment the only transformation implemented is flattening (approximating curve segments with successions of line segments).

Run ```$> lyon path --help``` for more details.
