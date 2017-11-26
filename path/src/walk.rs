use math::*;
use builder::FlatPathBuilder;

use std::f32;

/// TODO(doc)
pub trait Pattern {
    fn next(&mut self, position: Point, tangent: Vector, distance: f32) -> Option<f32>;
}

/// A helper struct to walk along a flattened path using a builder
/// API.
pub struct PathWalker<'l> {
    prev: Point,
    advancement: f32,
    leftover: f32,
    next_distance: f32,
    first: Point,

    pattern: &'l mut Pattern,
}

impl<'l> PathWalker<'l> {
    pub fn new(start: f32, pattern: &'l mut Pattern) -> PathWalker<'l> {
        let start = f32::max(start, 0.0);
        PathWalker {
            prev: point(0.0, 0.0),
            first: point(0.0, 0.0),
            advancement: 0.0,
            leftover: 0.0,
            next_distance: start,
            pattern,
        }
    }
}

impl<'l> FlatPathBuilder for PathWalker<'l> {
    type PathType = ();

    fn move_to(&mut self, to: Point) {
        self.first = to;
        self.prev = to;
    }

    fn line_to(&mut self, to: Point) {
        let v = to - self.prev;
        let d = v.length();

        if d < 1e-5 {
            return;
        }

        let tangent = v / d;

        let mut distance = self.leftover + d;
        while distance >= self.next_distance {
            let position = self.prev + tangent * (self.next_distance - self.leftover);
            self.prev = position;
            self.leftover = 0.0;
            self.advancement += self.next_distance;
            distance -= self.next_distance;

            self.next_distance = match self.pattern.next(position, tangent, self.advancement) {
                Some(distance) => distance,
                None => { return; }
            }
        }

        self.prev = to;
        self.leftover = distance;
    }

    fn close(&mut self) {
        let first = self.first;
        self.line_to(first);
    }

    fn build(self) -> () { () }

    fn build_and_reset(&mut self) -> () {
        self.first = point(0.0, 0.0);
        self.prev = point(0.0, 0.0);
        self.advancement = 0.0;
    }

    fn current_position(&self) -> Point { self.prev }
}

pub struct RegularPattern<Cb> {
    pub callback: Cb,
    pub interval: f32,
}

impl<Cb> Pattern for RegularPattern<Cb>
where Cb: FnMut(Point, Vector, f32) {
    #[inline]
    fn next(&mut self, position: Point, tangent: Vector, distance: f32) -> Option<f32> {
        (self.callback)(position, tangent, distance);
        Some(self.interval)
    }
}

pub struct RepeatedPattern<'l, Cb> {
    pub callback: Cb,
    pub intervals: &'l[f32],
    pub index: usize,
}

impl<'l, Cb> Pattern for RepeatedPattern<'l, Cb>
where Cb: FnMut(Point, Vector, f32) {
    #[inline]
    fn next(&mut self, position: Point, tangent: Vector, distance: f32) -> Option<f32> {
        (self.callback)(position, tangent, distance);
        let idx = self.index % self.intervals.len();
        self.index += 1;
        Some(self.intervals[idx])
    }
}

impl<Cb> Pattern for Cb
where Cb: FnMut(Point, Vector, f32) -> Option<f32> {
    #[inline]
    fn next(&mut self, position: Point, tangent: Vector, distance: f32) -> Option<f32> {
        (self)(position, tangent, distance)
    }
}

#[test]
fn walk_square() {
    let expected = [
        (point(0.0, 0.0), vector(1.0, 0.0), 0.0),
        (point(2.0, 0.0), vector(1.0, 0.0), 2.0),
        (point(4.0, 0.0), vector(1.0, 0.0), 4.0),
        (point(6.0, 0.0), vector(1.0, 0.0), 6.0),
        (point(6.0, 2.0), vector(0.0, 1.0), 8.0),
        (point(6.0, 4.0), vector(0.0, 1.0), 10.0),
        (point(6.0, 6.0), vector(0.0, 1.0), 12.0),
        (point(4.0, 6.0), vector(-1.0, 0.0), 14.0),
        (point(2.0, 6.0), vector(-1.0, 0.0), 16.0),
        (point(0.0, 6.0), vector(-1.0, 0.0), 18.0),
        (point(0.0, 4.0), vector(0.0, -1.0), 20.0),
        (point(0.0, 2.0), vector(0.0, -1.0), 22.0),
        (point(0.0, 0.0), vector(0.0, -1.0), 24.0),
    ];

    let mut i = 0;
    let mut pattern = RegularPattern {
        interval: 2.0,
        callback: |pos, n, d| {
            println!("p:{:?} n:{:?} d:{:?}", pos, n, d);
            assert_eq!(pos, expected[i].0);
            assert_eq!(n, expected[i].1);
            assert_eq!(d, expected[i].2);
            i += 1;
        },
    };

    let mut walker = PathWalker::new(0.0, &mut pattern);

    walker.move_to(point(0.0, 0.0));
    walker.line_to(point(6.0, 0.0));
    walker.line_to(point(6.0, 6.0));
    walker.line_to(point(0.0, 6.0));
    walker.close();
    walker.build();
}

#[test]
fn walk_with_leftover() {
    let expected = [
        (point(1.0, 0.0), vector(1.0, 0.0), 1.0),
        (point(4.0, 0.0), vector(1.0, 0.0), 4.0),
        (point(5.0, 2.0), vector(0.0, 1.0), 7.0),
        (point(5.0, 5.0), vector(0.0, 1.0), 10.0),
        (point(2.0, 5.0), vector(-1.0, 0.0), 13.0),
        (point(0.0, 4.0), vector(0.0, -1.0), 16.0),
        (point(0.0, 1.0), vector(0.0, -1.0), 19.0),
    ];

    let mut i = 0;
    let mut pattern = RegularPattern {
        interval: 3.0,
        callback: |pos, n, d| {
            println!("p:{:?} n:{:?} d:{:?}", pos, n, d);
            assert_eq!(pos, expected[i].0);
            assert_eq!(n, expected[i].1);
            assert_eq!(d, expected[i].2);
            i += 1;
        }
    };

    let mut walker = PathWalker::new(1.0, &mut pattern);

    walker.move_to(point(0.0, 0.0));
    walker.line_to(point(5.0, 0.0));
    walker.line_to(point(5.0, 5.0));
    walker.line_to(point(0.0, 5.0));
    walker.close();
    walker.build();
}

#[test]
fn walk_starting_after() {
    // With a starting distance that is greater than the path, the
    // callback should never be called.
    let cb = &mut |_, _, _| -> Option<f32> { panic!() };
    let mut walker = PathWalker::new(10.0, cb);

    walker.move_to(point(0.0, 0.0));
    walker.line_to(point(5.0, 0.0));
    walker.build();
}
