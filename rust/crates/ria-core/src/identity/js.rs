// Ria original, no Ardia source.

//! `Math.min`, `Math.max` and `Math.round` as JavaScript gives them: `NaN` propagates, `-0` is
//! kept, and halves round up.

/// JS `Math.max(a, b)`: NaN wins (Rust's `f64::max` drops it).
pub(crate) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b || (a == b && b.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// JS `Math.min(a, b)`.
pub(crate) fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// JS `Math.round`: halves go up, and -0.5..-0 gives -0.
pub(crate) fn js_round(x: f64) -> f64 {
    let r = x.floor();
    let r = if x - r >= 0.5 { r + 1.0 } else { r };
    if r == 0.0 && x < 0.0 { -0.0 } else { r }
}
