
use svgparser::{ Tokenize, TextFrame };
use svgparser::path::{ Tokenizer, Token };
use core::SvgEvent;
use core::math::*;
use core::ArcFlags;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParserError;

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
            Err(_) => { Some(Err(ParserError)) }
        }
    }
}

fn svg_event(token: &Token) -> SvgEvent {
    fn v(x: f64, y: f64) -> Point { point(x as f32, y as f32) }
    match *token {
        Token::MoveTo { abs, x, y } => {
            if abs {
                SvgEvent::MoveTo(v(x, y))
            } else {
                SvgEvent::RelativeMoveTo(v(x, y))
            }
        },
        Token::LineTo { abs, x, y } => {
            if abs {
                SvgEvent::LineTo(v(x, y))
            } else {
                SvgEvent::RelativeLineTo(v(x, y))
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
                SvgEvent::CubicTo(v(x1, y1), v(x2, y2), v(x, y))
            } else {
                SvgEvent::RelativeCubicTo(v(x1, y1), v(x2, y2), v(x, y))
            }
        },
        Token::SmoothCurveTo { abs, x2, y2, x, y } => {
            if abs {
                SvgEvent::SmoothCubicTo(v(x2, y2), v(x, y))
            } else {
                SvgEvent::SmoothRelativeCubicTo(v(x2, y2), v(x, y))
            }
        },
        Token::Quadratic { abs, x1, y1, x, y } => {
            if abs {
                SvgEvent::QuadraticTo(v(x1, y1), v(x, y))
            } else {
                SvgEvent::RelativeQuadraticTo(v(x1, y1), v(x, y))
            }
        },
        Token::SmoothQuadratic { abs, x, y } => {
            if abs {
                SvgEvent::SmoothQuadraticTo(v(x, y))
            } else {
                SvgEvent::SmoothRelativeQuadraticTo(v(x, y))
            }
        },
        Token::EllipticalArc { abs, rx, ry, x_axis_rotation, large_arc, sweep, x, y } => {
            if abs {
                SvgEvent::ArcTo(
                v(x, y), v(rx, ry), Radians::new(x_axis_rotation.to_radians() as f32),
                ArcFlags { large_arc: large_arc, sweep: sweep }
                )
            } else {
                SvgEvent::RelativeArcTo(
                v(x, y), v(rx, ry), Radians::new(x_axis_rotation.to_radians() as f32),
                ArcFlags { large_arc: large_arc, sweep: sweep }
                )
            }
        },
        Token::ClosePath { .. } => { SvgEvent::Close },
        Token::EndOfStream => unreachable!(),
    }
}
