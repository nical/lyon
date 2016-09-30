use svgparser::path::{ Segment, SegmentData, Tokenizer };
use core::SvgEvent;
use core::math::*;
use core::ArcFlags;

use super::Stream;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParserError;

pub struct PathTokenizer<'l> {
    tokenizer: Tokenizer<'l>
}

impl<'l> PathTokenizer<'l> {
    pub fn new(text: &str) -> PathTokenizer {
        PathTokenizer::from_stream(Stream::new(text.as_bytes()))
    }

    pub fn from_stream(stream: Stream) -> PathTokenizer {
        PathTokenizer {
            tokenizer: Tokenizer::new(stream)
        }
    }
}

impl<'l> Iterator for PathTokenizer<'l> {
    type Item = Result<SvgEvent, ParserError>;
    fn next(&mut self) -> Option<Result<SvgEvent, ParserError>> {
        return match self.tokenizer.next() {
            Some(Ok(segment)) => { Some(Ok(svg_event(&segment))) }
            Some(Err(_)) => { Some(Err(ParserError)) }
            None => { None }
        };
    }
}

fn svg_event(segment: &Segment) -> SvgEvent {
    fn v(x: f64, y: f64) -> Point { point(x as f32, y as f32) }
    return match (segment.cmd, &segment.data) {
        (b'M', &SegmentData::MoveTo { x, y }) => {
            SvgEvent::MoveTo(v(x, y))
        },
        (b'm', &SegmentData::MoveTo { x, y }) => {
            SvgEvent::RelativeMoveTo(v(x, y))
        },
        (b'L', &SegmentData::LineTo { x, y }) => {
            SvgEvent::LineTo(v(x, y))
        },
        (b'l', &SegmentData::LineTo { x, y }) => {
            SvgEvent::RelativeLineTo(v(x, y))
        },
        (b'H', &SegmentData::HorizontalLineTo { x }) => {
            SvgEvent::HorizontalLineTo(x as f32)
        },
        (b'h', &SegmentData::HorizontalLineTo { x }) => {
            SvgEvent::RelativeHorizontalLineTo(x as f32)
        },
        (b'V', &SegmentData::VerticalLineTo { y }) => {
            SvgEvent::VerticalLineTo(y as f32)
        },
        (b'v', &SegmentData::VerticalLineTo { y }) => {
            SvgEvent::RelativeVerticalLineTo(y as f32)
        },
        (b'C', &SegmentData::CurveTo { x1, y1, x2, y2, x, y }) => {
            SvgEvent::CubicTo(v(x1, y1), v(x2, y2), v(x, y))
        },
        (b'c', &SegmentData::CurveTo { x1, y1, x2, y2, x, y }) => {
            SvgEvent::RelativeCubicTo(v(x1, y1), v(x2, y2), v(x, y))
        },
        (b'S', &SegmentData::SmoothCurveTo { x2, y2, x, y }) => {
            SvgEvent::SmoothCubicTo(v(x2, y2), v(x, y))
        },
        (b's', &SegmentData::SmoothCurveTo { x2, y2, x, y }) => {
            SvgEvent::SmoothRelativeCubicTo(v(x2, y2), v(x, y))
        },
        (b'Q', &SegmentData::Quadratic { x1, y1, x, y }) => {
            SvgEvent::QuadraticTo(v(x1, y1), v(x, y))
        },
        (b'q', &SegmentData::Quadratic { x1, y1, x, y }) => {
            SvgEvent::RelativeQuadraticTo(v(x1, y1), v(x, y))
        },
        (b'T', &SegmentData::SmoothQuadratic { x, y }) => {
            SvgEvent::SmoothQuadraticTo(v(x, y))
        },
        (b't', &SegmentData::SmoothQuadratic { x, y }) => {
            SvgEvent::SmoothRelativeQuadraticTo(v(x, y))
        },
        (b'A', &SegmentData::EllipticalArc { rx, ry, x_axis_rotation, large_arc, sweep, x, y }) => {
            SvgEvent::ArcTo(
              v(x, y), v(rx, ry), Radians::new(x_axis_rotation.to_radians() as f32),
              ArcFlags { large_arc: large_arc, sweep: sweep }
            )
        },
        (b'a', &SegmentData::EllipticalArc { rx, ry, x_axis_rotation, large_arc, sweep, x, y }) => {
            SvgEvent::RelativeArcTo(
              v(x, y), v(rx, ry), Radians::new(x_axis_rotation.to_radians() as f32),
              ArcFlags { large_arc: large_arc, sweep: sweep }
            )
        },
        (_, &SegmentData::ClosePath) => { SvgEvent::Close },
        _ => {
            panic!("Unimplemented {:?} {:?}", segment.cmd, segment.data);
        }
    }
}