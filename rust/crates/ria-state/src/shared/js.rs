// Ria original, no Ardia source.

//! JavaScript number and string rules that Rust lacks, used by the ports of `apps/browser/src/lib`.
//! Text rules (`trim`, `\s`) are in `ria_core::search::js_text`.

pub use ria_core::search::js_text::{is_js_space, js_trim};

/// `Number(text)` for a string: `0` for blank text, `NaN` for text that is not a numeric literal.
pub fn js_number(text: &str) -> f64 {
    let t = js_trim(text);
    if t.is_empty() {
        return 0.0;
    }
    let radix = |digits: &str, radix: u32| match digits.is_empty() {
        true => f64::NAN,
        false => digits
            .chars()
            .try_fold(0.0, |acc, c| {
                c.to_digit(radix)
                    .map(|d| acc * f64::from(radix) + f64::from(d))
            })
            .unwrap_or(f64::NAN),
    };
    match t.as_bytes() {
        [b'0', b'x' | b'X', ..] => return radix(&t[2..], 16),
        [b'0', b'o' | b'O', ..] => return radix(&t[2..], 8),
        [b'0', b'b' | b'B', ..] => return radix(&t[2..], 2),
        _ => {}
    }
    let (sign, body) = match t.as_bytes()[0] {
        b'-' => (-1.0, &t[1..]),
        b'+' => (1.0, &t[1..]),
        _ => (1.0, t),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    if is_decimal_literal(body) {
        return body.parse::<f64>().map_or(f64::NAN, |v| sign * v);
    }
    f64::NAN
}

/// `digits [. digits] [e [+-] digits]` with at least one digit in the mantissa.
fn is_decimal_literal(s: &str) -> bool {
    let b = s.as_bytes();
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let int = digits(0);
    let mut i = int;
    let mut frac = 0;
    if b.get(i) == Some(&b'.') {
        frac = digits(i + 1);
        i += 1 + frac;
    }
    if int + frac == 0 {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exp = digits(i);
        if exp == 0 {
            return false;
        }
        i += exp;
    }
    i == b.len()
}

/// `parseInt(text, radix)` for radix 2 to 36 (and `0x` for radix 16), as an `f64`; `NaN` when no digit parses.
pub fn js_parse_int(text: &str, radix: u32) -> f64 {
    let t = text.trim_start_matches(is_js_space);
    let (sign, t) = match t.as_bytes().first() {
        Some(b'-') => (-1.0, &t[1..]),
        Some(b'+') => (1.0, &t[1..]),
        _ => (1.0, t),
    };
    let t = if radix == 16 {
        t.strip_prefix("0x")
            .or_else(|| t.strip_prefix("0X"))
            .unwrap_or(t)
    } else {
        t
    };
    let mut digits = t.chars().map_while(|c| c.to_digit(radix)).peekable();
    if digits.peek().is_none() {
        return f64::NAN;
    }
    sign * digits.fold(0.0, |acc, d| acc * f64::from(radix) + f64::from(d))
}

/// `Math.round(x)`: halves round toward positive infinity.
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() || x.fract() == 0.0 {
        return x;
    }
    // `(x + 0.5).floor()` rounds `0.49999999999999994` up; `x - floor(x)` is exact.
    let f = x.floor();
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    // `-0.2 + 0.5` floors to `0`, but JavaScript gives `-0`.
    if r == 0.0 && x < 0.0 { -0.0 } else { r }
}

/// `Math.max(a, b)`: `NaN` wins, and `+0` is above `-0`.
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_positive() { a } else { b }
    } else {
        a.max(b)
    }
}

/// `Math.min(a, b)`: `NaN` wins, and `-0` is below `+0`.
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() { a } else { b }
    } else {
        a.min(b)
    }
}
