// Ported from ardia packages/ardiacore/ios/ArdiaWindowSightMath.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn right(self) -> f64 {
        self.x + self.width
    }

    fn bottom(self) -> f64 {
        self.y + self.height
    }

    fn contains(self, x: f64, y: f64) -> bool {
        self.x < x && x < self.right() && self.y < y && y < self.bottom()
    }
}

pub fn visible_area(window: Rect, covers: &[Rect], displays: &[Rect]) -> f64 {
    if window.width <= 0. || window.height <= 0. || displays.is_empty() {
        return 0.;
    }
    fn edges(limits: [f64; 2], edges: impl Iterator<Item = f64>) -> Vec<f64> {
        let [low, high] = limits;
        let mut values = Vec::from(limits);
        values.extend(edges.filter(|&edge| low < edge && edge < high));
        values.sort_unstable_by(f64::total_cmp);
        values.dedup();
        values
    }
    let rects = || covers.iter().chain(displays).copied();
    let xs = edges(
        [window.x, window.right()],
        rects().flat_map(|r| [r.x, r.right()]),
    );
    let ys = edges(
        [window.y, window.bottom()],
        rects().flat_map(|r| [r.y, r.bottom()]),
    );
    let mut area = 0.;
    for x in xs.windows(2) {
        let mid_x = (x[0] + x[1]) / 2.;
        for y in ys.windows(2) {
            let mid_y = (y[0] + y[1]) / 2.;
            if displays.iter().any(|r| r.contains(mid_x, mid_y))
                && !covers.iter().any(|r| r.contains(mid_x, mid_y))
            {
                area += (x[1] - x[0]) * (y[1] - y[0]);
            }
        }
    }
    area
}

pub fn counts_as_cover(layer: i32, alpha: f64) -> bool {
    layer == 0 && alpha >= 1.
}

pub fn partly_visible(window: Rect, covers: &[Rect], displays: &[Rect]) -> bool {
    visible_area(window, covers, displays) >= 1.
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: Rect = Rect::new(0., 0., 1512., 982.);
    const EXTERNAL: Rect = Rect::new(1512., -200., 2560., 1440.);
    const BOTH: [Rect; 2] = [LAPTOP, EXTERNAL];

    #[test]
    fn window_sight() {
        let browser = Rect::new(100., 80., 1200., 800.);
        let straddling = Rect::new(1400., 100., 300., 300.);
        let left_of_seam = Rect::new(1400., 100., 112., 300.);
        for (window, covers, displays, expected) in [
            (browser, &[][..], &BOTH[..], true),
            (browser, &[Rect::new(1600., 0., 2000., 1200.)], &BOTH, true),
            (
                browser,
                &[
                    Rect::new(0., 0., 1512., 25.),
                    Rect::new(200., 200., 1400., 600.),
                ],
                &BOTH,
                true,
            ),
            (browser, &[LAPTOP], &BOTH, false),
            (
                browser,
                &[
                    Rect::new(0., 0., 800., 982.),
                    Rect::new(700., 0., 812., 982.),
                ],
                &BOTH,
                false,
            ),
            (
                browser,
                &[
                    Rect::new(0., 0., 800., 982.),
                    Rect::new(801., 0., 711., 982.),
                ],
                &BOTH,
                true,
            ),
            (browser, &[], &[EXTERNAL], false),
            (straddling, &[left_of_seam], &[LAPTOP], false),
            (straddling, &[left_of_seam], &BOTH, true),
            (Rect::new(0., 0., 0., 100.), &[], &BOTH, false),
        ] {
            assert_eq!(partly_visible(window, covers, displays), expected);
        }
        let area = visible_area(
            Rect::new(0., 0., 100., 100.),
            &[Rect::new(50., 0., 100., 100.)],
            &BOTH,
        );
        assert!((area - 5000.).abs() < 1e-9);
        for (layer, alpha, expected) in [
            (0, 1., true),
            (0, 0.97, false),
            (0, 0.5, false),
            (3, 1., false),
            (25, 1., false),
            (1000, 1., false),
            (-1, 1., false),
        ] {
            assert_eq!(counts_as_cover(layer, alpha), expected);
        }
    }

    #[test]
    fn sampled_area_agrees_with_exact_area() {
        struct Random(u64);
        impl Random {
            fn next(&mut self) -> u64 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                self.0 >> 33
            }

            fn uniform(&mut self, low: f64, high: f64) -> f64 {
                low + (high - low) * (self.next() % 1_000_000) as f64 / 1_000_000.
            }
        }

        let mut random = Random(0x1236);
        let mut covers = Vec::new();
        for round in 0..3000 {
            let window = Rect::new(
                random.uniform(-200., 3000.),
                random.uniform(-300., 900.),
                random.uniform(50., 1500.),
                random.uniform(50., 1000.),
            );
            covers.clear();
            for _ in 0..random.next() % 12 {
                covers.push(Rect::new(
                    random.uniform(-300., 3500.),
                    random.uniform(-400., 1200.),
                    random.uniform(10., 2000.),
                    random.uniform(10., 1400.),
                ));
            }
            let dx = window.width / 400.;
            let dy = window.height / 400.;
            let mut sampled = 0.;
            for i in 0..400 {
                let x = window.x + (f64::from(i) + 0.5) * dx;
                for j in 0..400 {
                    let y = window.y + (f64::from(j) + 0.5) * dy;
                    let includes = |r: &Rect| {
                        (0. ..r.width).contains(&(x - r.x))
                            && (0. ..r.height).contains(&(y - r.y))
                    };
                    if BOTH.iter().any(includes) && !covers.iter().any(includes) {
                        sampled += dx * dy;
                    }
                }
            }
            let exact = visible_area(window, &covers, &BOTH);
            assert!(
                (exact - sampled).abs() <= window.width * window.height * 0.02 + 1.,
                "round {round}: exact {exact}, sampled {sampled}"
            );
        }
    }
}
