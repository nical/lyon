
use svgparser::{ Tokenize, TextFrame };
use svgparser::path::{ Tokenizer, Token };
use path::{SvgEvent, ArcFlags};
use path::math::{Point, point, Vector, vector, Angle};
use path::builder::SvgBuilder;
use super::error::ParserError;

/// Builds path object using an SvgBuilder and a list of commands.
/// Once the path is built you can tessellate it.
///
/// The [SvgBuilder](trait.SvgBuilder.html) Adds to [PathBuilder](traits.PathBuilder.html)
/// the rest of the [SVG path](https://svgwg.org/specs/paths/) commands.
///
/// # Examples
///
/// ```
/// # extern crate lyon_svg as svg;
/// # extern crate lyon_path;
/// # use lyon_path::default::Path;
/// # use svg::parser::build_path;
/// # fn main() {
/// // Create a simple path.
/// let commands = &"M 0 0 L 10 0  10 10 L 0 10 z";
/// let svg_builder = Path::builder().with_svg();
/// let path = build_path(svg_builder, commands);
/// # }
/// ```
pub fn build_path<Builder>(mut builder: Builder, src: &str) -> Result<Builder::PathType, ParserError>
where
    Builder: SvgBuilder
{
    for item in PathTokenizer::new(src) {
        match item {
            Ok(event) => { builder.svg_event(event); }
            Err(err) => { return Err(err); }
        }
    }

    Ok(builder.build())
}


pub struct PathTokenizer<'l> {
    tokenizer: Tokenizer<'l>
}

impl<'l> PathTokenizer<'l> {
    pub fn new(text: &str) -> PathTokenizer {
        PathTokenizer {
            tokenizer: Tokenizer::from_str(text)
        }
    }

    pub fn from_frame(frame: TextFrame) -> PathTokenizer {
        PathTokenizer {
            tokenizer: Tokenizer::from_frame(frame)
        }
    }
}

impl<'l> Iterator for PathTokenizer<'l> {
    type Item = Result<SvgEvent, ParserError>;

    fn next(&mut self) -> Option<Result<SvgEvent, ParserError>> {
        match self.tokenizer.parse_next() {
            Ok(token) => {
                if token != Token::EndOfStream {
                    Some(Ok(svg_event(&token)))
                } else {
                    None
                }
            }
            Err(err) => { Some(Err(ParserError::PathToken(err))) }
        }
    }
}

fn svg_event(token: &Token) -> SvgEvent {
    fn vec2(x: f64, y: f64) -> Vector { vector(x as f32, y as f32) }
    fn point2(x: f64, y: f64) -> Point { point(x as f32, y as f32) }
    match *token {
        Token::MoveTo { abs, x, y } => {
            if abs {
                SvgEvent::MoveTo(point2(x, y))
            } else {
                SvgEvent::RelativeMoveTo(vec2(x, y))
            }
        },
        Token::LineTo { abs, x, y } => {
            if abs {
                SvgEvent::LineTo(point2(x, y))
            } else {
                SvgEvent::RelativeLineTo(vec2(x, y))
            }
        },
        Token::HorizontalLineTo { abs, x } => {
            if abs {
                SvgEvent::HorizontalLineTo(x as f32)
            } else {
                SvgEvent::RelativeHorizontalLineTo(x as f32)
            }
        },
        Token::VerticalLineTo { abs, y } => {
            if abs {
                SvgEvent::VerticalLineTo(y as f32)
            } else {
                SvgEvent::RelativeVerticalLineTo(y as f32)
            }
        },
        Token::CurveTo { abs, x1, y1, x2, y2, x, y } => {
            if abs {
                SvgEvent::CubicTo(point2(x1, y1), point2(x2, y2), point2(x, y))
            } else {
                SvgEvent::RelativeCubicTo(vec2(x1, y1), vec2(x2, y2), vec2(x, y))
            }
        },
        Token::SmoothCurveTo { abs, x2, y2, x, y } => {
            if abs {
                SvgEvent::SmoothCubicTo(point2(x2, y2), point2(x, y))
            } else {
                SvgEvent::SmoothRelativeCubicTo(vec2(x2, y2), vec2(x, y))
            }
        },
        Token::Quadratic { abs, x1, y1, x, y } => {
            if abs {
                SvgEvent::QuadraticTo(point2(x1, y1), point2(x, y))
            } else {
                SvgEvent::RelativeQuadraticTo(vec2(x1, y1), vec2(x, y))
            }
        },
        Token::SmoothQuadratic { abs, x, y } => {
            if abs {
                SvgEvent::SmoothQuadraticTo(point2(x, y))
            } else {
                SvgEvent::SmoothRelativeQuadraticTo(vec2(x, y))
            }
        },
        Token::EllipticalArc { abs, rx, ry, x_axis_rotation, large_arc, sweep, x, y } => {
            if abs {
                SvgEvent::ArcTo(
                    vec2(rx, ry),
                    Angle::degrees(x_axis_rotation as f32),
                    ArcFlags { large_arc: large_arc, sweep: sweep },
                    point2(x, y),
                )
            } else {
                SvgEvent::RelativeArcTo(
                    vec2(rx, ry),
                    Angle::degrees(x_axis_rotation as f32),
                    ArcFlags { large_arc: large_arc, sweep: sweep },
                    vec2(x, y),
                )
            }
        },
        Token::ClosePath { .. } => { SvgEvent::Close },
        Token::EndOfStream => unreachable!(),
    }
}
