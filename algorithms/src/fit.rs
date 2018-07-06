//! Fit paths into rectangles.

use math::*;
use aabb::bounding_rect;
use path::default::Path;
use path::iterator::*;
use path::builder::*;

/// The strategy to use when fitting (stretching, overflow, etc.)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FitStyle {
    /// Stretch vertically and horizontally to fit the destination rectangle exactly.
    Stretch,
    /// Uniformly scale without overflow.
    Min,
    /// Uniformly scale with overflow.
    Max,
    /// Uniformly scale to fit horizontally.
    Horizontal,
    /// Uniformly scale to fit vertically.
    Vertical,
}

/// Computes a transform that fits a rectangle into another one.
pub fn fit_rectangle(src_rect: &Rect, dst_rect: &Rect, style: FitStyle) -> Transform2D {
    let scale: Vector = vector(
        dst_rect.size.width / src_rect.size.width,
        dst_rect.size.height / src_rect.size.height
    );

    let scale = match style {
        FitStyle::Stretch => scale,
        FitStyle::Min => {
            let s = f32::min(scale.x, scale.y);
            vector(s, s)
        }
        FitStyle::Max => {
            let s = f32::max(scale.x, scale.y);
            vector(s, s)
        }
        FitStyle::Horizontal => vector(scale.x, scale.x),
        FitStyle::Vertical => vector(scale.y, scale.y),
    };

    let src_center = src_rect.origin.lerp(src_rect.bottom_right(), 0.5);
    let dst_center = dst_rect.origin.lerp(dst_rect.bottom_right(), 0.5);

    Transform2D::create_translation(-src_center.x, -src_center.y)
        .post_scale(scale.x, scale.y)
        .post_translate(dst_center.to_vector())
}

/// Fits a path into a rectangle.
pub fn fit_path(path: &Path, output_rect: &Rect, style: FitStyle) -> Path {
    let aabb = bounding_rect(path.iter());
    let transform = fit_rectangle(&aabb, output_rect, style);

    let mut builder = Path::builder();
    for evt in path.path_iter().transformed(&transform) {
        builder.path_event(evt)
    }

    builder.build()
}

#[test]
fn simple_fit() {
    fn approx_eq(a: &Rect, b: &Rect) -> bool {
        use geom::euclid::approxeq::ApproxEq;
        let result = a.origin.approx_eq(&b.origin) && a.bottom_right().approx_eq(&b.bottom_right());
        if !result {
            println!("{:?} == {:?}", a, b);
        }
        result
    }

    let t = fit_rectangle(
        &rect(0.0, 0.0, 1.0, 1.0),
        &rect(0.0, 0.0, 2.0, 2.0),
        FitStyle::Stretch
    );

    assert!(approx_eq(
        &t.transform_rect(&rect(0.0, 0.0, 1.0, 1.0)),
        &rect(0.0, 0.0, 2.0, 2.0)
    ));

    let t = fit_rectangle(
        &rect(1.0, 2.0, 4.0, 4.0),
        &rect(0.0, 0.0, 2.0, 8.0),
        FitStyle::Stretch
    );

    assert!(approx_eq(
        &t.transform_rect(&rect(1.0, 2.0, 4.0, 4.0)),
        &rect(0.0, 0.0, 2.0, 8.0)
    ));

    let t = fit_rectangle(
        &rect(1.0, 2.0, 2.0, 4.0),
        &rect(0.0, 0.0, 2.0, 2.0),
        FitStyle::Horizontal
    );

    assert!(approx_eq(
        &t.transform_rect(&rect(1.0, 2.0, 2.0, 4.0)),
        &rect(0.0, -1.0, 2.0, 4.0)
    ));

    let t = fit_rectangle(
        &rect(1.0, 2.0, 2.0, 2.0),
        &rect(0.0, 0.0, 4.0, 2.0),
        FitStyle::Horizontal
    );

    assert!(approx_eq(
        &t.transform_rect(&rect(1.0, 2.0, 2.0, 2.0)),
        &rect(0.0, -1.0, 4.0, 4.0)
    ));
}

