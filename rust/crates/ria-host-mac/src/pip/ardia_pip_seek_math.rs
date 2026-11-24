// Ported from ardia packages/ardiacore/ios/ArdiaPiPSeekMath.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

pub const SKIP_SECONDS: f64 = 10.0;
pub const DEAD_BAND_HALF_WIDTH: f64 = 40.0;
pub const CLICK_SLOP: f64 = 3.0;

pub fn skip_seconds_at(x: f64, width: f64) -> f64 {
    if width.is_nan() || width <= 0.0 || x < 0.0 || x > width {
        return 0.0;
    }
    let from_centre = x - width / 2.0;
    if from_centre.abs() <= DEAD_BAND_HALF_WIDTH {
        return 0.0;
    }
    if from_centre < 0.0 {
        -SKIP_SECONDS
    } else {
        SKIP_SECONDS
    }
}

pub fn travel(dx: f64, dy: f64) -> f64 {
    dx.hypot(dy)
}

pub fn is_click(travel: f64) -> bool {
    travel < CLICK_SLOP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_seconds_by_side() {
        assert_eq!(skip_seconds_at(10.0, 480.0), -10.0);
        assert_eq!(skip_seconds_at(470.0, 480.0), 10.0);
        assert_eq!(skip_seconds_at(240.0, 480.0), 0.0);
    }

    #[test]
    fn skip_seconds_dead_band_edges() {
        assert!(skip_seconds_at(200.0, 480.0) == 0.0 && skip_seconds_at(280.0, 480.0) == 0.0);
        assert!(skip_seconds_at(199.5, 480.0) == -10.0 && skip_seconds_at(280.5, 480.0) == 10.0);
    }

    #[test]
    fn skip_seconds_just_outside_the_dead_band() {
        assert!(
            skip_seconds_at(240.0 - 76.0, 480.0) == -10.0
                && skip_seconds_at(240.0 + 76.0, 480.0) == 10.0
        );
        assert!(
            skip_seconds_at(240.0 - 56.0, 480.0) == -10.0
                && skip_seconds_at(240.0 + 56.0, 480.0) == 10.0
        );
    }

    #[test]
    fn skip_seconds_on_the_smallest_window() {
        assert!(skip_seconds_at(101.9, 284.0) == -10.0 && skip_seconds_at(102.0, 284.0) == 0.0);
        assert!(skip_seconds_at(182.0, 284.0) == 0.0 && skip_seconds_at(182.1, 284.0) == 10.0);
        assert!(skip_seconds_at(0.0, 284.0) == -10.0 && skip_seconds_at(284.0, 284.0) == 10.0);
    }

    #[test]
    fn skip_seconds_outside_or_without_a_window() {
        assert!(skip_seconds_at(-1.0, 284.0) == 0.0 && skip_seconds_at(285.0, 284.0) == 0.0);
        assert!(
            skip_seconds_at(0.0, 0.0) == 0.0
                && skip_seconds_at(40.0, 80.0) == 0.0
                && skip_seconds_at(0.0, 80.0) == 0.0
        );
    }

    #[test]
    fn skip_seconds_is_symmetric() {
        for w in [284.0, 400.0, 853.0, 1209.0] {
            let mut x = 0.0;
            while x <= w {
                assert_eq!(skip_seconds_at(x, w), -skip_seconds_at(w - x, w));
                x += 0.25;
            }
        }
    }

    #[test]
    fn click_slop() {
        assert!(is_click(0.0) && is_click(2.99) && !is_click(3.0));
        assert!(
            is_click(travel(2.0, 2.0))
                && !is_click(travel(3.0, 0.0))
                && !is_click(travel(0.0, -3.0))
                && !is_click(travel(2.2, 2.2))
        );
    }
}
