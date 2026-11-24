// Ported from ardia packages/ardiacore/ios/ArdiaPiPThrowMath.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn max_x(&self) -> f64 {
        self.x + self.w
    }

    pub fn max_y(&self) -> f64 {
        self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub const CORNERS: [Corner; 4] = [
    Corner::TopLeft,
    Corner::TopRight,
    Corner::BottomLeft,
    Corner::BottomRight,
];

pub const PI: f64 = std::f64::consts::PI;
pub const CORNER_INSET: f64 = 10.0;
pub const BROWSER_CARD_INSET: f64 = 9.0;
pub const BROWSER_CARD_RADIUS: f64 = 10.0;
pub const FREE_RADIUS: f64 = 6.0;
pub const SNAPPED_RADIUS: f64 = BROWSER_CARD_RADIUS - (CORNER_INSET - BROWSER_CARD_INSET);
pub const DRAG_FLING_SPEED: f64 = 350.0;
pub const SWIPE_SURE_FLING_DELTA: f64 = 10.0;
pub const TUCK_PAST_EDGE: f64 = 0.5;
pub const UNTUCK_PULL: f64 = 48.0;
pub const CORNER_PULL: f64 = 80.0;
pub const SWIPE_EVENTS_PER_SECOND: f64 = 90.0;
pub const RESPONSE: f64 = 0.9;
pub const DAMPING_RATIO: f64 = 0.9;
pub const RESOLVE: f64 = 0.2;
pub const VELOCITY_WINDOW: f64 = 0.1;
pub const VELOCITY_STALE: f64 = 0.05;
pub const MOMENTUM_WAIT: f64 = 0.08;
pub const MOMENTUM_LATE: f64 = 0.3;
pub const FLICK_REACH: f64 = 0.1;
pub const SIDEWAYS_FLICK: f64 = 2.0;

/// `std::clamp` as libc++ computes it: no panic when `lo > hi` or on NaN, unlike `f64::clamp`.
pub fn std_clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if hi < v {
        hi
    } else {
        v
    }
}

fn is_left(corner: Corner) -> bool {
    matches!(corner, Corner::TopLeft | Corner::BottomLeft)
}

fn is_top(corner: Corner) -> bool {
    matches!(corner, Corner::TopLeft | Corner::TopRight)
}

pub fn corner_point(corner: Corner, r: Rect) -> Point {
    Point {
        x: if is_left(corner) { r.x } else { r.max_x() },
        y: if is_top(corner) { r.max_y() } else { r.y },
    }
}

pub fn angle_apart(a: f64, b: f64) -> f64 {
    let mut d = (a - b).abs();
    d -= (d / (2.0 * PI)).floor() * 2.0 * PI;
    if d < PI { d } else { 2.0 * PI - d }
}

pub fn pick_corner(from: Point, velocity: Point, screen: Rect) -> Option<Corner> {
    if velocity.x == 0.0 && velocity.y == 0.0 {
        return None;
    }
    let heading = velocity.y.atan2(velocity.x);
    let mut best = None;
    let mut closest = 2.0 * PI;
    for corner in CORNERS {
        let p = corner_point(corner, screen);
        let apart = angle_apart((p.y - from.y).atan2(p.x - from.x), heading);
        if apart < closest {
            best = Some(corner);
            closest = apart;
        }
    }
    best
}

pub fn corner_frame(corner: Corner, w: f64, h: f64, visible: Rect) -> Rect {
    let inner = Rect {
        x: visible.x + CORNER_INSET,
        y: visible.y + CORNER_INSET,
        w: visible.w - 2.0 * CORNER_INSET,
        h: visible.h - 2.0 * CORNER_INSET,
    };
    Rect {
        x: (if is_left(corner) {
            inner.x
        } else {
            inner.max_x() - w
        })
        .round(),
        y: (if is_top(corner) {
            inner.max_y() - h
        } else {
            inner.y
        })
        .round(),
        w,
        h,
    }
}

pub fn snapped_corner(frame: Rect, visible: Rect) -> Option<Corner> {
    CORNERS.into_iter().find(|&corner| {
        let at = corner_frame(corner, frame.w, frame.h, visible);
        (at.x - frame.x).abs() <= 0.5 && (at.y - frame.y).abs() <= 0.5
    })
}

pub fn flick_landing(frame: Rect, velocity: Point) -> Rect {
    if velocity.x.abs() < SIDEWAYS_FLICK * velocity.y.abs() {
        return frame;
    }
    Rect {
        x: frame.x + velocity.x * FLICK_REACH,
        ..frame
    }
}

pub fn corner_in_reach(frame: Rect, visible: Rect) -> Option<Corner> {
    let mut best = None;
    let mut closest = CORNER_PULL;
    for corner in CORNERS {
        let at = corner_frame(corner, frame.w, frame.h, visible);
        let apart = (at.x - frame.x).hypot(at.y - frame.y);
        if apart <= closest {
            best = Some(corner);
            closest = apart;
        }
    }
    best
}

pub fn corner_pulled_into(start: Rect, released: Rect, visible: Rect) -> Option<Corner> {
    let corner = corner_in_reach(released, visible);
    if corner.is_none() || corner_in_reach(start, visible) == corner {
        None
    } else {
        corner
    }
}

pub fn past_edge_enough_to_tuck(past: f64, width: f64) -> bool {
    past > width * TUCK_PAST_EDGE
}

pub fn pulled_out_of_tuck(from_tucked: f64) -> bool {
    from_tucked >= UNTUCK_PULL
}

pub fn pulled_past_half(frame: Rect, visible: Rect, tucked_right: bool) -> bool {
    let middle = visible.x + visible.w / 2.0;
    let centre = frame.x + frame.w / 2.0;
    if tucked_right {
        centre < middle
    } else {
        centre > middle
    }
}

pub fn corner_radius(frame: Rect, visible: Rect) -> f64 {
    if snapped_corner(frame, visible).is_none() {
        FREE_RADIUS
    } else {
        SNAPPED_RADIUS
    }
}

pub fn finger_delta(scrolling_delta_x: f64, scrolling_delta_y: f64, inverted: bool) -> Point {
    if inverted {
        Point {
            x: scrolling_delta_x,
            y: -scrolling_delta_y,
        }
    } else {
        Point {
            x: -scrolling_delta_x,
            y: scrolling_delta_y,
        }
    }
}

pub fn swipe_velocity(last_delta: Point) -> Point {
    Point {
        x: last_delta.x * SWIPE_EVENTS_PER_SECOND,
        y: last_delta.y * SWIPE_EVENTS_PER_SECOND,
    }
}

pub fn swipe_surely_flings(last_delta: Point) -> bool {
    last_delta.x.hypot(last_delta.y) >= SWIPE_SURE_FLING_DELTA
}

pub fn drag_flings(velocity: Point) -> bool {
    velocity.x.hypot(velocity.y) > DRAG_FLING_SPEED
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample {
    pub t: f64,
    pub x: f64,
    pub y: f64,
}

pub fn release_velocity(samples: &[Sample], released_at: f64) -> Point {
    let count = samples.len();
    let mut first = count;
    while first > 0 && samples[first - 1].t >= released_at - VELOCITY_WINDOW {
        first -= 1;
    }
    if count - first < 2 || released_at - samples[count - 1].t > VELOCITY_STALE {
        return Point::default();
    }
    let (a, b) = (samples[first], samples[count - 1]);
    let dt = b.t - a.t;
    if dt <= 0.0 {
        return Point::default();
    }
    Point {
        x: (b.x - a.x) / dt,
        y: (b.y - a.y) / dt,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Screen {
    pub frame: Rect,
    pub visible: Rect,
}

pub fn largest_overlap(r: Rect, screens: &[Screen]) -> usize {
    let mut best = 0;
    let mut best_area = 0.0;
    for (i, screen) in screens.iter().enumerate() {
        let f = screen.frame;
        let w = r.max_x().min(f.max_x()) - r.x.max(f.x);
        let h = r.max_y().min(f.max_y()) - r.y.max(f.y);
        if w > 0.0 && h > 0.0 && w * h > best_area {
            best = i;
            best_area = w * h;
        }
    }
    best
}

fn home_or_largest(r: Rect, screens: &[Screen], home: Option<usize>) -> usize {
    home.filter(|&home| home < screens.len())
        .unwrap_or_else(|| largest_overlap(r, screens))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Above,
    Below,
    Left,
    Right,
}

pub fn display_beyond(window: Rect, current: usize, screens: &[Screen], side: Side) -> bool {
    let edge = screens[current].frame;
    screens.iter().enumerate().any(|(i, screen)| {
        let f = screen.frame;
        let across = window.max_x().min(f.max_x()) > window.x.max(f.x);
        let along = window.max_y().min(f.max_y()) > window.y.max(f.y);
        let (touches, beside) = match side {
            Side::Above => ((f.y - edge.max_y()).abs() <= 1.0, across),
            Side::Below => ((f.max_y() - edge.y).abs() <= 1.0, across),
            Side::Left => ((f.max_x() - edge.x).abs() <= 1.0, along),
            Side::Right => ((f.x - edge.max_x()).abs() <= 1.0, along),
        };
        i != current && touches && beside
    })
}

pub fn usable(i: usize, window: Rect, screens: &[Screen]) -> Rect {
    let (v, f) = (screens[i].visible, screens[i].frame);
    let beyond = |side| display_beyond(window, i, screens, side);
    let x0 = if beyond(Side::Left) { f.x } else { v.x };
    let x1 = if beyond(Side::Right) {
        f.max_x()
    } else {
        v.max_x()
    };
    let y0 = if beyond(Side::Below) { f.y } else { v.y };
    let y1 = if beyond(Side::Above) {
        f.max_y()
    } else {
        v.max_y()
    };
    Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    }
}

pub const MIN_VISIBLE_WIDTH: f64 = 60.0;
pub const MIN_VISIBLE_SHARE: f64 = 0.4;

pub fn shortfall(p: Rect, screens: &[Screen]) -> f64 {
    let mut reach = f64::INFINITY;
    let mut top = -f64::INFINITY;
    let mut bottom = f64::INFINITY;
    let mut overlapping = false;
    for (i, screen) in screens.iter().enumerate() {
        let u = usable(i, p, screens);
        let wide = 0.0f64.max(p.max_x().min(u.max_x()) - p.x.max(u.x));
        let tall = 0.0f64.max(p.max_y().min(u.max_y()) - p.y.max(u.y));
        reach = reach.min(
            0.0f64.max(MIN_VISIBLE_WIDTH.min(p.w) - wide)
                + 0.0f64.max((MIN_VISIBLE_SHARE * p.h).min(u.h) - tall),
        );
        let v = screen.visible;
        if p.max_x().min(v.max_x()) > p.x.max(v.x) {
            overlapping = true;
            top = top.max(v.max_y());
            bottom = bottom.min(v.y);
        }
    }
    if !overlapping {
        return reach;
    }
    reach + 0.0f64.max(p.max_y() - top) + 0.0f64.max(bottom - p.y)
}

pub fn shifted(r: Rect, dx: f64, dy: f64) -> Rect {
    Rect {
        x: r.x + dx,
        y: r.y + dy,
        ..r
    }
}

pub fn outside(v: f64, size: f64, start: f64, extent: f64) -> f64 {
    let (lo, hi) = if size > extent {
        (start + extent - size, start)
    } else {
        (start, start + extent - size)
    };
    if v < lo {
        lo - v
    } else if v > hi {
        v - hi
    } else {
        0.0
    }
}

pub struct Mover<'a> {
    pub screens: &'a [Screen],
    pub boxed: bool,
    pub bounds: Rect,
}

impl<'a> Mover<'a> {
    pub fn new(from: Rect, screens: &'a [Screen], home: Option<usize>) -> Self {
        let mut mover = Mover {
            screens,
            boxed: false,
            bounds: Rect::default(),
        };
        if screens.is_empty() {
            return mover;
        }
        let visible = screens[home_or_largest(from, screens, home)].visible;
        mover.boxed = from.w > visible.w || from.h > visible.h;
        mover.bounds = visible;
        mover
    }

    pub fn shortfall(&self, p: Rect) -> f64 {
        if self.screens.is_empty() {
            return 0.0;
        }
        let b = self.bounds;
        if self.boxed {
            return outside(p.x, p.w, b.x, b.w) + outside(p.y, p.h, b.y, b.h);
        }
        shortfall(p, self.screens)
    }

    pub fn allowed(&self, next: Rect, current: Rect) -> bool {
        if self.screens.is_empty() {
            return true;
        }
        let b = self.bounds;
        if self.boxed {
            return outside(next.x, next.w, b.x, b.w)
                <= outside(current.x, current.w, b.x, b.w) + 1e-9
                && outside(next.y, next.h, b.y, b.h)
                    <= outside(current.y, current.h, b.y, b.h) + 1e-9;
        }
        self.shortfall(next) <= self.shortfall(current).max(0.0) + 1e-9
    }

    pub fn along(&self, r: Rect, dx: f64, dy: f64) -> Rect {
        if dx == 0.0 && dy == 0.0 {
            return r;
        }
        if self.allowed(shifted(r, dx, dy), r) {
            return shifted(r, dx, dy);
        }
        let length = dx.abs().max(dy.abs());
        let mut k = length.ceil() as i64 - 1;
        while k >= 1 {
            let candidate = shifted(r, dx * k as f64 / length, dy * k as f64 / length);
            if self.allowed(candidate, r) {
                return candidate;
            }
            k -= 1;
        }
        r
    }

    pub fn move_by(&self, frame: Rect, delta: Point) -> Rect {
        if self.screens.is_empty() || self.allowed(shifted(frame, delta.x, delta.y), frame) {
            return shifted(frame, delta.x, delta.y);
        }
        self.along(self.along(frame, delta.x, 0.0), 0.0, delta.y)
    }
}

pub fn reachable(p: Rect, screens: &[Screen]) -> bool {
    if screens.is_empty() {
        return true;
    }
    screens.iter().any(|screen| {
        let v = screen.visible;
        let wide = p.max_x().min(v.max_x()) - p.x.max(v.x);
        let tall = p.max_y().min(v.max_y()) - p.y.max(v.y);
        wide >= MIN_VISIBLE_WIDTH.min(p.w) - 1e-9
            && tall >= (MIN_VISIBLE_SHARE * p.h).min(v.h) - 1e-9
    })
}

pub fn settle(v: f64, size: f64, start: f64, extent: f64) -> f64 {
    if size > extent {
        std_clamp(v, start + extent - size, start)
    } else {
        std_clamp(v, start, start + extent - size)
    }
}

pub fn rescue(p: Rect, screens: &[Screen], home: Option<usize>) -> Rect {
    if screens.is_empty() || reachable(p, screens) {
        return p;
    }
    let v = screens[home_or_largest(p, screens, home)].visible;
    Rect {
        x: settle(p.x, p.w, v.x, v.w).round(),
        y: settle(p.y, p.h, v.y, v.h).round(),
        ..p
    }
}

pub fn move_within(frame: Rect, delta: Point, screens: &[Screen], home: Option<usize>) -> Rect {
    Mover::new(frame, screens, home).move_by(frame, delta)
}

pub fn swipe_step(frame: Rect, delta: Point, screens: &[Screen], home: Option<usize>) -> Rect {
    move_within(frame, delta, screens, home)
}

pub fn drag_step(frame: Rect, delta: Point, screens: &[Screen], home: Option<usize>) -> Rect {
    move_within(frame, delta, screens, home)
}

pub fn grabbed_origin(pointer: Point, grab: Point) -> Point {
    Point {
        x: (pointer.x - grab.x).round(),
        y: (pointer.y - grab.y).round(),
    }
}

pub fn drag_to(
    frame: Rect,
    pointer: Point,
    grab: Point,
    screens: &[Screen],
    home: Option<usize>,
) -> Rect {
    let origin = grabbed_origin(pointer, grab);
    let delta = Point {
        x: origin.x - frame.x,
        y: origin.y - frame.y,
    };
    move_within(frame, delta, screens, home)
}

pub fn held_back(placed: Rect, pointer: Point, grab: Point) -> bool {
    let origin = grabbed_origin(pointer, grab);
    (placed.x - origin.x).abs() > 1e-6 || (placed.y - origin.y).abs() > 1e-6
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpringState {
    pub offset: f64,
    pub velocity: f64,
}

pub fn spring_at(offset: f64, velocity: f64, t: f64, response: f64, ratio: f64) -> SpringState {
    let w0 = 2.0 * PI / response.max(0.0001);
    let decay = ratio * w0;
    let wd = w0 * (1.0 - ratio * ratio).max(1e-9).sqrt();
    let envelope = (-decay * t).exp();
    let (c, s) = ((wd * t).cos(), (wd * t).sin());
    let b = (velocity + decay * offset) / wd;
    SpringState {
        offset: envelope * (offset * c + b * s),
        velocity: envelope * (velocity * c - (decay * velocity + w0 * w0 * offset) / wd * s),
    }
}

pub fn resolved(x: SpringState, y: SpringState) -> bool {
    x.offset.abs() < RESOLVE
        && y.offset.abs() < RESOLVE
        && x.velocity.abs() < RESOLVE
        && y.velocity.abs() < RESOLVE
}

#[cfg(test)]
mod tests {
    use super::Corner::{BottomLeft, BottomRight, TopLeft, TopRight};
    use super::*;

    const fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    const fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    const fn screen(frame: Rect, visible: Rect) -> Screen {
        Screen { frame, visible }
    }

    const fn sample(t: f64, x: f64, y: f64) -> Sample {
        Sample { t, x, y }
    }

    fn near(a: f64, b: f64) -> bool {
        near_by(a, b, 1e-9)
    }

    fn near_by(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.max_x() && b.x < a.max_x() && a.y < b.max_y() && b.y < a.max_y()
    }

    fn inside(w: Rect, v: Rect) -> bool {
        w.x >= v.x - 1e-9
            && w.y >= v.y - 1e-9
            && w.max_x() <= v.max_x() + 1e-9
            && w.max_y() <= v.max_y() + 1e-9
    }

    struct Random {
        state: u64,
    }

    impl Random {
        fn next(&mut self) -> u64 {
            self.state = self
                .state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.state >> 33
        }

        fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * (self.next() % 1000000) as f64 / 1000000.0
        }

        fn integer(&mut self, lo: i32, hi: i32) -> i32 {
            lo + (self.next() % (hi - lo + 1) as u64) as i32
        }

        fn chance(&mut self, p: f64) -> bool {
            self.uniform(0.0, 1.0) < p
        }
    }

    fn inset(r: &mut Random, frame: Rect, menu_bar: bool) -> Rect {
        let mut v = frame;
        if menu_bar {
            v.h -= f64::from(r.integer(24, 40));
        }
        if r.chance(0.4) {
            let dock = f64::from(r.integer(40, 100));
            match r.integer(0, 2) {
                0 => {
                    v.y += dock;
                    v.h -= dock;
                }
                1 => {
                    v.x += dock;
                    v.w -= dock;
                }
                _ => v.w -= dock,
            }
        }
        v
    }

    fn layout(r: &mut Random, out: &mut [Screen; 3], half_points: bool) -> usize {
        let w0 = f64::from(r.integer(1024, 2560));
        let h0 = f64::from(r.integer(640, 1440));
        out[0].frame = rect(0.0, 0.0, w0, h0);
        out[0].visible = inset(r, out[0].frame, true);
        let mut count = 1;
        let extra = r.integer(0, 2) as usize;
        for _attempt in 0..40 {
            if count > extra {
                break;
            }
            let base = out[r.integer(0, count as i32 - 1) as usize].frame;
            let w = f64::from(if r.chance(0.2) {
                r.integer(300, 800)
            } else {
                r.integer(1024, 2560)
            });
            let h = f64::from(if r.chance(0.2) {
                r.integer(300, 600)
            } else {
                r.integer(640, 1440)
            });
            let mut f = rect(0.0, 0.0, w, h);
            match r.integer(0, 3) {
                0 => {
                    f.y = base.max_y();
                    f.x = base.x + r.uniform(-w + 100.0, base.w - 100.0);
                }
                1 => {
                    f.y = base.y - h;
                    f.x = base.x + r.uniform(-w + 100.0, base.w - 100.0);
                }
                2 => {
                    f.x = base.max_x();
                    f.y = base.y + r.uniform(-h + 100.0, base.h - 100.0);
                }
                _ => {
                    f.x = base.x - w;
                    f.y = base.y + r.uniform(-h + 100.0, base.h - 100.0);
                }
            }
            f.x = f.x.round();
            f.y = f.y.round();
            if half_points {
                f.x += 0.5;
                f.y += 0.5;
            }
            if out[..count].iter().any(|s| overlaps(f, s.frame)) {
                continue;
            }
            out[count].frame = f;
            let menu_bar = r.chance(0.5);
            out[count].visible = inset(r, f, menu_bar);
            count += 1;
        }
        count
    }

    fn describe(s: &[Screen], start: Rect, walk: &[Point], failed_at: usize, what: &str) {
        println!("PROPERTY FAIL ({what}) at step {failed_at}");
        print_screens(s);
        print!("  start {start:?}\n  walk:");
        for p in walk.iter().take(failed_at + 1) {
            print!(" {{{},{}}}", p.x, p.y);
        }
        println!();
    }

    fn print_screens(s: &[Screen]) {
        for (i, screen) in s.iter().enumerate() {
            println!(
                "  screen {i} frame {:?} visible {:?}",
                screen.frame, screen.visible
            );
        }
    }

    #[test]
    fn pick_corner_follows_the_heading() {
        let screen = rect(0.0, 0.0, 1512.0, 982.0);
        let middle = pt(756.0, 491.0);
        assert_eq!(
            pick_corner(middle, pt(1000.0, 650.0), screen),
            Some(TopRight)
        );
        assert_eq!(
            pick_corner(middle, pt(-1000.0, 650.0), screen),
            Some(TopLeft)
        );
        assert_eq!(
            pick_corner(middle, pt(-1000.0, -650.0), screen),
            Some(BottomLeft)
        );
        assert_eq!(
            pick_corner(middle, pt(1000.0, -650.0), screen),
            Some(BottomRight)
        );
        assert_eq!(pick_corner(middle, pt(0.0, 0.0), screen), None);
        let up = pick_corner(middle, pt(0.0, 1000.0), screen);
        assert!(up == Some(TopLeft) || up == Some(TopRight));
        let right = pick_corner(middle, pt(1000.0, 0.0), screen);
        assert!(right == Some(TopRight) || right == Some(BottomRight));
        assert_eq!(
            pick_corner(pt(1300.0, 200.0), pt(1000.0, 0.0), screen),
            Some(BottomRight)
        );
        assert_eq!(
            pick_corner(pt(1300.0, 800.0), pt(1000.0, 0.0), screen),
            Some(TopRight)
        );
        assert_eq!(
            pick_corner(pt(200.0, 200.0), pt(1000.0, 0.0), screen),
            Some(BottomRight)
        );
        assert_eq!(
            pick_corner(pt(1400.0, 100.0), pt(0.0, 1000.0), screen),
            Some(TopRight)
        );
        assert_eq!(
            pick_corner(pt(-100.0, 491.0), pt(1000.0, 900.0), screen),
            Some(TopRight)
        );
    }

    #[test]
    fn angle_apart_wraps_around() {
        assert!(angle_apart(0.1, 2.0 * PI - 0.1) < 0.2 + 1e-12);
        assert!(near(angle_apart(-PI, PI), 0.0));
        assert!(near(angle_apart(PI / 2.0, -PI / 2.0), PI));
    }

    #[test]
    fn corner_frame_insets_and_rounds() {
        let visible = rect(0.0, 0.0, 1512.0, 945.0);
        let mut r = corner_frame(TopRight, 468.0, 263.0, visible);
        assert!(
            r.x == 1512.0 - 10.0 - 468.0
                && r.y == 945.0 - 10.0 - 263.0
                && r.w == 468.0
                && r.h == 263.0
        );
        r = corner_frame(BottomLeft, 468.0, 263.0, visible);
        assert!(r.x == 10.0 && r.y == 10.0);
        r = corner_frame(TopLeft, 300.4, 169.0, rect(1512.0, -200.0, 1920.0, 1055.0));
        assert!(r.x == 1522.0 && r.y == -200.0 + 1055.0 - 10.0 - 169.0);
        r = corner_frame(
            BottomRight,
            300.0,
            169.0,
            rect(1512.0, -200.0, 1920.0, 1055.0),
        );
        assert!(r.x == 1512.0 + 1920.0 - 10.0 - 300.0 && r.y == -190.0);
    }

    #[test]
    fn snapped_corner_picks_the_card_radius() {
        assert_eq!(SNAPPED_RADIUS, 9.0);
        assert_eq!(FREE_RADIUS, 6.0);
        assert!(near(
            CORNER_INSET + SNAPPED_RADIUS,
            BROWSER_CARD_INSET + BROWSER_CARD_RADIUS
        ));
        let zoomed = rect(0.0, 0.0, 1512.0, 949.0);
        let beside = rect(1512.0, -200.0, 1920.0, 1055.0);
        for visible in [zoomed, beside] {
            for corner in [TopLeft, TopRight, BottomLeft, BottomRight] {
                for w in [336.0, 300.4, 468.0] {
                    let at = corner_frame(corner, w, 189.0, visible);
                    assert_eq!(snapped_corner(at, visible), Some(corner));
                    assert_eq!(corner_radius(at, visible), SNAPPED_RADIUS);
                    assert_eq!(corner_radius(shifted(at, 1.0, 0.0), visible), FREE_RADIUS);
                    assert_eq!(corner_radius(shifted(at, 0.0, -1.0), visible), FREE_RADIUS);
                }
            }
        }
        assert_eq!(
            snapped_corner(rect(1157.0, 19.0, 336.0, 189.0), zoomed),
            None
        );
        assert_eq!(
            corner_radius(rect(1157.0, 19.0, 336.0, 189.0), zoomed),
            FREE_RADIUS
        );
        assert_eq!(
            snapped_corner(rect(1166.0, 10.0, 336.0, 189.0), zoomed),
            Some(BottomRight)
        );
        assert_eq!(
            snapped_corner(rect(1166.0, 10.0, 336.0, 189.0), beside),
            None
        );
        assert_eq!(
            snapped_corner(rect(1166.4, 10.0, 335.6, 189.0), zoomed),
            Some(BottomRight)
        );

        let docked = rect(0.0, 70.0, 1512.0, 879.0);
        let dock_hidden = rect(0.0, 0.0, 1512.0, 949.0);
        let menu_bar_hidden = rect(0.0, 70.0, 1512.0, 911.0);
        let thrown_bottom = corner_frame(BottomRight, 336.0, 189.0, docked);
        let thrown_top = corner_frame(TopLeft, 336.0, 189.0, docked);
        assert_eq!(corner_radius(thrown_bottom, docked), SNAPPED_RADIUS);
        assert_eq!(corner_radius(thrown_bottom, dock_hidden), FREE_RADIUS);
        assert_eq!(corner_radius(thrown_top, dock_hidden), SNAPPED_RADIUS);
        assert_eq!(corner_radius(thrown_top, menu_bar_hidden), FREE_RADIUS);
        assert_eq!(
            corner_radius(thrown_bottom, menu_bar_hidden),
            SNAPPED_RADIUS
        );
        assert_eq!(
            corner_radius(
                corner_frame(BottomRight, 336.0, 189.0, dock_hidden),
                dock_hidden
            ),
            SNAPPED_RADIUS
        );
    }

    #[test]
    fn finger_delta_and_fling_thresholds() {
        assert!(finger_delta(5.0, 3.0, true).x == 5.0 && finger_delta(5.0, 3.0, true).y == -3.0);
        assert!(finger_delta(5.0, 3.0, false).x == -5.0 && finger_delta(5.0, 3.0, false).y == 3.0);
        assert!(!swipe_surely_flings(pt(-7.0, 0.0)) && swipe_surely_flings(pt(-10.0, 0.0)));
        assert!(
            swipe_velocity(pt(10.0, -2.0)).x == 900.0 && swipe_velocity(pt(10.0, -2.0)).y == -180.0
        );
        assert!(!drag_flings(pt(350.0, 0.0)) && drag_flings(pt(300.0, 200.0)));
    }

    #[test]
    fn release_velocity_uses_the_recent_window() {
        let s = [
            sample(0.00, 0.0, 0.0),
            sample(0.02, 10.0, 0.0),
            sample(0.10, 50.0, 0.0),
            sample(0.15, 100.0, 20.0),
            sample(0.18, 130.0, 26.0),
        ];
        let mut v = release_velocity(&s, 0.19);
        assert!(near_by(v.x, (130.0 - 50.0) / 0.08, 1e-6) && near_by(v.y, 26.0 / 0.08, 1e-6));
        v = release_velocity(&s, 0.30);
        assert!(v.x == 0.0 && v.y == 0.0);
        v = release_velocity(&s[..1], 0.01);
        assert!(v.x == 0.0 && v.y == 0.0);
    }

    #[test]
    fn release_velocity_of_a_held_drag_is_zero() {
        let drag = [
            sample(1.000, 0.0, 0.0),
            sample(1.008, 8.0, 0.0),
            sample(1.016, 16.0, 0.0),
            sample(1.024, 24.0, 0.0),
            sample(1.032, 32.0, 0.0),
            sample(1.040, 40.0, 0.0),
        ];
        let held = release_velocity(&drag, 1.100);
        assert!(held.x == 0.0 && held.y == 0.0);
        let flicked = release_velocity(&drag, 1.044);
        assert!(near_by(flicked.x, 1000.0, 1e-6) && flicked.y == 0.0 && drag_flings(flicked));
    }

    #[test]
    fn swipe_crosses_to_an_offset_display_without_a_jump() {
        let offset = [
            screen(rect(0.0, 0.0, 1512.0, 982.0), rect(0.0, 0.0, 1512.0, 945.0)),
            screen(
                rect(1000.0, 982.0, 1512.0, 982.0),
                rect(1000.0, 982.0, 1512.0, 945.0),
            ),
        ];
        let mut w = rect(700.0, 600.0, 468.0, 263.0);
        let mut biggest = 0.0f64;
        for _ in 0..1064 {
            let next = swipe_step(w, pt(0.0, 1.0), &offset, None);
            biggest = biggest.max((next.y - w.y - 1.0).abs());
            w = next;
        }
        assert!(biggest == 0.0 && w.y == 982.0 + 945.0 - 263.0);
        assert_eq!(largest_overlap(w, &offset), 1);
        assert_eq!(swipe_step(w, pt(0.0, 1.0), &offset, None).y, w.y);
        assert_eq!(swipe_step(w, pt(0.0, -1.0), &offset, None).y, w.y - 1.0);
        let mut beside = rect(100.0, 600.0, 468.0, 263.0);
        for _ in 0..600 {
            beside = swipe_step(beside, pt(0.0, 1.0), &offset, None);
        }
        assert_eq!(beside.y, 945.0 - 263.0);

        let single = [screen(
            rect(0.0, 0.0, 1512.0, 982.0),
            rect(0.0, 0.0, 1512.0, 945.0),
        )];
        let mut top = rect(500.0, 600.0, 468.0, 263.0);
        for _ in 0..300 {
            top = swipe_step(top, pt(0.0, 2.0), &single, None);
        }
        assert_eq!(top.y, 945.0 - 263.0);
        top = swipe_step(top, pt(0.0, -5.0), &single, None);
        assert_eq!(top.y, 945.0 - 263.0 - 5.0);
        let mut above = rect(500.0, 800.0, 468.0, 263.0);
        above = swipe_step(above, pt(3.0, 4.0), &single, None);
        assert!(above.y == 800.0 && above.x == 503.0);
        above = swipe_step(above, pt(0.0, -4.0), &single, None);
        assert_eq!(above.y, 796.0);
    }

    #[test]
    fn swipe_stops_at_the_dock_and_crosses_to_a_stacked_display() {
        let single = [screen(
            rect(0.0, 0.0, 1512.0, 982.0),
            rect(0.0, 70.0, 1512.0, 875.0),
        )];
        let mut w = rect(500.0, 100.0, 468.0, 263.0);
        for _ in 0..100 {
            w = swipe_step(w, pt(0.0, -5.0), &single, None);
        }
        assert!(w.y == 70.0 && w.x == 500.0);
        let mut low = rect(500.0, 20.0, 468.0, 263.0);
        low = swipe_step(low, pt(0.0, -5.0), &single, None);
        assert_eq!(low.y, 20.0);
        low = swipe_step(low, pt(0.0, 5.0), &single, None);
        assert_eq!(low.y, 25.0);

        let stacked = [
            screen(rect(0.0, 0.0, 1512.0, 982.0), rect(0.0, 0.0, 1512.0, 945.0)),
            screen(
                rect(-200.0, -1080.0, 1920.0, 1080.0),
                rect(-200.0, -1080.0, 1920.0, 1055.0),
            ),
        ];
        let mut s = rect(500.0, 100.0, 468.0, 263.0);
        let mut biggest = 0.0f64;
        for _ in 0..400 {
            let next = swipe_step(s, pt(0.0, -5.0), &stacked, None);
            biggest = biggest.max((next.y - s.y + 5.0).abs());
            s = next;
            if s.y == -1080.0 {
                break;
            }
        }
        assert!(s.y == -1080.0 && biggest == 0.0);
        assert_eq!(swipe_step(s, pt(0.0, -5.0), &stacked, None).y, -1080.0);
        let narrow_below = [
            screen(rect(0.0, 0.0, 1512.0, 982.0), rect(0.0, 0.0, 1512.0, 945.0)),
            screen(
                rect(0.0, -1080.0, 1200.0, 1080.0),
                rect(0.0, -1080.0, 1200.0, 1055.0),
            ),
        ];
        let mut off = rect(1250.0, 100.0, 250.0, 141.0);
        for _ in 0..100 {
            off = swipe_step(off, pt(0.0, -5.0), &narrow_below, None);
        }
        assert_eq!(off.y, 0.0);
    }

    #[test]
    fn drag_crosses_to_an_offset_display_and_keeps_a_strip_visible() {
        let offset = [
            screen(rect(0.0, 0.0, 1512.0, 982.0), rect(0.0, 0.0, 1512.0, 945.0)),
            screen(
                rect(1000.0, 982.0, 1512.0, 982.0),
                rect(1000.0, 982.0, 1512.0, 945.0),
            ),
        ];
        let mut w = rect(700.0, 600.0, 468.0, 263.0);
        let mut biggest = 0.0f64;
        for _pointer in 0..1064 {
            let next = drag_step(w, pt(0.0, 1.0), &offset, None);
            biggest = biggest.max((next.y - w.y - 1.0).abs());
            w = next;
        }
        assert!(biggest == 0.0 && w.y == 982.0 + 945.0 - 263.0);
        let mut beside = rect(100.0, 600.0, 468.0, 263.0);
        for _pointer in 0..600 {
            beside = drag_step(beside, pt(0.0, 1.0), &offset, None);
        }
        assert_eq!(beside.y, 945.0 - 263.0);
        beside = drag_step(beside, pt(0.0, 500.0 - beside.y), &offset, None);
        assert_eq!(beside.y, 500.0);
        let under = drag_step(
            rect(100.0, 100.0, 468.0, 263.0),
            pt(0.0, -300.0),
            &offset,
            None,
        );
        assert_eq!(under.y, 0.0);

        let mut lost = rect(1100.0, 1300.0, 468.0, 263.0);
        for _ in 0..600 {
            lost = swipe_step(lost, pt(-1.0, 0.0), &offset, None);
        }
        assert!(lost.x == 1000.0 + MIN_VISIBLE_WIDTH - 468.0 && lost.y == 1300.0);
        let mut lower = rect(1000.0, 100.0, 468.0, 263.0);
        for _ in 0..600 {
            lower = swipe_step(lower, pt(1.0, 0.0), &offset, None);
        }
        assert!(lower.x == 1512.0 - MIN_VISIBLE_WIDTH && lower.y == 100.0);
        let dragged = drag_step(
            rect(1100.0, 1300.0, 468.0, 263.0),
            pt(-700.0, 0.0),
            &offset,
            None,
        );
        assert_eq!(dragged.x, 1000.0 + MIN_VISIBLE_WIDTH - 468.0);

        let mut crossing = rect(100.0, 600.0, 468.0, 263.0);
        for _ in 0..281 {
            crossing = drag_step(crossing, pt(0.0, 1.0), &offset, None);
        }
        assert_eq!(crossing.y, 945.0 - 263.0);
        for _ in 0..432 {
            crossing = drag_step(crossing, pt(1.0, 0.0), &offset, None);
        }
        assert!(crossing.x == 532.0 && crossing.y == 945.0 - 263.0);
        crossing = drag_step(crossing, pt(1.0, 0.0), &offset, None);
        assert!(crossing.x == 533.0 && crossing.y == 945.0 - 263.0);
        crossing = drag_step(crossing, pt(0.0, 1.0), &offset, None);
        assert_eq!(crossing.y, 945.0 - 263.0 + 1.0);
    }

    #[test]
    fn oversized_window_stays_boxed_in_its_display() {
        let small = [screen(
            rect(0.0, 0.0, 1280.0, 720.0),
            rect(0.0, 0.0, 1280.0, 683.0),
        )];
        let at = |y: f64, dy: f64| {
            swipe_step(rect(100.0, y, 1000.0, 720.0), pt(0.0, dy), &small, None).y
        };
        assert!(at(0.0, 5.0) == 0.0 && at(0.0, -5.0) == -5.0);
        assert!(at(-37.0, -5.0) == -37.0 && at(-37.0, 5.0) == -32.0);
        assert!(at(-20.0, 5.0) == -15.0 && at(-20.0, -5.0) == -25.0);
        assert_eq!(
            swipe_step(
                rect(100.0, -20.0, 1000.0, 720.0),
                pt(30.0, 0.0),
                &small,
                None
            )
            .x,
            130.0
        );
        assert_eq!(
            swipe_step(
                rect(100.0, -20.0, 1000.0, 720.0),
                pt(300.0, 0.0),
                &small,
                None
            )
            .x,
            280.0
        );

        let tower = [
            screen(rect(0.0, 0.0, 1280.0, 720.0), rect(0.0, 0.0, 1280.0, 683.0)),
            screen(
                rect(0.0, 720.0, 1280.0, 720.0),
                rect(0.0, 720.0, 1280.0, 683.0),
            ),
        ];
        let mut tall = rect(100.0, 0.0, 1000.0, 720.0);
        for _ in 0..1000 {
            tall = swipe_step(tall, pt(0.0, 1.0), &tower, None);
        }
        assert_eq!(tall.y, 0.0);
        tall = swipe_step(tall, pt(0.0, -1.0), &tower, None);
        assert_eq!(tall.y, -1.0);
        for _ in 0..100 {
            tall = drag_step(tall, pt(0.0, -1.0), &tower, None);
        }
        assert_eq!(tall.y, -37.0);
        assert_eq!(swipe_step(tall, pt(0.0, 1.0), &tower, None).y, -36.0);
    }

    #[test]
    fn window_passes_between_an_offset_pair() {
        let pair = [
            screen(rect(0.0, 0.0, 1280.0, 720.0), rect(0.0, 0.0, 1280.0, 683.0)),
            screen(
                rect(1280.0, 500.0, 1280.0, 720.0),
                rect(1280.0, 500.0, 1280.0, 683.0),
            ),
        ];
        for drag in [false, true] {
            let step = |w: Rect, d: Point| {
                if drag {
                    drag_step(w, d, &pair, None)
                } else {
                    swipe_step(w, d, &pair, None)
                }
            };
            let mut w = rect(1000.0, 400.0, 468.0, 263.0);
            for _ in 0..800 {
                w = step(w, pt(1.0, 0.0));
            }
            assert!(w.x == 1279.0 && w.y == 400.0);
            for _ in 0..100 {
                w = step(w, pt(0.0, 1.0));
            }
            assert_eq!(w.y, 500.0);
            for _ in 0..521 {
                w = step(w, pt(1.0, 0.0));
            }
            assert!(w.x == 1800.0 && inside(w, pair[1].visible));
            for _ in 0..300 {
                w = step(w, pt(0.0, 1.0));
            }
            assert_eq!(w.y, 800.0);
            let mut u = rect(700.0, 300.0, 468.0, 263.0);
            for _ in 0..300 {
                u = step(u, pt(0.0, 1.0));
            }
            assert_eq!(u.y, 683.0 - 263.0);
            for _ in 0..800 {
                u = step(u, pt(1.0, 0.0));
            }
            assert!(u.x == 1279.0 && u.y == 683.0 - 263.0);
            let up = step(u, pt(0.0, 1.0));
            assert_eq!(up.y, 683.0 - 263.0 + 1.0);
        }
    }

    #[test]
    fn pick_corner_aims_from_the_grabbed_spot() {
        let one = [screen(
            rect(0.0, 0.0, 1280.0, 720.0),
            rect(0.0, 0.0, 1280.0, 683.0),
        )];
        let mut w = rect(100.0, 0.0, 700.0, 400.0);
        let grab = pt(100.0, 20.0);
        for _ in 0..630 {
            w = drag_step(w, pt(0.0, 1.0), &one, None);
        }
        assert_eq!(w.y, 683.0 - 400.0);
        for _ in 0..100 {
            w = drag_step(w, pt(1.0, 0.0), &one, None);
        }
        assert!(w.x == 200.0 && w.y == 283.0);
        let spot = pt(w.x + grab.x, w.y + grab.y);
        let cursor = pt(300.0, 650.0);
        assert!(spot.x == 300.0 && spot.y == 303.0);
        assert_eq!(
            pick_corner(spot, pt(2000.0, 0.0), one[0].frame),
            Some(BottomRight)
        );
        assert_eq!(
            pick_corner(cursor, pt(2000.0, 0.0), one[0].frame),
            Some(TopRight)
        );
    }

    #[test]
    fn fractional_swipe_steps_are_kept() {
        let one = [screen(
            rect(0.0, 0.0, 1280.0, 720.0),
            rect(0.0, 0.0, 1280.0, 683.0),
        )];
        let mut w = rect(100.0, 100.0, 468.0, 263.0);
        w = swipe_step(w, pt(500.0, 0.0), &one, None);
        assert_eq!(w.x, 600.0);
        w = swipe_step(w, pt(0.0, 500.0), &one, None);
        assert_eq!(w.y, 683.0 - 263.0);
        w = swipe_step(w, pt(0.4, -0.3), &one, None);
        assert!(near(w.x, 600.4) && near(w.y, 683.0 - 263.0 - 0.3));
        w = swipe_step(w, pt(0.0, 0.8), &one, None);
        assert!(near(w.y, 683.0 - 263.0 - 0.3));
        w = swipe_step(w, pt(-0.4, 0.3), &one, None);
        assert!(near(w.x, 600.0) && near(w.y, 683.0 - 263.0));
    }

    #[test]
    fn rescue_brings_a_stashed_window_back_on_a_wide_display() {
        let wide = [
            screen(
                rect(0.0, 0.0, 3440.0, 1440.0),
                rect(0.0, 0.0, 3440.0, 1403.0),
            ),
            screen(
                rect(2000.0, 1440.0, 1280.0, 800.0),
                rect(2000.0, 1440.0, 1280.0, 763.0),
            ),
        ];
        let mut w = rect(760.0, 400.0, 1300.0, 650.0);
        for _ in 0..1040 {
            w = swipe_step(w, pt(0.0, 1.0), &wide, Some(0));
        }
        assert!(w.y == 1440.0 && reachable(w, &wide));
        let stashed = rect(728.0, 1440.0, 1300.0, 650.0);
        assert!(!reachable(stashed, &wide));
        let mut lost = stashed;
        for _ in 0..800 {
            lost = swipe_step(lost, pt(1.0, 1.0), &wide, Some(1));
        }
        assert!(lost.y <= 2203.0 - 650.0 && reachable(lost, &wide));
        let far = rect(1528.0, 2240.0, 1300.0, 650.0);
        assert!(reachable(rescue(far, &wide, Some(1)), &wide));
        assert!(
            rescue(far, &wide, Some(1)).x == 1980.0
                && rescue(far, &wide, Some(1)).y == 2203.0 - 650.0
        );
        assert!(rescue(w, &wide, Some(0)).x == w.x && rescue(w, &wide, Some(0)).y == w.y);
    }

    #[test]
    fn window_round_trips_between_adjacent_displays() {
        let primary = screen(rect(0.0, 0.0, 1512.0, 982.0), rect(0.0, 0.0, 1512.0, 945.0));
        let beside = [
            primary,
            screen(
                rect(1512.0, 0.0, 1920.0, 1080.0),
                rect(1512.0, 0.0, 1920.0, 1055.0),
            ),
        ];
        let above = [
            primary,
            screen(
                rect(0.0, 982.0, 1920.0, 1080.0),
                rect(0.0, 982.0, 1920.0, 1055.0),
            ),
        ];
        let offset = [
            primary,
            screen(
                rect(1512.0, -200.0, 1920.0, 1080.0),
                rect(1512.0, -200.0, 1920.0, 1055.0),
            ),
        ];
        for drag in [false, true] {
            let step = |w: Rect, d: Point, layout: &[Screen]| {
                if drag {
                    drag_step(w, d, layout, None)
                } else {
                    swipe_step(w, d, layout, None)
                }
            };
            let mut w = rect(1000.0, 400.0, 468.0, 263.0);
            for _ in 0..1200 {
                w = step(w, pt(1.0, 0.0), &beside);
            }
            assert!(w.x == 2200.0 && w.y == 400.0 && inside(w, beside[1].visible));
            for _ in 0..1200 {
                w = step(w, pt(-1.0, 0.0), &beside);
            }
            assert!(w.x == 1000.0 && w.y == 400.0 && inside(w, beside[0].visible));

            let mut up = rect(500.0, 600.0, 468.0, 263.0);
            for _ in 0..700 {
                up = step(up, pt(0.0, 1.0), &above);
            }
            assert!(up.x == 500.0 && up.y == 1300.0 && inside(up, above[1].visible));
            for _ in 0..700 {
                up = step(up, pt(0.0, -1.0), &above);
            }
            assert!(up.x == 500.0 && up.y == 600.0 && inside(up, above[0].visible));

            let mut level = rect(1000.0, 400.0, 468.0, 263.0);
            for _ in 0..1200 {
                level = step(level, pt(1.0, 0.0), &offset);
            }
            assert!(level.x == 2200.0 && level.y == 400.0 && inside(level, offset[1].visible));

            let mut high = rect(1000.0, 945.0 - 263.0, 468.0, 263.0);
            for _ in 0..1200 {
                high = step(high, pt(1.0, 0.0), &offset);
            }
            assert!(high.x == 1511.0 && high.y == 945.0 - 263.0);
            for _ in 0..100 {
                high = step(high, pt(0.0, -1.0), &offset);
            }
            for _ in 0..700 {
                high = step(high, pt(1.0, 0.0), &offset);
            }
            assert!(high.x > 1512.0 + 468.0 && inside(high, offset[1].visible));
        }

        let only = [primary];
        for (layout, on_second) in [
            (&beside, rect(2200.0, 400.0, 468.0, 263.0)),
            (&above, rect(500.0, 1300.0, 468.0, 263.0)),
            (&offset, rect(2200.0, 400.0, 468.0, 263.0)),
        ] {
            assert!(reachable(on_second, layout) && !reachable(on_second, &only));
            let rescued = rescue(on_second, &only, Some(0));
            assert!(reachable(rescued, &only) && inside(rescued, primary.visible));
        }
        let right = rect(2200.0, 400.0, 468.0, 263.0);
        assert!(
            rescue(right, &only, Some(0)).x == 1512.0 - 468.0
                && rescue(right, &only, Some(0)).y == 400.0
        );
        let high = rect(500.0, 1300.0, 468.0, 263.0);
        assert!(
            rescue(high, &only, Some(0)).x == 500.0
                && rescue(high, &only, Some(0)).y == 945.0 - 263.0
        );

        let docked = [
            primary,
            screen(
                rect(0.0, 982.0, 1512.0, 1080.0),
                rect(0.0, 1052.0, 1512.0, 985.0),
            ),
        ];
        for drag in [false, true] {
            let mut w = rect(500.0, 600.0, 468.0, 263.0);
            let mut outside = 0;
            for _ in 0..900 {
                w = if drag {
                    drag_step(w, pt(0.0, 1.0), &docked, None)
                } else {
                    swipe_step(w, pt(0.0, 1.0), &docked, None)
                };
                if shortfall(w, &docked) > 1e-9 {
                    outside += 1;
                }
            }
            assert!(outside == 0 && w.y == 1500.0 && inside(w, docked[1].visible));
        }
        let strip = rect(500.0, 840.0, 468.0, 263.0);
        assert!(shortfall(strip, &docked) <= 1e-9 && !reachable(strip, &docked));
        let edge = rect(500.0, 981.0, 468.0, 263.0);
        assert!(shortfall(edge, &only) > 0.0 && shortfall(edge, &above) <= 1e-9);
        assert!(
            shortfall(rect(100.0, 774.0, 400.0, 225.0), &only) > 0.0
                && shortfall(rect(1512.0 - 28.0, 400.0, 468.0, 263.0), &only) > 0.0
        );
    }

    #[test]
    fn spring_starts_at_rest_and_settles_with_little_overshoot() {
        let spring = |offset, velocity, t| spring_at(offset, velocity, t, RESPONSE, DAMPING_RATIO);
        let mut a = spring(500.0, 0.0, 0.0);
        assert!(near(a.offset, 500.0) && near(a.velocity, 0.0));
        a = spring(0.0, 1000.0, 0.0);
        assert!(near(a.offset, 0.0) && near(a.velocity, 1000.0));
        let dt = 1e-6;
        for t in [0.05, 0.2, 0.5] {
            let p = spring(-800.0, 2000.0, t);
            let q = spring(-800.0, 2000.0, t + dt);
            assert!(near_by((q.offset - p.offset) / dt, p.velocity, 0.05));
        }
        let mut settled = -1.0;
        for frame in 0..600 {
            let t = f64::from(frame) / 60.0;
            let (x, y) = (spring(-1000.0, 0.0, t), spring(0.0, 0.0, t));
            if resolved(x, y) {
                settled = t;
                break;
            }
        }
        assert!(settled > 1.5 && settled < 3.5);
        let mut overshoot = 0.0f64;
        for frame in 0..600 {
            overshoot = overshoot.max(spring(-1000.0, 0.0, f64::from(frame) / 120.0).offset);
        }
        assert!(overshoot > 0.0 && overshoot < 2.0);
    }

    #[test]
    fn drag_to_follows_the_pointer_where_drag_step_drifts() {
        let one = [screen(
            rect(0.0, 0.0, 1512.0, 982.0),
            rect(0.0, 0.0, 1512.0, 949.0),
        )];
        let grab = pt(186.5, 63.0);
        let placed = |r: Rect| rect(r.x.round(), r.y.round(), r.w, r.h);
        let mut stepped = rect(574.0, 386.0, 373.0, 210.0);
        let mut held = stepped;
        let mut pointer = pt(stepped.x + grab.x, stepped.y + grab.y);
        for _ in 0..600 {
            let next = pt(pointer.x - 0.5005, pointer.y + 0.3337);
            stepped = placed(drag_step(
                stepped,
                pt(next.x - pointer.x, next.y - pointer.y),
                &one,
                None,
            ));
            held = placed(drag_to(held, next, grab, &one, None));
            pointer = next;
        }
        assert!(
            (stepped.x + grab.x - pointer.x).abs() > 100.0
                || (stepped.y + grab.y - pointer.y).abs() > 100.0
        );
        assert!(
            (held.x + grab.x - pointer.x).abs() <= 0.5
                && (held.y + grab.y - pointer.y).abs() <= 0.5
        );
        assert!(!held_back(
            drag_to(held, pointer, grab, &one, None),
            pointer,
            grab
        ));

        let mut w = rect(574.0, 386.0, 373.0, 210.0);
        pointer = pt(w.x + grab.x, w.y + grab.y);
        let mut saw_held = false;
        for _ in 0..400 {
            pointer.x += 3.37;
            let next = drag_to(w, pointer, grab, &one, None);
            saw_held = saw_held || held_back(next, pointer, grab);
            w = placed(next);
        }
        assert!(saw_held && w.max_x() - 1512.0 > 0.0 && 1512.0 - w.x >= MIN_VISIBLE_WIDTH - 1e-9);
        for _ in 0..400 {
            pointer.x -= 3.37;
            w = placed(drag_to(w, pointer, grab, &one, None));
        }
        assert!((w.x + grab.x - pointer.x).abs() <= 0.5 && (w.y + grab.y - pointer.y).abs() <= 0.5);

        let mut top = rect(574.0, 700.0, 373.0, 210.0);
        pointer = pt(top.x + grab.x, top.y + grab.y);
        for i in 0..200 {
            top = placed(drag_to(
                top,
                pt(pointer.x, pointer.y + 2.25 * f64::from(i + 1)),
                grab,
                &one,
                None,
            ));
        }
        assert!(top.max_y() <= 949.0 + 1e-9);
        for i in (0..=200).rev() {
            top = placed(drag_to(
                top,
                pt(pointer.x, pointer.y + 2.25 * f64::from(i)),
                grab,
                &one,
                None,
            ));
        }
        assert!(near(top.y, 700.0) && near(top.x, 574.0));
    }

    #[test]
    fn property_random_walks_never_jump_stick_or_strand() {
        let mut r = Random { state: 20261022 };
        let (mut failed, mut steps, mut moves, mut reversals) = (0, 0, 0, 0);
        let (mut multi, mut oversized, mut invalid, mut crossings) = (0, 0, 0, 0);
        let (mut gestures, mut rescues, mut stranded_in_strip) = (0, 0, 0);
        for _c in 0..4000 {
            if failed >= 3 {
                break;
            }
            let mut all = [Screen::default(); 3];
            let half_points = r.chance(0.2);
            let n = layout(&mut r, &mut all, half_points);
            let screens = &all[..n];
            let home = r.integer(0, n as i32 - 1) as usize;
            let v = screens[home].visible;
            let mut w = f64::from(r.integer(284, 1400));
            let mut h = (w * r.uniform(0.4, 0.8)).round();
            if r.chance(0.15) {
                h = v.h + f64::from(r.integer(1, 120));
            }
            if r.chance(0.05) {
                w = v.w + f64::from(r.integer(1, 120));
            }
            let mut frame = rect(0.0, 0.0, w, h);
            if r.chance(0.8) {
                frame.x = if w <= v.w {
                    v.x + r.uniform(0.0, v.w - w)
                } else {
                    v.x + v.w - w + r.uniform(0.0, w - v.w)
                };
                frame.y = if h <= v.h {
                    v.y + r.uniform(0.0, v.h - h)
                } else {
                    v.y + v.h - h + r.uniform(0.0, h - v.h)
                };
            } else {
                frame.x = r.uniform(-2000.0, 4000.0);
                frame.y = r.uniform(-2000.0, 3000.0);
            }
            if r.chance(0.5) {
                frame.x = frame.x.round();
                frame.y = frame.y.round();
            }
            if r.chance(0.2) && w <= v.w && h <= v.h {
                match r.integer(0, 3) {
                    0 => frame.y = v.max_y() - h,
                    1 => frame.y = v.y,
                    2 => frame.x = v.max_x() - MIN_VISIBLE_WIDTH,
                    _ => frame.x = v.x + MIN_VISIBLE_WIDTH - w,
                }
            }
            let start = frame;
            multi += i32::from(n > 1);
            oversized += i32::from(w > v.w || h > v.h);
            invalid += i32::from(Mover::new(frame, screens, None).shortfall(frame) > 1e-9);
            let mut last_home = largest_overlap(frame, screens);
            let mut walk = Vec::new();
            let mut direction = pt(0.0, 0.0);
            let mut gesture = largest_overlap(frame, screens);
            let mut gesture_started_reachable = reachable(frame, screens);
            let target = if n > 1 && r.chance(0.5) {
                Some(r.integer(0, n as i32 - 1) as usize)
            } else {
                None
            };
            let mut broke = false;
            let mut end_gesture = |at: usize,
                                   walk: &[Point],
                                   frame: &mut Rect,
                                   gesture: &mut usize,
                                   started_reachable: &mut bool,
                                   failed: &mut i32| {
                gestures += 1;
                if *started_reachable && !reachable(*frame, screens) {
                    stranded_in_strip += 1;
                    if std::env::var_os("PIP_THROW_VERBOSE").is_some() {
                        describe(screens, start, walk, at, "left in a passage strip");
                    }
                }
                let rescued = rescue(*frame, screens, Some(*gesture));
                rescues += i32::from(rescued.x != frame.x || rescued.y != frame.y);
                *frame = rescued;
                if !reachable(*frame, screens) {
                    describe(
                        screens,
                        start,
                        walk,
                        at,
                        "unreachable after the gesture-end rescue",
                    );
                    *failed += 1;
                    return false;
                }
                *gesture = largest_overlap(*frame, screens);
                *started_reachable = true;
                true
            };
            for i in 0..250 {
                if r.chance(0.08)
                    && !end_gesture(
                        i,
                        &walk,
                        &mut frame,
                        &mut gesture,
                        &mut gesture_started_reachable,
                        &mut failed,
                    )
                {
                    broke = true;
                    break;
                }
                if i == 0 || r.chance(0.3) {
                    let mut angle = r.uniform(0.0, 2.0 * PI);
                    let size = if r.chance(0.03) {
                        500.0
                    } else if r.chance(0.1) {
                        r.uniform(0.05, 0.95)
                    } else {
                        r.uniform(1.0, 20.0)
                    };
                    if let Some(target) = target
                        && r.chance(0.7)
                    {
                        let t = screens[target].frame;
                        angle = (t.y + t.h / 2.0 - (frame.y + frame.h / 2.0))
                            .atan2(t.x + t.w / 2.0 - (frame.x + frame.w / 2.0))
                            + r.uniform(-0.6, 0.6);
                    }
                    direction = pt(size * angle.cos(), size * angle.sin());
                    if r.chance(0.3) {
                        direction = pt(direction.x.round(), direction.y.round());
                    }
                    if r.chance(0.2) {
                        direction = if r.chance(0.5) {
                            pt(direction.x, 0.0)
                        } else {
                            pt(0.0, direction.y)
                        };
                    }
                }
                walk.push(direction);
                let d = direction;
                let mover = Mover::new(frame, screens, Some(gesture));
                let before = mover.shortfall(frame);
                let next = if r.chance(0.5) {
                    swipe_step(frame, d, screens, Some(gesture))
                } else {
                    drag_step(frame, d, screens, Some(gesture))
                };
                steps += 1;
                let (mx, my) = (next.x - frame.x, next.y - frame.y);
                let jump_x =
                    mx.abs() > d.x.abs() + 1e-9 || (mx != 0.0 && (mx > 0.0) != (d.x > 0.0));
                let jump_y =
                    my.abs() > d.y.abs() + 1e-9 || (my != 0.0 && (my > 0.0) != (d.y > 0.0));
                if jump_x || jump_y || next.w != frame.w || next.h != frame.h {
                    describe(screens, start, &walk, i, "jump");
                    failed += 1;
                    break;
                }
                if !mover.allowed(next, frame) {
                    describe(screens, start, &walk, i, "reachability");
                    failed += 1;
                    break;
                }
                if mx != 0.0 || my != 0.0 {
                    moves += 1;
                    if before <= 1e-9 {
                        reversals += 1;
                        let back = swipe_step(next, pt(-d.x, -d.y), screens, Some(gesture));
                        if back.x == next.x && back.y == next.y {
                            describe(screens, start, &walk, i, "stuck on reversal");
                            println!(
                                "  from {{{},{}}} moved to {{{},{}}} by {{{},{}}}; shortfall there {}, reversed full {}",
                                frame.x,
                                frame.y,
                                next.x,
                                next.y,
                                d.x,
                                d.y,
                                mover.shortfall(next),
                                mover.shortfall(shifted(next, -d.x, -d.y))
                            );
                            failed += 1;
                            break;
                        }
                    }
                }
                frame = next;
                let now_home = largest_overlap(frame, screens);
                crossings += i32::from(now_home != last_home);
                last_home = now_home;
            }
            if !broke {
                end_gesture(
                    250,
                    &walk,
                    &mut frame,
                    &mut gesture,
                    &mut gesture_started_reachable,
                    &mut failed,
                );
            }
        }
        println!(
            "pip-throw property: {steps} steps, {moves} moves, {reversals} reversals checked, {multi} multi-display layouts, {oversized} oversized, {invalid} invalid starts, {crossings} display crossings, {gestures} gestures ended, {rescues} rescued ({stranded_in_strip} of them from a reachable start, ending in a passage strip), {failed} failures"
        );
        assert_eq!(failed, 0);
    }

    #[test]
    fn progress_greedy_walks_reach_every_touching_display() {
        let mut r = Random { state: 7311 };
        let (mut failed, mut walks, mut total_steps) = (0, 0, 0);
        let (mut tight, mut offset_pairs, mut half_point) = (0, 0, 0);
        for _c in 0..1200 {
            if failed >= 3 {
                break;
            }
            let mut all = [Screen::default(); 3];
            let mut n = 1;
            for _attempt in 0..20 {
                if n >= 2 {
                    break;
                }
                let half_points = r.chance(0.3);
                n = layout(&mut r, &mut all, half_points);
            }
            if n < 2 {
                continue;
            }
            let screens = &all[..n];
            let (mut fit_w, mut fit_h) = (f64::INFINITY, f64::INFINITY);
            for s in screens {
                fit_w = fit_w.min(s.visible.w);
                fit_h = fit_h.min(s.visible.h);
            }
            let w = f64::from(r.integer(284, 900)).min(fit_w);
            let h = (w * r.uniform(0.4, 0.8)).round().min(fit_h);
            let from = r.integer(0, n as i32 - 1) as usize;
            let v = screens[from].visible;
            for target in 0..n {
                if target == from {
                    continue;
                }
                let (a, b) = (screens[from].frame, screens[target].frame);
                let side_by_side = ((a.max_x() - b.x).abs() <= 1.0
                    || (b.max_x() - a.x).abs() <= 1.0)
                    && a.max_y().min(b.max_y()) > a.y.max(b.y);
                let stacked = ((a.max_y() - b.y).abs() <= 1.0 || (b.max_y() - a.y).abs() <= 1.0)
                    && a.max_x().min(b.max_x()) > a.x.max(b.x);
                if !side_by_side && !stacked {
                    continue;
                }
                let mut frame = rect(
                    v.x + r.uniform(0.0, v.w - w),
                    v.y + r.uniform(0.0, v.h - h),
                    w,
                    h,
                );
                let start = frame;
                let t = screens[target].visible;
                let mut waypoint = frame;
                offset_pairs += i32::from(if side_by_side {
                    (a.y - b.y).abs() > 1.0
                } else {
                    (a.x - b.x).abs() > 1.0
                });
                half_point +=
                    i32::from((b.x - b.x.round()).abs() > 0.1 || (a.x - a.x.round()).abs() > 0.1);
                if side_by_side {
                    let (lo, hi) = (v.y.max(t.y), v.max_y().min(t.max_y()));
                    tight += i32::from(hi - lo < h);
                    waypoint.y = std_clamp((lo + hi) / 2.0 - h / 2.0, v.y, v.max_y() - h);
                } else {
                    let (lo, hi) = (v.x.max(t.x), v.max_x().min(t.max_x()));
                    tight += i32::from(hi - lo < w);
                    waypoint.x = std_clamp((lo + hi) / 2.0 - w / 2.0, v.x, v.max_x() - w);
                }
                walks += 1;
                let mut arrived = false;
                let mut phase = 0;
                for step in 0..40000 {
                    if inside(frame, t) {
                        arrived = true;
                        break;
                    }
                    if phase == 0
                        && (frame.x - waypoint.x).abs() < 1e-9
                        && (frame.y - waypoint.y).abs() < 1e-9
                    {
                        phase = 1;
                    }
                    if phase == 1
                        && (if side_by_side {
                            frame.x >= t.x && frame.max_x() <= t.max_x()
                        } else {
                            frame.y >= t.y && frame.max_y() <= t.max_y()
                        })
                    {
                        phase = 2;
                    }
                    let mut dx = if phase == 0 {
                        waypoint.x - frame.x
                    } else {
                        t.x + t.w / 2.0 - (frame.x + w / 2.0)
                    };
                    let mut dy = if phase == 0 {
                        waypoint.y - frame.y
                    } else {
                        t.y + t.h / 2.0 - (frame.y + h / 2.0)
                    };
                    if phase == 1 {
                        if side_by_side {
                            dy = 0.0;
                        } else {
                            dx = 0.0;
                        }
                    }
                    let d = pt(
                        if dx > 0.0 { dx.min(1.0) } else { dx.max(-1.0) },
                        if dy > 0.0 { dy.min(1.0) } else { dy.max(-1.0) },
                    );
                    let drag = step % 2 == 1;
                    let go = move |delta| {
                        if drag {
                            drag_step(frame, delta, screens, None)
                        } else {
                            swipe_step(frame, delta, screens, None)
                        }
                    };
                    let mut next = go(d);
                    if next.x == frame.x && next.y == frame.y {
                        next = go(pt(d.x, 0.0));
                    }
                    if next.x == frame.x && next.y == frame.y {
                        next = go(pt(0.0, d.y));
                    }
                    total_steps += 1;
                    if next.x == frame.x && next.y == frame.y {
                        if phase < 2 {
                            phase += 1;
                            continue;
                        }
                        break;
                    }
                    frame = next;
                }
                if !arrived {
                    failed += 1;
                    println!("PROGRESS FAIL: from screen {from} to {target}, stuck at {frame:?}");
                    print_screens(screens);
                    println!("  start {start:?}");
                }
            }
        }
        println!(
            "pip-throw progress: {walks} greedy walks between touching displays ({offset_pairs} offset, {tight} with a shared edge shorter than the window, {half_point} at half-point origins), {total_steps} steps, {failed} failures"
        );
        assert_eq!(failed, 0);
    }

    #[test]
    fn tuck_flick_and_corner_pull_thresholds() {
        let visible = rect(0.0, 25.0, 1512.0, 920.0);
        assert!(!pulled_past_half(
            rect(1512.0 - 480.0 - 200.0, 600.0, 480.0, 270.0),
            visible,
            true
        ));
        let docked = corner_frame(BottomRight, 480.0, 270.0, visible);
        assert!(
            flick_landing(docked, pt(900.0, 0.0)).x == docked.x + 90.0
                && flick_landing(docked, pt(900.0, 0.0)).y == docked.y
        );
        assert_eq!(flick_landing(docked, pt(900.0, 600.0)).x, docked.x);
        assert_eq!(
            flick_landing(docked, pt(-2000.0, 300.0)).x,
            docked.x - 200.0
        );
        assert!(
            !past_edge_enough_to_tuck(384.0 * 0.3, 384.0)
                && !past_edge_enough_to_tuck(384.0 * 0.5, 384.0)
                && past_edge_enough_to_tuck(384.0 * 0.5 + 1.0, 384.0)
        );
        assert!(
            !pulled_out_of_tuck(20.0)
                && !pulled_out_of_tuck(47.5)
                && pulled_out_of_tuck(48.0)
                && pulled_out_of_tuck(300.0)
        );
        let near_dock = |dx: f64, dy: f64| rect(docked.x + dx, docked.y + dy, 480.0, 270.0);
        assert_eq!(
            corner_in_reach(near_dock(-40.0, 50.0), visible),
            Some(BottomRight)
        );
        assert_eq!(
            corner_in_reach(near_dock(29.0, 61.0), visible),
            Some(BottomRight)
        );
        assert_eq!(corner_in_reach(near_dock(-70.0, 60.0), visible), None);
        assert!(
            corner_in_reach(docked, visible) == Some(BottomRight)
                && corner_in_reach(rect(600.0, 400.0, 480.0, 270.0), visible).is_none()
        );
        let middle = rect(600.0, 400.0, 480.0, 270.0);
        assert_eq!(
            corner_pulled_into(middle, near_dock(-40.0, 50.0), visible),
            Some(BottomRight)
        );
        assert_eq!(
            corner_pulled_into(near_dock(-12.0, 12.0), near_dock(-42.0, 32.0), visible),
            None
        );
        assert_eq!(
            corner_pulled_into(docked, near_dock(-30.0, 20.0), visible),
            None
        );
        assert_eq!(
            corner_pulled_into(middle, rect(700.0, 400.0, 480.0, 270.0), visible),
            None
        );
        assert_eq!(
            corner_pulled_into(
                rect(10.0, 600.0, 480.0, 270.0),
                near_dock(-20.0, 20.0),
                visible
            ),
            Some(BottomRight)
        );
        assert!(
            !swipe_surely_flings(pt(6.0, 1.0))
                && !swipe_surely_flings(pt(7.0, 6.0))
                && swipe_surely_flings(pt(7.0, 9.0))
                && swipe_surely_flings(pt(0.0, -10.0))
                && swipe_surely_flings(pt(124.0, 4.0))
        );
        let whole = rect(0.0, 0.0, 1512.0, 982.0);
        assert_eq!(
            pick_corner(pt(700.0, 500.0), pt(2700.0, 4410.0), whole),
            Some(TopRight)
        );
        assert_eq!(
            pick_corner(pt(700.0, 500.0), pt(0.0, 1000.0), whole),
            Some(TopLeft)
        );
        assert!(
            pick_corner(pt(900.0, 500.0), pt(0.0, 1000.0), whole) == Some(TopRight)
                && pick_corner(pt(900.0, 500.0), pt(0.0, -1000.0), whole) == Some(BottomRight)
        );
        assert!(
            pick_corner(pt(700.0, 500.0), pt(1000.0, 0.0), whole).is_some()
                && pick_corner(pt(700.0, 500.0), pt(0.0, 0.0), whole).is_none()
        );
        assert!(pulled_past_half(
            rect(300.0, 600.0, 480.0, 270.0),
            visible,
            true
        ));
        assert!(!pulled_past_half(
            rect(1512.0 / 2.0 - 240.0 + 1.0, 600.0, 480.0, 270.0),
            visible,
            true
        ));
        assert!(pulled_past_half(
            rect(1512.0 / 2.0 - 240.0 - 1.0, 600.0, 480.0, 270.0),
            visible,
            true
        ));
        assert!(!pulled_past_half(
            rect(200.0, 600.0, 480.0, 270.0),
            visible,
            false
        ));
        assert!(pulled_past_half(
            rect(900.0, 600.0, 480.0, 270.0),
            visible,
            false
        ));
    }
}
