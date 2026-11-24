// Ported from ardia packages/ardiacore/ios/ArdiaPaintProbeMath.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

pub const ALPHA_ZERO: u32 = 1 << 0;
pub const VIEW_HIDDEN: u32 = 1 << 1;
pub const PAGE_HIDDEN: u32 = 1 << 2;
pub const PAGE_DETACHED: u32 = 1 << 3;
pub const FRAME_HOLD_AWAITING: u32 = 1 << 4;
pub const STAND_IN_LENT: u32 = 1 << 5;
pub const STAND_IN_LEAVING: u32 = 1 << 6;
pub const VEIL_UP: u32 = 1 << 7;

pub const REASON_COUNT: u32 = 8;

#[derive(Clone, Copy, Default)]
pub struct View {
    pub alpha_zero: bool,
    pub view_hidden: bool,
    pub page_hidden: bool,
    pub page_detached: bool,
    pub awaiting_frame: bool,
    pub stand_in_lent: bool,
    pub stand_in_leaving: bool,
    pub veil_up: bool,
}

pub fn reasons_of(view: &View) -> u32 {
    [
        (view.alpha_zero, ALPHA_ZERO),
        (view.view_hidden, VIEW_HIDDEN),
        (view.page_hidden, PAGE_HIDDEN),
        (view.page_detached, PAGE_DETACHED),
        (view.awaiting_frame, FRAME_HOLD_AWAITING),
        (view.stand_in_lent, STAND_IN_LENT),
        (view.stand_in_leaving, STAND_IN_LEAVING),
        (view.veil_up, VEIL_UP),
    ]
    .into_iter()
    .filter(|(set, _)| *set)
    .fold(0, |reasons, (_, bit)| reasons | bit)
}

#[derive(Clone, Copy, Default)]
pub struct Tally {
    pub views: u32,
    pub unpainted: u32,
    pub reasons: u32,
}

impl Tally {
    pub fn add(&mut self, view_reasons: u32) {
        self.views += 1;
        if view_reasons == 0 {
            return;
        }
        self.unpainted += 1;
        self.reasons |= view_reasons;
    }
}

#[derive(Default)]
pub struct Episode {
    sent: bool,
}

impl Episode {
    pub fn should_send(&mut self, tally: &Tally) -> bool {
        if tally.unpainted == 0 {
            self.sent = false;
            return false;
        }
        if self.sent {
            return false;
        }
        self.sent = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_probe() {
        assert_eq!(reasons_of(&View::default()), 0);

        let alpha = View {
            alpha_zero: true,
            ..View::default()
        };
        assert_eq!(reasons_of(&alpha), ALPHA_ZERO);

        let lent = View {
            stand_in_lent: true,
            stand_in_leaving: true,
            ..View::default()
        };
        assert_eq!(reasons_of(&lent), STAND_IN_LENT | STAND_IN_LEAVING);

        let everything = View {
            alpha_zero: true,
            view_hidden: true,
            page_hidden: true,
            page_detached: true,
            awaiting_frame: true,
            stand_in_lent: true,
            stand_in_leaving: true,
            veil_up: true,
        };
        assert_eq!(reasons_of(&everything), (1 << REASON_COUNT) - 1);

        let mut painted = Tally::default();
        painted.add(0);
        painted.add(0);
        assert!(painted.views == 2 && painted.unpainted == 0 && painted.reasons == 0);

        let mut mixed = Tally::default();
        mixed.add(0);
        mixed.add(FRAME_HOLD_AWAITING);
        mixed.add(VEIL_UP | ALPHA_ZERO);
        assert!(mixed.views == 3 && mixed.unpainted == 2);
        assert_eq!(mixed.reasons, FRAME_HOLD_AWAITING | VEIL_UP | ALPHA_ZERO);

        let mut episode = Episode::default();
        assert!(!episode.should_send(&painted));
        assert!(episode.should_send(&mixed));
        assert!(!episode.should_send(&mixed));
        assert!(!episode.should_send(&mixed));
        assert!(!episode.should_send(&painted));
        assert!(episode.should_send(&mixed));

        assert!(!Episode::default().should_send(&Tally::default()));
    }
}
