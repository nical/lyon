mod load_svg;

use load_svg::load_svg;
use lyon::geom::{LineSegment, QuadraticBezierSegment, CubicBezierSegment, Point, Scalar};
use lyon::geom::euclid::point2 as point;
use lyon::path::{EndpointId, Attributes, PathEvent};
use lyon::path::traits::{Build, PathBuilder};

static TOLERANCES: [f32; 10] = [0.01, 0.025, 0.05, 0.075, 0.1, 0.15, 0.2, 0.25, 0.5, 1.0];

use std::ops::Range;
pub trait Flatten<S> {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb);
    fn flatten_quad<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &QuadraticBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        unimplemented!()
    }
}

pub struct ForwardDifference;
impl<S: Scalar> Flatten<S> for ForwardDifference {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_fd(curve, tolerance, cb);
    }
    fn flatten_quad<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &QuadraticBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::quadratic_bezier::flatten_fd(curve, tolerance, cb);
    }
}

pub struct Hfd;
impl<S: Scalar> Flatten<S> for Hfd {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_hfd(curve, tolerance, cb);
    }
}

pub struct Cagd;
impl<S: Scalar> Flatten<S> for Cagd {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_cagd(curve, tolerance, cb);
    }
    fn flatten_quad<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &QuadraticBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::quadratic_bezier::flatten_cagd(curve, tolerance, cb);
    }
}

pub struct Levien;
impl<S: Scalar> Flatten<S> for Levien {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_levien(curve, tolerance, cb);
    }
    fn flatten_quad<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &QuadraticBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        curve.for_each_flattened_with_t(tolerance, cb);
    }
}

pub struct Linear;
impl<S: Scalar> Flatten<S> for Linear {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_linear(curve, tolerance, cb);
    }
}

pub struct Linear2;
impl<S: Scalar> Flatten<S> for Linear2 {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_linear2(curve, tolerance, cb);
    }
    fn flatten_quad<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &QuadraticBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::quadratic_bezier::flatten_linear2(curve, tolerance, cb);
    }
}

pub struct Recursive;
impl<S: Scalar> Flatten<S> for Recursive {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_recursive(curve, tolerance, cb);
    }
    fn flatten_quad<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &QuadraticBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::quadratic_bezier::flatten_recursive(curve, tolerance, cb);
    }
}

pub struct RecursiveHfd;
impl<S: Scalar> Flatten<S> for RecursiveHfd {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_recursive_hfd(curve, tolerance, cb);
    }
}

pub struct RecursiveAgg;
impl<S: Scalar> Flatten<S> for RecursiveAgg {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_recursive_agg(curve, tolerance, cb);
    }
}

pub struct Pa;
impl<S: Scalar> Flatten<S> for Pa {
    fn flatten<Cb: FnMut(&LineSegment<S>, Range<S>)>(curve: &CubicBezierSegment<S>, tolerance: S, cb: &mut Cb) {
        lyon::geom::cubic_bezier::flatten_pa(curve, tolerance, cb);
    }
}

// Extracts the curves from the rust logo.
pub fn generate_bezier_curves2() -> Vec<CubicBezierSegment<f32>> {
    use lyon::extra::rust_logo::build_logo_path;

    struct CubicBeziers { current: Point<f32>, curves: Vec<CubicBezierSegment<f32>> }

    impl PathBuilder for CubicBeziers {
        fn num_attributes(&self) -> usize { 0 }
        fn begin(&mut self, at: Point<f32>, _: Attributes) -> EndpointId {
            self.current = at;
            EndpointId(0)
        }
        fn end(&mut self, _: bool) {}
        fn line_to(&mut self, to: Point<f32>, _: Attributes) -> EndpointId {
            let from = self.current;
            self.curves.push(CubicBezierSegment { from, ctrl1: from, ctrl2: from.lerp(to, 0.5), to });
            self.current = to;
            EndpointId(0)
        }
        fn quadratic_bezier_to(&mut self, ctrl: Point<f32>, to: Point<f32>, _: Attributes) -> EndpointId {
            let from = self.current;
            self.curves.push(QuadraticBezierSegment { from, ctrl, to}.to_cubic());
            self.current = to;
            EndpointId(0)
        }
        fn cubic_bezier_to(&mut self, ctrl1: Point<f32>, ctrl2: Point<f32>, to: Point<f32>, _: Attributes) -> EndpointId {
            let from = self.current;
            self.curves.push(CubicBezierSegment { from, ctrl1, ctrl2, to });
            self.current = to;
            EndpointId(0)
        }
    }

    impl Build for CubicBeziers {
        type PathType = Vec<CubicBezierSegment<f32>>;
        fn build(self) -> Vec<CubicBezierSegment<f32>> { self.curves }
    }

    let mut curves = CubicBeziers { curves: Vec::new(), current: point(0.0, 0.0) }.with_svg();

    build_logo_path(&mut curves);

    curves.build()
}

pub fn generate_bezier_curves() -> Vec<CubicBezierSegment<f32>> {
    let (_, paths) = load_svg("tiger.svg", 1.0);

    let mut curves = Vec::new();
    for (path, _) in paths {
        for evt in path.iter() {
            match evt {
                PathEvent::Cubic { from, ctrl1, ctrl2, to } => {
                    curves.push(CubicBezierSegment { from, ctrl1, ctrl2, to });
                }
                PathEvent::Quadratic { from, ctrl, to } => {
                    curves.push(QuadraticBezierSegment { from, ctrl, to }.to_cubic());
                }
                _ => {}
            }
        }
    }

    curves
}


pub fn generate_quadratic_curves() -> Vec<QuadraticBezierSegment<f32>> {
    let cubics = generate_bezier_curves();
    let mut quads = Vec::new();
    for cubic in &cubics {
        cubic.for_each_quadratic_bezier(0.25, &mut |quad| {
            quads.push(*quad);
        })
    }

    quads
}

fn count_edges_cubic<F: Flatten<f32>>(curves: &[CubicBezierSegment<f32>], tolerance: f32) -> u32 {
    let mut count = 0;
    for curve in curves {
        F::flatten(curve, tolerance, &mut |_, _| { count += 1; });
    }

    count
}

fn count_edges_quad<F: Flatten<f32>>(curves: &[QuadraticBezierSegment<f32>], tolerance: f32) -> u32 {
    let mut count = 0;
    for curve in curves {
        F::flatten_quad(curve, tolerance, &mut |_, _| { count += 1; });
    }

    count
}

#[test]
fn flatten_edge_count() {
    let curves = generate_bezier_curves();
    let mut pa = Vec::new();
    let mut rec = Vec::new();
    let mut rec_hfd = Vec::new();
    let mut rec_agg = Vec::new();
    let mut linear = Vec::new();
    let mut linear2 = Vec::new();
    let mut levien = Vec::new();
    let mut fd = Vec::new();
    let mut hfd = Vec::new();
    let mut cagd = Vec::new();
    for tolerance in TOLERANCES {
        pa.push(count_edges_cubic::<Pa>(&curves, tolerance));
        rec.push(count_edges_cubic::<Recursive>(&curves, tolerance));
        rec_hfd.push(count_edges_cubic::<RecursiveHfd>(&curves, tolerance));
        rec_agg.push(count_edges_cubic::<RecursiveAgg>(&curves, tolerance));
        linear.push(count_edges_cubic::<Linear>(&curves, tolerance));
        linear2.push(count_edges_cubic::<Linear2>(&curves, tolerance));
        levien.push(count_edges_cubic::<Levien>(&curves, tolerance));
        fd.push(count_edges_cubic::<ForwardDifference>(&curves, tolerance));
        hfd.push(count_edges_cubic::<Hfd>(&curves, tolerance));
        cagd.push(count_edges_cubic::<Cagd>(&curves, tolerance));
    }

    fn print_first_row() {
        print!("|tolerance\t");
        for tolerance in &TOLERANCES {
            print!("| {}", tolerance);
        }
        println!("|");
        print!("|----------");
        for _ in 0..TOLERANCES.len() {
            print!("| -----:");
        }
        println!("|");    
    }

    fn print_edges(name: &str, vals: &[u32]) {
        print!("|{}", name);
        for val in vals {
            print!("| {:.2}\t", val);
        }
        println!("|");    
    }

    fn print_first_row_csv() {
        print!("tolerance, ");
        for tolerance in &TOLERANCES {
            print!("{}, ", tolerance);
        }
    }

    fn print_edges_csv(name: &str, vals: &[u32]) {
        print!("{}, ", name);
        for val in vals {
            print!("{:.2}, ", val);
        }
        println!(",");
    }

    println!("Cubic bézier curves:");
    print_first_row_csv();
    print_edges_csv("recursive ", &rec);
    print_edges_csv("rec_hfd   ", &rec_hfd);
    print_edges_csv("rec_agg   ", &rec_agg);
    print_edges_csv("fwd-diff  ", &fd);
    print_edges_csv("hfd       ", &hfd);
    print_edges_csv("pa        ", &pa);
    print_edges_csv("levien    ", &levien);
    print_edges_csv("linear    ", &linear);
    print_edges_csv("linear2   ", &linear2);
    print_edges_csv("cagd      ", &cagd);

    println!();

    let curves = generate_quadratic_curves();
    let mut rec = Vec::new();
    let mut linear2 = Vec::new();
    let mut levien = Vec::new();
    let mut fd = Vec::new();
    let mut cagd = Vec::new();
    for tolerance in TOLERANCES {
        rec.push(count_edges_quad::<Recursive>(&curves, tolerance));
        linear2.push(count_edges_quad::<Linear2>(&curves, tolerance));
        levien.push(count_edges_quad::<Levien>(&curves, tolerance));
        fd.push(count_edges_quad::<ForwardDifference>(&curves, tolerance));
        cagd.push(count_edges_quad::<Cagd>(&curves, tolerance));
    }

    println!("Quadratic bézier curves:");
    print_first_row_csv();
    print_edges_csv("recursive ", &rec);
    print_edges_csv("fwd-diff  ", &fd);
    print_edges_csv("levien    ", &levien);
    print_edges_csv("linear2   ", &linear2);
    print_edges_csv("cagd      ", &cagd);

    panic!();
}

#[test]
fn flatten_tolerances() {
    fn check_flattener<F: Flatten<f64>>(name: &str) {
        let curves = generate_bezier_curves();
        for tolerance in TOLERANCES {
            tolerance_test::<F>(name, tolerance as f64, &curves);
        }
    }

    fn tolerance_test<F: Flatten<f64>>(name: &str, tolerance: f64, curves: &[CubicBezierSegment<f32>]) {
        let mut max_error: f64 = 0.0;
        let mut failures: u32 = 0;
        for curve in curves {
            let curve = CubicBezierSegment {
                from: curve.from.to_f64(),
                ctrl1: curve.ctrl1.to_f64(),
                ctrl2: curve.ctrl2.to_f64(),
                to: curve.to.to_f64(),
            };
            F::flatten(&curve, tolerance, &mut |seg, range| {
                const STEPS: u32 = 300;
                let check_range = (range.end - range.start) * 0.5;
                let step = check_range / (STEPS as f64 );
                let mut failed = false;
                for i in 0..STEPS {
                    let t = (range.start + check_range * 0.25 + i as f64 * step).min(range.end);
                    let s = curve.sample(t);
                    let err = seg.distance_to_point(s);
                    let df = curve.sample(range.start) - seg.from;
                    let dt = curve.sample(range.end) - seg.to;
                    assert!(df.length() < 0.05, "{} start {:?} sample {:?} should be {:?} error {:?}", name, range.start, curve.sample(range.start), seg.from, df);
                    assert!(dt.length() < 0.05, "{} end {:?} sample {:?} should be {:?} error {:?}", name, range.end, curve.sample(range.end), seg.to, dt);
                    if err > tolerance * 1.2 {
                        //println!("!! {:?} tolerance {:?} error {:.3?} at t={:.3?} segment {:.3?} sample {:.3?}", name, tolerance, err, t, seg, s);
                        failed = true;
                    }
                    max_error = max_error.max(err);
                }
                if failed {
                    failures += 1;
                }
            });
        }

        let ok = max_error <= tolerance * 1.1;
        //assert!(ok, "--> {:?} tolerance {:?} failures: {:?} max error {:?}", name, tolerance, failures, max_error);
        println!("{}\t\t tolerance {:.3?}, {} failures, max error {:.3} ({:.3})", name, tolerance, failures, max_error, max_error / tolerance);
    }

    check_flattener::<Recursive>("rec");
    check_flattener::<Linear>("linear");
    check_flattener::<Linear2>("linear2");
    check_flattener::<RecursiveHfd>("rec_hfd");
    check_flattener::<Hfd>("hfd");
    check_flattener::<Cagd>("cagd");
    check_flattener::<ForwardDifference>("fwddiff");
    check_flattener::<Pa>("pa");
    check_flattener::<Levien>("levien");
    panic!();
}
