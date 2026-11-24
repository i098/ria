// Ported from ardia packages/ardiacore/ios/ArdiaPiPZoomMath.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use super::ardia_pip_throw_math::{Point, Rect, TUCK_PAST_EDGE, std_clamp};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

pub const PER_POINT: f64 = 0.004;
pub const PER_LINE: f64 = 0.08;
pub const DOCK_SLOP: f64 = 64.0;
pub const MOST_PAST_EDGE: f64 = TUCK_PAST_EDGE;
pub const PINCH_FOLLOW: f64 = 0.05;
pub const HAND_OFF_LINGER: f64 = 0.25;
pub const HAND_OFF_TIMEOUT: f64 = 0.6;
pub const HITTABLE_HIDDEN_ALPHA: f64 = 0.01;

pub fn scroll_factor(delta_y: f64, precise: bool) -> f64 {
    ((if precise { PER_POINT } else { PER_LINE }) * delta_y).exp()
}

pub fn magnify_factor(magnification: f64) -> f64 {
    0.05f64.max(1.0 + magnification)
}

/// Returns `(low, high)`.
pub fn width_limits(aspect: f64, min_size: Size, max_size: Size, visible: Rect) -> (f64, f64) {
    let low = min_size.w.max(min_size.h * aspect).max(1.0);
    let high = max_size
        .w
        .min(max_size.h * aspect)
        .min(visible.w)
        .min(visible.h * aspect);
    (if low > high { high } else { low }, high)
}

pub fn anchor(origin: f64, size: f64, next: f64, pointer: f64, low: f64, high: f64) -> f64 {
    let before = origin - low;
    let after = high - (origin + size);
    if before >= 0.0 && after >= 0.0 && before.min(after) <= DOCK_SLOP {
        return if before <= after {
            origin
        } else {
            origin + size - next
        };
    }
    let fraction = std_clamp((pointer - origin) / size, 0.0, 1.0);
    pointer - fraction * next
}

pub fn corner_growth(by_x: f64, by_y: f64, aspect: f64) -> f64 {
    let squared = aspect * aspect;
    (squared * by_x + by_y) / (squared + 1.0)
}

pub fn kept_within(value: f64, next: f64, origin: f64, size: f64, low: f64, high: f64) -> f64 {
    let most = (next * MOST_PAST_EDGE).floor();
    std_clamp(
        value,
        low.min(origin).max(low - most),
        high.max(origin + size).min(high + most) - next,
    )
}

pub fn finite(r: Rect) -> bool {
    r.x.is_finite()
        && r.y.is_finite()
        && r.w.is_finite()
        && r.h.is_finite()
        && r.w > 0.0
        && r.h > 0.0
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ZoomResult {
    pub frame: Rect,
    pub width: f64,
}

#[allow(clippy::too_many_arguments)]
pub fn zoom(
    frame: Rect,
    width: f64,
    aspect: f64,
    factor: f64,
    pointer: Point,
    visible: Rect,
    min_size: Size,
    max_size: Size,
) -> ZoomResult {
    let (low, high) = width_limits(aspect, min_size, max_size, visible);
    let w = std_clamp(width * factor, low, high);
    let rw = std_clamp(w.round(), low.ceil(), low.ceil().max(high.floor()));
    let rh = (rw / aspect).round();
    let x = anchor(frame.x, frame.w, rw, pointer.x, visible.x, visible.max_x());
    let y = anchor(frame.y, frame.h, rh, pointer.y, visible.y, visible.max_y());
    let x = kept_within(x, rw, frame.x, frame.w, visible.x, visible.max_x());
    let y = kept_within(y, rh, frame.y, frame.h, visible.y, visible.max_y());
    ZoomResult {
        frame: Rect {
            x: x.round(),
            y: y.round(),
            w: rw,
            h: rh,
        },
        width: w,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VISIBLE: Rect = rect(0.0, 0.0, 1512.0, 945.0);
    const MIN_SIZE: Size = Size { w: 284.0, h: 160.0 };
    const MAX_SIZE: Size = Size {
        w: 1512.0 * 0.8,
        h: 945.0 * 0.8,
    };
    const ASPECT: f64 = 16.0 / 9.0;

    const fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    const fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    fn near(a: f64, b: f64) -> bool {
        near_by(a, b, 0.51)
    }

    fn near_by(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn zoom_on_main(frame: Rect, width: f64, factor: f64, pointer: Point) -> ZoomResult {
        zoom(
            frame, width, ASPECT, factor, pointer, VISIBLE, MIN_SIZE, MAX_SIZE,
        )
    }

    #[test]
    fn scroll_and_magnify_factors() {
        assert!(scroll_factor(10.0, true) > 1.0 && scroll_factor(-10.0, true) < 1.0);
        assert!(near_by(
            scroll_factor(25.0, true) * scroll_factor(-25.0, true),
            1.0,
            1e-12
        ));
        assert!(near_by(scroll_factor(150.0, true), 1.82, 0.01));
        assert!(near_by(scroll_factor(1.0, false), 1.083, 0.001));
        assert_eq!(scroll_factor(0.0, true), 1.0);
        assert!(
            magnify_factor(0.1) > 1.0 && magnify_factor(-0.1) < 1.0 && magnify_factor(-5.0) > 0.0
        );
    }

    #[test]
    fn zoom_keeps_the_docked_top_right_corner() {
        let frame = rect(1512.0 - 16.0 - 480.0, 16.0, 480.0, 270.0);
        let r = zoom_on_main(frame, 480.0, 1.25, pt(1300.0, 100.0));
        assert!(r.frame.w == 600.0 && near(r.frame.h, 337.5));
        assert!(near(r.frame.max_x(), 1512.0 - 16.0) && r.frame.y == 16.0);
        assert!(near_by(r.frame.w / r.frame.h, ASPECT, 0.01));
    }

    #[test]
    fn zoom_keeps_the_docked_bottom_left_corner() {
        let frame = rect(20.0, 945.0 - 20.0 - 270.0, 480.0, 270.0);
        let r = zoom_on_main(frame, 480.0, 0.8, pt(100.0, 800.0));
        assert!(r.frame.x == 20.0 && near(r.frame.max_y(), 945.0 - 20.0) && r.frame.w == 384.0);
    }

    #[test]
    fn zoom_keeps_the_point_under_the_pointer() {
        let frame = rect(500.0, 300.0, 480.0, 270.0);
        let pointer = pt(500.0 + 480.0 * 0.25, 300.0 + 270.0 * 0.75);
        let r = zoom_on_main(frame, 480.0, 1.5, pointer);
        assert!(near_by((pointer.x - r.frame.x) / r.frame.w, 0.25, 0.01));
        assert!(near_by((pointer.y - r.frame.y) / r.frame.h, 0.75, 0.01));
    }

    #[test]
    fn zoom_keeps_a_docked_left_edge() {
        let frame = rect(10.0, 300.0, 480.0, 270.0);
        let r = zoom_on_main(frame, 480.0, 1.2, pt(200.0, 435.0));
        assert!(r.frame.x == 10.0 && near_by(r.frame.y + r.frame.h / 2.0, 435.0, 1.0));
    }

    #[test]
    fn zoom_clamps_to_the_size_limits() {
        let frame = rect(1000.0, 16.0, 480.0, 270.0);
        let pointer = pt(1200.0, 100.0);
        let small = zoom_on_main(frame, 480.0, 0.01, pointer);
        assert!(
            small.frame.w == 285.0
                && small.frame.h == 160.0
                && near_by(small.width, 160.0 * ASPECT, 1e-9)
        );
        let again = zoom_on_main(small.frame, small.width, 0.5, pointer);
        assert!(
            again.frame.w == small.frame.w
                && again.frame.x == small.frame.x
                && again.frame.y == small.frame.y
        );
        let big = zoom_on_main(frame, 480.0, 100.0, pointer);
        assert!(big.frame.w == 1209.0 && big.frame.h <= 945.0 * 0.8 + 0.5);
        let more = zoom_on_main(big.frame, big.width, 3.0, pointer);
        let back = zoom_on_main(more.frame, more.width, scroll_factor(-5.0, true), pointer);
        assert!(more.frame.w == big.frame.w && back.frame.w < big.frame.w);
        assert!(
            big.frame.x >= 0.0
                && big.frame.max_x() <= 1512.0
                && big.frame.y >= 0.0
                && big.frame.max_y() <= 945.0
        );
    }

    #[test]
    fn width_limits_for_a_tall_aspect() {
        let tall = 9.0 / 16.0;
        let (low, high) = width_limits(tall, MIN_SIZE, MAX_SIZE, VISIBLE);
        assert!(near_by(high, 945.0 * 0.8 * tall, 1e-9) && near_by(low, 284.0, 1e-9));
    }

    #[test]
    fn zoom_stays_on_screen() {
        let frame = rect(900.0, 300.0, 480.0, 270.0);
        let r = zoom_on_main(frame, 480.0, 2.0, pt(1380.0, 435.0));
        assert!(
            r.frame.x >= 0.0
                && r.frame.max_x() <= 1512.0
                && r.frame.y >= 0.0
                && r.frame.max_y() <= 945.0
        );
    }

    #[test]
    fn zoom_keeps_a_corner_on_a_second_display() {
        let second = rect(1512.0, -200.0, 1920.0, 1055.0);
        let frame = rect(1512.0 + 1920.0 - 16.0 - 480.0, -200.0 + 16.0, 480.0, 270.0);
        let max_size = Size {
            w: 1920.0 * 0.8,
            h: 1055.0 * 0.8,
        };
        let r = zoom(
            frame,
            480.0,
            ASPECT,
            1.5,
            pt(3200.0, -100.0),
            second,
            MIN_SIZE,
            max_size,
        );
        assert!(
            near(r.frame.max_x(), 1512.0 + 1920.0 - 16.0)
                && r.frame.y == -200.0 + 16.0
                && r.frame.w == 720.0
        );
    }

    #[test]
    fn repeated_small_zooms_compound_without_rounding_drift() {
        let mut frame = rect(1000.0, 16.0, 480.0, 270.0);
        let mut width = 480.0;
        for _ in 0..100 {
            let r = zoom_on_main(frame, width, scroll_factor(0.5, true), pt(1200.0, 100.0));
            frame = r.frame;
            width = r.width;
        }
        assert!(near_by(width, 480.0 * (0.004f64 * 50.0).exp(), 1e-6) && near(frame.w, width));
    }

    #[test]
    fn zoom_in_then_out_returns_to_the_start() {
        let mut frame = rect(1000.0, 16.0, 480.0, 270.0);
        let mut width = 480.0;
        for i in 0..400 {
            let f = scroll_factor(if i < 200 { 0.5 } else { -0.5 }, true);
            let r = zoom_on_main(frame, width, f, pt(1200.0, 100.0));
            frame = r.frame;
            width = r.width;
            assert!(near(frame.h, frame.w / ASPECT));
        }
        assert!(
            near_by(width, 480.0, 1e-6)
                && frame.w == 480.0
                && frame.h == 270.0
                && near(frame.max_x(), 1480.0)
                && frame.y == 16.0
        );
    }

    #[test]
    fn zoom_of_a_window_parked_past_the_right_edge() {
        let parked = rect(1512.0 - 300.0, 300.0, 480.0, 270.0);
        let grown = zoom_on_main(parked, 480.0, 1.25, pt(1400.0, 400.0));
        assert!(
            grown.frame.w == 600.0
                && near(grown.frame.max_x(), parked.max_x())
                && grown.frame.x < parked.x
        );
        assert!(grown.frame.max_x() > VISIBLE.max_x());
        let shrunk = zoom_on_main(parked, 480.0, 0.8, pt(1400.0, 400.0));
        assert!(
            shrunk.frame.w == 384.0
                && shrunk.frame.x > parked.x
                && shrunk.frame.max_x() < parked.max_x()
        );
        assert!(shrunk.frame.x <= 1400.0 && shrunk.frame.max_x() >= 1400.0);
    }

    #[test]
    fn zoom_of_a_window_parked_past_the_bottom_left() {
        let parked = rect(-200.0, 945.0 - 150.0, 480.0, 270.0);
        let grown = zoom_on_main(parked, 480.0, 1.25, pt(100.0, 900.0));
        assert!(grown.frame.x == parked.x && near(grown.frame.max_y(), parked.max_y()));
        assert!(grown.frame.max_x() > parked.max_x() && grown.frame.y < parked.y);
    }

    #[test]
    fn zoom_of_a_parked_window_never_goes_further_than_the_tuck() {
        for parked in [
            rect(1512.0 - 300.0, 300.0, 480.0, 270.0),
            rect(-200.0, 300.0, 480.0, 270.0),
            rect(500.0, 945.0 - 140.0, 480.0, 270.0),
            rect(500.0, -130.0, 480.0, 270.0),
        ] {
            let mut frame = parked;
            let mut width = parked.w;
            for f in [1.1, 1.3, 0.7, 1.6, 0.5, 1.2] {
                let centre = pt(frame.x + frame.w / 2.0, frame.y + frame.h / 2.0);
                let r = zoom_on_main(frame, width, f, centre);
                let v = VISIBLE;
                assert!(
                    r.frame.x >= v.x.min(frame.x) - 0.51
                        && r.frame.max_x() <= v.max_x().max(frame.max_x()) + 0.51
                );
                assert!(
                    r.frame.y >= v.y.min(frame.y) - 0.51
                        && r.frame.max_y() <= v.max_y().max(frame.max_y()) + 0.51
                );
                assert!(
                    v.x - r.frame.x <= r.frame.w * MOST_PAST_EDGE
                        && r.frame.max_x() - v.max_x() <= r.frame.w * MOST_PAST_EDGE
                );
                assert!(
                    v.y - r.frame.y <= r.frame.h * MOST_PAST_EDGE
                        && r.frame.max_y() - v.max_y() <= r.frame.h * MOST_PAST_EDGE
                );
                frame = r.frame;
                width = r.width;
            }
        }
    }

    #[test]
    fn kept_within_allows_half_past_the_edge() {
        assert_eq!(kept_within(900.0, 400.0, 800.0, 480.0, 0.0, 1512.0), 900.0);
        assert_eq!(
            kept_within(1300.0, 400.0, 1212.0, 480.0, 0.0, 1512.0),
            1292.0
        );
        assert_eq!(
            kept_within(-300.0, 400.0, -200.0, 480.0, 0.0, 1512.0),
            -200.0
        );
        assert_eq!(
            kept_within(1300.0, 400.0, 800.0, 480.0, 0.0, 1512.0),
            1112.0
        );
        assert_eq!(
            kept_within(1352.0, 360.0, 1232.0, 480.0, 0.0, 1512.0),
            1332.0
        );
        assert_eq!(
            kept_within(1500.0, 200.0, 1232.0, 480.0, 0.0, 1512.0),
            1412.0
        );
        assert_eq!(
            kept_within(-320.0, 200.0, -200.0, 480.0, 0.0, 1512.0),
            -100.0
        );
    }
}
