use lyon::path::geom::euclid::default::{Box2D, Transform2D};
use lyon::path::math::{Point, point};
use lyon::path::Path;
pub type Color = (u8, u8, u8, u8);

pub const FALLBACK_COLOR: Color = (0, 255, 0, 255 );

#[derive(Clone, Debug)]
pub enum SvgPattern {
    Color(Color),
    Gradient { color0: Color, color1: Color, from: Point, to: Point },
}

pub fn load_svg(filename: &str, scale_factor: f32) -> (Box2D<f32>, Vec<(Path, SvgPattern)>) {
    let opt = usvg::Options::default();
    let file_data = std::fs::read(filename).unwrap();
    let rtree = usvg::Tree::from_data(&file_data, &opt).unwrap();
    let mut paths = Vec::new();

    let s = scale_factor;

    let mut gradients = std::collections::HashMap::new();

    let view_box = rtree.svg_node().view_box;
    for node in rtree.root().descendants() {
        use usvg::NodeExt;
        let t = node.transform();
        let transform = Transform2D::new(
            t.a as f32, t.b as f32,
            t.c as f32, t.d as f32,
            t.e as f32, t.f as f32,
        );

        match *node.borrow() {
            usvg::NodeKind::LinearGradient(ref gradient) => {
                let color0 = gradient.base.stops.first().map(|stop| {
                    (
                        stop.color.red,
                        stop.color.green,
                        stop.color.blue,
                        (stop.opacity.value() * 255.0) as u8,
                    )
                }).unwrap_or(FALLBACK_COLOR);
                let color1 = gradient.base.stops.last().map(|stop| {
                    (
                        stop.color.red,
                        stop.color.green,
                        stop.color.blue,
                        (stop.opacity.value() * 255.0) as u8,
                    )
                }).unwrap_or(FALLBACK_COLOR);
                gradients.insert(gradient.id.clone(), SvgPattern::Gradient {
                    color0,
                    color1,
                    from: point(gradient.x1 as f32, gradient.y1 as f32),
                    to: point(gradient.x2 as f32, gradient.y2 as f32),
                });
            }
            usvg::NodeKind::Path(ref usvg_path) => {
                let pattern = match usvg_path.fill {
                    Some(ref fill) => {
                        match fill.paint {
                            usvg::Paint::Color(c) => SvgPattern::Color((c.red, c.green, c.blue, 255)),
                            usvg::Paint::Link(ref id) => {
                                gradients.get(id).cloned().unwrap_or_else(|| {
                                    println!("Could not find pattern {:?}", id);
                                    SvgPattern::Color(FALLBACK_COLOR)
                                })
                            }
                        }
                    }
                    None => {
                        continue;
                    }
                };
    
                let mut builder = Path::builder().with_svg();
                for segment in usvg_path.data.iter() {
                    match *segment {
                        usvg::PathSegment::MoveTo { x, y } => {
                            builder.move_to(transform.transform_point(point(x as f32, y as f32)) * s);
                        }
                        usvg::PathSegment::LineTo { x, y } => {
                            builder.line_to(transform.transform_point(point(x as f32, y as f32)) * s);
                        }
                        usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y, } => {
                            builder.cubic_bezier_to(
                                transform.transform_point(point(x1 as f32, y1 as f32)) * s,
                                transform.transform_point(point(x2 as f32, y2 as f32)) * s,
                                transform.transform_point(point(x as f32, y as f32)) * s,
                            );
                        }
                        usvg::PathSegment::ClosePath => {
                            builder.close();
                        }
                    }
                }
                let path = builder.build();
    
                paths.push((path, pattern));    
            }
            _ => {}
        }
    }

    let vb = Box2D {
        min: point(
            view_box.rect.x() as f32 * s,
            view_box.rect.y() as f32 * s,
        ),
        max: point(
            view_box.rect.x() as f32 + view_box.rect.width() as f32 * s,
            view_box.rect.y() as f32 + view_box.rect.height() as f32 * s,
        ),
    };

    (vb, paths)
}

