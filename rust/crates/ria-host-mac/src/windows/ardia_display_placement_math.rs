// Ported from ardia packages/ardiacore/ios/ArdiaDisplayPlacementMath.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn max_x(self) -> f64 {
        self.x + self.w
    }

    pub fn max_y(self) -> f64 {
        self.y + self.h
    }
}

pub fn overlap_area(a: Rect, b: Rect) -> f64 {
    let w = a.max_x().min(b.max_x()) - a.x.max(b.x);
    let h = a.max_y().min(b.max_y()) - a.y.max(b.y);
    if w > 0. && h > 0. { w * h } else { 0. }
}

pub fn holds_most(frame: Rect, screens: &[Rect], target: usize) -> bool {
    let Some(screen) = screens.get(target) else {
        return false;
    };
    has_most(frame, *screen, screens.iter().copied())
}

pub(super) fn has_most(frame: Rect, target: Rect, screens: impl IntoIterator<Item = Rect>) -> bool {
    let overlap = overlap_area(frame, target);
    overlap > 0.
        && screens
            .into_iter()
            .all(|screen| overlap_area(frame, screen) <= overlap)
}

pub fn on_screen(frame: Rect, screens: &[Rect], target: usize, visible: Rect) -> Rect {
    place(frame, visible, holds_most(frame, screens, target))
}

pub(super) fn place(frame: Rect, visible: Rect, holds_most: bool) -> Rect {
    let w = frame.w.min(visible.w);
    let h = frame.h.min(visible.h);
    if holds_most {
        Rect::new(
            frame.x.clamp(visible.x, visible.max_x() - w),
            frame.y.clamp(visible.y, visible.max_y() - h),
            w,
            h,
        )
    } else {
        Rect::new(
            (visible.x + (visible.w - w) / 2.).round(),
            (visible.y + (visible.h - h) / 2.).round(),
            w,
            h,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: Rect = Rect::new(0., 0., 1512., 982.);
    const MAIN_VISIBLE: Rect = Rect::new(0., 0., 1512., 944.);
    const RIGHT: Rect = Rect::new(1512., -120., 2560., 1440.);
    const RIGHT_VISIBLE: Rect = Rect::new(1512., -120., 2560., 1415.);
    const LEFT: Rect = Rect::new(-1920., 200., 1920., 1080.);
    const LEFT_VISIBLE: Rect = Rect::new(-1920., 200., 1920., 1055.);

    fn inside(inner: Rect, outer: Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.max_x() <= outer.max_x()
            && inner.max_y() <= outer.max_y()
    }

    #[test]
    fn window_saved_on_main_moves_to_external() {
        let screens = [MAIN, RIGHT];
        let saved = Rect::new(100., 80., 1200., 760.);
        let placed = on_screen(saved, &screens, 1, RIGHT_VISIBLE);
        assert!(inside(placed, RIGHT_VISIBLE));
        assert_eq!((placed.w, placed.h), (saved.w, saved.h));
        assert!(holds_most(placed, &screens, 1));
        assert!(!holds_most(placed, &screens, 0));
    }

    #[test]
    fn window_already_on_target_is_untouched() {
        let screens = [MAIN, RIGHT];
        let saved = Rect::new(1700., 100., 1000., 700.);
        assert_eq!(on_screen(saved, &screens, 1, RIGHT_VISIBLE), saved);
        let saved = Rect::new(40., 30., 900., 600.);
        assert_eq!(on_screen(saved, &screens, 0, MAIN_VISIBLE), saved);
    }

    #[test]
    fn window_straddling_goes_to_the_display_holding_most() {
        let screens = [MAIN, RIGHT];
        let saved = Rect::new(1312., 100., 1000., 700.);
        assert!(holds_most(saved, &screens, 1));
        assert!(!holds_most(saved, &screens, 0));
        assert_eq!(
            on_screen(saved, &screens, 1, RIGHT_VISIBLE),
            Rect::new(1512., 100., 1000., 700.)
        );
        assert!(inside(
            on_screen(saved, &screens, 0, MAIN_VISIBLE),
            MAIN_VISIBLE
        ));
    }

    #[test]
    fn frame_holding_most_but_past_the_visible_area_is_pulled_in_not_centred() {
        let screens = [MAIN, RIGHT];
        let saved = Rect::new(600., 700., 800., 600.);
        assert!(holds_most(saved, &screens, 0));
        let placed = on_screen(saved, &screens, 0, MAIN_VISIBLE);
        assert!(inside(placed, MAIN_VISIBLE));
        assert_eq!(placed, Rect::new(600., 344., 800., 600.));
        assert_eq!(
            on_screen(
                Rect::new(1000., 100., 900., 600.),
                &screens,
                0,
                MAIN_VISIBLE
            ),
            Rect::new(612., 100., 900., 600.)
        );
        assert_eq!(
            on_screen(Rect::new(100., 0., 700., 960.), &screens, 0, MAIN_VISIBLE),
            Rect::new(100., 0., 700., 944.)
        );
    }

    #[test]
    fn oversized_window_is_clamped_and_centred() {
        assert_eq!(
            on_screen(
                Rect::new(50., 50., 3000., 2000.),
                &[MAIN, LEFT],
                1,
                LEFT_VISIBLE
            ),
            LEFT_VISIBLE
        );
    }

    #[test]
    fn zero_overlap_is_centred_on_target() {
        let screens = [MAIN, RIGHT];
        let saved = Rect::new(9000., 9000., 800., 600.);
        assert!(!holds_most(saved, &screens, 1));
        let placed = on_screen(saved, &screens, 1, RIGHT_VISIBLE);
        assert!(inside(placed, RIGHT_VISIBLE));
        assert_eq!(placed, Rect::new(2392., 288., 800., 600.));
    }

    #[test]
    fn negative_coordinate_layouts() {
        let screens = [LEFT, MAIN, RIGHT];
        let placed = on_screen(
            Rect::new(200., 100., 1000., 700.),
            &screens,
            0,
            LEFT_VISIBLE,
        );
        assert!(inside(placed, LEFT_VISIBLE));
        assert!(holds_most(placed, &screens, 0));
        let saved = Rect::new(-1500., 300., 1000., 700.);
        assert_eq!(on_screen(saved, &screens, 0, LEFT_VISIBLE), saved);
        let placed = on_screen(
            Rect::new(300., -500., 800., 600.),
            &screens,
            2,
            RIGHT_VISIBLE,
        );
        assert!(inside(placed, RIGHT_VISIBLE));
    }

    #[test]
    fn overlap_is_symmetric_and_zero_when_apart() {
        assert_eq!(overlap_area(MAIN, RIGHT), 0.);
        assert_eq!(overlap_area(RIGHT, MAIN), 0.);
        let a = Rect::new(0., 0., 100., 100.);
        let b = Rect::new(50., 50., 100., 100.);
        assert_eq!(overlap_area(a, b), 2500.);
        assert_eq!(overlap_area(b, a), 2500.);
        assert_eq!(overlap_area(a, Rect::new(100., 0., 100., 100.)), 0.);
    }
}
