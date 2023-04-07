use itertools::{Itertools, Tuples};
use lyon::math::Point;
use lyon::tessellation::geometry_builder::VertexBuffers;
use regex::Regex;

const DEFAULT_FMT: &str = r"vertices: [@vertices{sep=, }{fmt=({position.x}, {position.y})}@]\nindices: [@indices{sep=, }{fmt={index}}@]";

pub fn format_output(
    fmt_string: Option<&str>,
    precision: Option<usize>,
    buffers: &VertexBuffers<Point, u16>,
) -> String {
    let fmt = fmt_string.unwrap_or(DEFAULT_FMT).split('@');
    let extract = Regex::new(r"^(.*)\{sep=(.+?)\}\{fmt=(.*)\}$").unwrap();

    let mut output = String::with_capacity(buffers.vertices.len() + buffers.indices.len());
    for section in fmt {
        if let Some(capture) = extract.captures(section) {
            let iter_name = capture.get(1).map(|m| m.as_str()).unwrap();
            let sep = capture.get(2).map(|m| m.as_str()).unwrap();
            let pattern = capture.get(3).map(|m| m.as_str()).unwrap();

            match iter_name {
                "vertices" => {
                    output.push_str(&format_iter(buffers.vertices.iter(), sep, pattern, |x| {
                        format_float(x, precision)
                    }));
                }
                "indices" => {
                    output.push_str(&format_iter(buffers.indices.iter(), sep, pattern, |x| {
                        format!("{x}")
                    }));
                }
                "triangles" => {
                    let triangles: Tuples<_, (_, _, _)> = buffers.indices.iter().tuples();
                    output.push_str(&format_iter(triangles, sep, pattern, |x| format!("{x}")));
                }
                invalid => {
                    eprintln!("ERROR: `@{invalid}...@` does not name an expansion");
                    std::process::exit(1);
                }
            }
        } else {
            output.push_str(section);
        }
    }
    escape_specials(&output)
}

fn format_iter<M, F, I>(iter: I, sep: &str, pattern: &str, value_fmt: F) -> String
where
    M: MatchVariable,
    F: Fn(M::Value) -> String,
    I: Iterator<Item = M>,
{
    let mut fmt_items: Vec<String> = Vec::new();
    let extract = Regex::new(r"(\\\{.*?)*(\{.*?\})(\\\})*").unwrap();

    for item in iter {
        let mut buf = String::from(pattern);
        for var in extract.captures_iter(pattern) {
            let var = &var[2];
            if let Some(val) = item.match_var(var) {
                let value = value_fmt(val);
                let value = &value[..];
                let replace = Regex::new(&regex_escape_brackets(var)).unwrap();
                buf = replace.replace_all(&buf, value).to_string();
            } else {
                eprintln!("ERROR: `{var}` does not name a variable");
                std::process::exit(1)
            }
        }
        fmt_items.push(buf)
    }
    fmt_items.iter().join(sep)
}

fn format_float(value: f32, precision: Option<usize>) -> String {
    if let Some(p) = precision {
        format!("{value:.p$}")
    } else {
        format!("{value}")
    }
}

fn regex_escape_brackets(s: &str) -> String {
    let mut buf = String::new();
    for c in s.chars() {
        match c {
            '{' | '}' => {
                buf.push('\\');
            }
            _ => {}
        };
        buf.push(c)
    }
    buf
}

fn escape_specials(s: &str) -> String {
    s.chars()
        .coalesce(|prev, cur| {
            if prev == '\\' {
                match cur {
                    'n' => Ok('\n'),
                    't' => Ok('\t'),
                    '{' => Ok('{'),
                    '}' => Ok('}'),
                    _ => Err((prev, cur)),
                }
            } else {
                Err((prev, cur))
            }
        })
        .collect::<String>()
}

trait MatchVariable {
    type Value;
    fn match_var(&self, key: &str) -> Option<Self::Value>;
}

impl<'a> MatchVariable for &'a Point {
    type Value = f32;

    fn match_var(&self, key: &str) -> Option<Self::Value> {
        match key {
            "{position.x}" | "{pos.x}" => Some(self.x),
            "{position.y}" | "{pos.y}" => Some(self.y),
            _ => None,
        }
    }
}

impl<'a> MatchVariable for &'a u16 {
    type Value = u16;

    fn match_var(&self, key: &str) -> Option<Self::Value> {
        match key {
            "{index}" | "{i}" => Some(**self),
            _ => None,
        }
    }
}

impl<'a> MatchVariable for (&'a u16, &'a u16, &'a u16) {
    type Value = u16;

    fn match_var(&self, key: &str) -> Option<Self::Value> {
        match key {
            "{index0}" | "{i0}" => Some(*self.0),
            "{index1}" | "{i1}" => Some(*self.1),
            "{index2}" | "{i2}" => Some(*self.2),
            _ => None,
        }
    }
}
