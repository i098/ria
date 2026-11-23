// Ported from ardia packages/ardiacore/ios/ArdiaLittleFitMath.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use super::ardia_display_placement_math::overlap_area;
use super::ardia_window_sight_math::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

impl Size {
    pub const fn new(w: f64, h: f64) -> Self {
        Self { w, h }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Display {
    pub frame: Rect,
    pub visible: Rect,
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub wanted: Size,
    pub placed: Rect,
}

pub const CONTAIN_TOLERANCE: f64 = 1.;
const LARGEST_SHARE: f64 = 0.8;
const WHOLE_SCREEN: f64 = 1.;

fn fit_side(wanted: f64, minimum: f64, visible: f64, share: f64) -> f64 {
    let largest = (visible * share).round().max(minimum).min(visible);
    wanted.max(minimum).min(largest).round().min(visible)
}

fn fit_origin(start: f64, extent: f64, side: f64) -> f64 {
    (start + (extent - side) / 2.)
        .round()
        .clamp(start, (start + extent - side).max(start))
}

fn fit_with_share(visible: Rect, wanted: Size, minimum: Size, share: f64) -> Rect {
    let w = fit_side(wanted.w, minimum.w, visible.width, share);
    let h = fit_side(wanted.h, minimum.h, visible.height, share);
    Rect::new(
        fit_origin(visible.x, visible.width, w),
        fit_origin(visible.y, visible.height, h),
        w,
        h,
    )
}

pub fn fit(visible: Rect, wanted: Size, minimum: Size) -> Rect {
    fit_with_share(visible, wanted, minimum, LARGEST_SHARE)
}

pub fn contains(outer: Rect, inner: Rect, tolerance: f64) -> bool {
    inner.x >= outer.x - tolerance
        && inner.y >= outer.y - tolerance
        && inner.max_x() <= outer.max_x() + tolerance
        && inner.max_y() <= outer.max_y() + tolerance
}

pub fn screen_holding(bounds: Rect, frames: impl IntoIterator<Item = Rect>) -> Option<usize> {
    let mut best = None;
    let mut best_overlap = 0.;
    for (index, frame) in frames.into_iter().enumerate() {
        let overlap = overlap_area(bounds, frame);
        if overlap > best_overlap {
            best = Some(index);
            best_overlap = overlap;
        }
    }
    best
}

pub fn flip_from_top_left(rect: Rect, primary_height: f64) -> Rect {
    Rect {
        y: primary_height - rect.y - rect.height,
        ..rect
    }
}

pub fn fit_landing(
    landed: Rect,
    displays: &[Display],
    minimum: Size,
    placement: Option<Placement>,
) -> Option<Rect> {
    let frames = || displays.iter().map(|display| display.frame);
    let held = screen_holding(landed, frames())?;
    let visible = displays[held].visible;
    if let Some(placement) = placement
        && screen_holding(placement.placed, frames()) != Some(held)
    {
        return Some(fit(visible, placement.wanted, minimum));
    }
    (!contains(visible, landed, CONTAIN_TOLERANCE)).then(|| {
        fit_with_share(
            visible,
            Size::new(landed.width, landed.height),
            minimum,
            WHOLE_SCREEN,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMUM: Size = Size::new(420., 300.);
    const BUILT_IN: Rect = Rect::new(-1512., 0., 1512., 950.);
    const ULTRAWIDE: Rect = Rect::new(0., 0., 3440., 1415.);
    const DISPLAYS: [Display; 2] = [
        Display {
            frame: Rect::new(0., 0., 3440., 1440.),
            visible: ULTRAWIDE,
        },
        Display {
            frame: Rect::new(-1512., 0., 1512., 982.),
            visible: Rect::new(-1512., 70., 1512., 886.),
        },
    ];

    #[test]
    fn fit_tests() {
        for (visible, wanted, expected) in [
            (
                ULTRAWIDE,
                Size::new(2406., 1085.),
                Rect::new(517., 165., 2406., 1085.),
            ),
            (
                BUILT_IN,
                Size::new(2406., 1085.),
                Rect::new(-1361., 95., 1210., 760.),
            ),
            (
                BUILT_IN,
                Size::new(1400., 400.),
                Rect::new(-1361., 275., 1210., 400.),
            ),
            (
                BUILT_IN,
                Size::new(1210., 760.),
                Rect::new(-1361., 95., 1210., 760.),
            ),
            (
                BUILT_IN,
                Size::new(1200., 700.),
                Rect::new(-1356., 125., 1200., 700.),
            ),
            (
                ULTRAWIDE,
                Size::new(3200., 1400.),
                Rect::new(344., 142., 2752., 1132.),
            ),
            (
                Rect::new(0., 0., 300., 200.),
                Size::new(900., 640.),
                Rect::new(0., 0., 300., 200.),
            ),
            (
                Rect::new(-1512., 40., 1512., 910.),
                Size::new(2406., 1085.),
                Rect::new(-1361., 131., 1210., 728.),
            ),
            (
                Rect::new(0., 0., 500., 360.),
                Size::new(900., 640.),
                Rect::new(40., 30., 420., 300.),
            ),
        ] {
            let fitted = fit(visible, wanted, MINIMUM);
            assert_eq!(fitted, expected);
            assert!(contains(visible, fitted, CONTAIN_TOLERANCE));
        }
        let floor = fit(BUILT_IN, Size::new(100., 100.), MINIMUM);
        assert_eq!((floor.width, floor.height), (420., 300.));
        assert!(contains(BUILT_IN, floor, CONTAIN_TOLERANCE));
    }

    #[test]
    fn centre_tests() {
        let visible = Rect::new(0., 0., 1000., 1000.);
        for (height, expected) in [
            (400., Rect::new(250., 300., 500., 400.)),
            (800., Rect::new(250., 100., 500., 800.)),
            (960., Rect::new(250., 100., 500., 800.)),
        ] {
            assert_eq!(fit(visible, Size::new(500., height), MINIMUM), expected);
        }
    }

    #[test]
    fn screen_holding_tests() {
        let screens = [ULTRAWIDE, BUILT_IN];
        for (bounds, expected) in [
            (Rect::new(-1400., 100., 800., 600.), Some(1)),
            (Rect::new(100., 100., 800., 600.), Some(0)),
            (Rect::new(-300., 100., 800., 600.), Some(0)),
            (Rect::new(-900., 100., 1000., 600.), Some(1)),
            (Rect::new(9000., 9000., 100., 100.), None),
        ] {
            assert_eq!(screen_holding(bounds, screens), expected);
        }
        assert_eq!(screen_holding(Rect::new(0., 0., 10., 10.), []), None);
    }

    #[test]
    fn contains_tests() {
        for (inner, expected) in [
            (BUILT_IN, true),
            (Rect::new(-1512.5, -0.5, 1513., 951.), true),
            (Rect::new(-1959., 100., 2406., 1085.), false),
            (Rect::new(-1512., 0., 1512., 952.), false),
        ] {
            assert_eq!(contains(BUILT_IN, inner, CONTAIN_TOLERANCE), expected);
        }
    }

    #[test]
    fn flip_tests() {
        let input = Rect::new(-1959., 233., 2406., 1085.);
        let flipped = flip_from_top_left(input, 1415.);
        assert_eq!(
            flipped,
            Rect::new(-1959., 1415. - 233. - 1085., 2406., 1085.)
        );
        assert_eq!(flip_from_top_left(flipped, 1415.), input);
    }

    #[test]
    fn landed_window_tests() {
        assert_eq!(
            fit_landing(
                Rect::new(517., 236., 2406., 1085.),
                &DISPLAYS,
                MINIMUM,
                None
            ),
            None
        );
        assert_eq!(
            fit_landing(
                Rect::new(-1959., 100., 2406., 1085.),
                &DISPLAYS,
                MINIMUM,
                None
            ),
            Some(DISPLAYS[1].visible)
        );
    }

    #[test]
    fn landed_beyond_the_visible_frame_tests() {
        let display = DISPLAYS[1];
        for (landed, expected) in [
            (
                Rect::new(-1400., 10., 800., 600.),
                Rect::new(-1156., 213., 800., 600.),
            ),
            (
                Rect::new(-1400., 500., 800., 480.),
                Rect::new(-1156., 273., 800., 480.),
            ),
        ] {
            assert!(contains(display.frame, landed, CONTAIN_TOLERANCE));
            assert!(!contains(display.visible, landed, CONTAIN_TOLERANCE));
            assert_eq!(
                fit_landing(landed, &DISPLAYS, MINIMUM, None),
                Some(expected)
            );
            assert!(contains(display.visible, expected, CONTAIN_TOLERANCE));
        }
        let inside = Rect::new(-1400., 100., 800., 600.);
        assert!(contains(display.visible, inside, CONTAIN_TOLERANCE));
        assert_eq!(fit_landing(inside, &DISPLAYS, MINIMUM, None), None);
        assert_eq!(
            fit_landing(
                Rect::new(9000., 9000., 800., 600.),
                &DISPLAYS,
                MINIMUM,
                None
            ),
            None
        );
        assert_eq!(fit_landing(inside, &[], MINIMUM, None), None);
    }

    #[test]
    fn landed_with_a_wanted_size_tests() {
        let wanted = Size::new(1400., 850.);
        let wide = fit(DISPLAYS[0].visible, wanted, MINIMUM);
        assert_eq!(wide, Rect::new(1020., 283., 1400., 850.));
        let wide_placement = Some(Placement {
            wanted,
            placed: wide,
        });
        assert_eq!(fit_landing(wide, &DISPLAYS, MINIMUM, wide_placement), None);
        let moved = Rect::new(-1450., 80., 1400., 850.);
        assert!(contains(DISPLAYS[1].visible, moved, CONTAIN_TOLERANCE));
        assert_eq!(
            fit_landing(moved, &DISPLAYS, MINIMUM, wide_placement),
            Some(Rect::new(-1361., 159., 1210., 709.))
        );
        let laptop = fit(DISPLAYS[1].visible, wanted, MINIMUM);
        let laptop_placement = Some(Placement {
            wanted,
            placed: laptop,
        });
        assert_eq!(
            fit_landing(laptop, &DISPLAYS, MINIMUM, laptop_placement),
            None
        );
        assert_eq!(
            fit_landing(
                Rect::new(200., 200., laptop.width, laptop.height),
                &DISPLAYS,
                MINIMUM,
                laptop_placement
            ),
            Some(wide)
        );
        assert_eq!(
            fit_landing(
                Rect::new(-1450., 100., 1300., 800.),
                &DISPLAYS,
                MINIMUM,
                laptop_placement
            ),
            None
        );
        let resized = fit_landing(
            Rect::new(-1450., 10., 1300., 800.),
            &DISPLAYS,
            MINIMUM,
            laptop_placement,
        )
        .unwrap();
        assert_eq!((resized.width, resized.height), (1300., 800.));
        assert!(contains(DISPLAYS[1].visible, resized, CONTAIN_TOLERANCE));
        assert_eq!(fit_landing(moved, &DISPLAYS, MINIMUM, None), None);
    }

    #[test]
    fn rounding_stays_inside_tests() {
        let visible = Rect::new(0., 0., 419.5, 299.5);
        let fitted = fit(visible, Size::new(900., 640.), MINIMUM);
        assert!(fitted.width <= visible.width && fitted.height <= visible.height);
        assert!(contains(visible, fitted, 0.));
        assert_eq!(
            fit(
                Rect::new(0.5, 0., 1000., 1000.),
                Size::new(501., 400.),
                MINIMUM
            ),
            Rect::new(250., 300., 501., 400.)
        );
    }
}
