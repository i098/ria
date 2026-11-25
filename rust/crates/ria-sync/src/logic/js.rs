// Ria original, no Ardia source.

//! The JavaScript behaviours that the port must keep: `Number` to string, `JSON.stringify` key
//! order, `parseInt` in base 36, `Math.min`/`Math.max` with `NaN`, and `Date.now`.

use serde_json::Value;
use std::fmt::Write as _;

/// `String(x)` for a number (ECMA-262 Number::toString, radix 10).
pub fn number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.into();
    }
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().unwrap_or(0) + 1;
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    if k <= n && n <= 21 {
        out.push_str(&digits);
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(&digits);
    } else {
        let e = n - 1;
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        let _ = write!(out, "e{}{}", if e < 0 { '-' } else { '+' }, e.abs());
    }
    out
}

/// The JSON number of a JS number: an integer when it has no fraction, else a float.
/// `NaN` and the infinities become `null`, as `JSON.stringify` does.
pub fn number(x: f64) -> Value {
    if x.is_finite() && x.fract() == 0.0 && x.abs() < 9.0e15 {
        Value::from(x as i64)
    } else {
        serde_json::Number::from_f64(x).map_or(Value::Null, Value::Number)
    }
}

/// `true` when `key` is a canonical array index (`"0"`, `"17"`, not `"01"`, not `"-1"`).
fn is_index(key: &str) -> bool {
    match key.parse::<u32>() {
        Ok(n) => n != u32::MAX && n.to_string() == key,
        Err(_) => false,
    }
}

fn write_string(out: &mut String, s: &str) {
    // serde_json escapes the same set as `JSON.stringify` for well-formed strings.
    out.push_str(&serde_json::to_string(s).unwrap_or_default());
}

fn write_value(out: &mut String, v: &Value) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.is_finite() => out.push_str(&number_to_string(f)),
            _ => out.push_str("null"),
        },
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut index: Vec<(&String, &Value)> =
                map.iter().filter(|(k, _)| is_index(k)).collect();
            index.sort_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0));
            let rest = map.iter().filter(|(k, _)| !is_index(k));
            for (i, (k, item)) in index.into_iter().chain(rest).enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, k);
                out.push(':');
                write_value(out, item);
            }
            out.push('}');
        }
    }
}

/// `JSON.stringify(value)`: integer-like keys first and ascending, numbers as JS prints them.
pub fn stringify(v: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, v);
    out
}

/// Compare strings by UTF-16 code units, as the default `Array.prototype.sort` does.
pub fn cmp_utf16(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// `str.startsWith`-safe length in UTF-16 units, as `String.prototype.length` gives.
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{a}' | '\u{b}' | '\u{c}' | '\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `parseInt(text, 36)`: leading white space, a sign, then the longest run of base-36 digits.
/// Like V8, it adds the digits in groups of 5, so a number above 2^53 rounds as V8 rounds it.
pub fn parse_int36(text: &str) -> f64 {
    const GROUP: u32 = 36 * 36 * 36 * 36 * 36;
    let s = text.trim_start_matches(is_js_space);
    let (neg, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let (mut value, mut part, mut multiplier, mut any) = (0.0_f64, 0_u32, 1_u32, false);
    for d in s
        .chars()
        .map_while(|c| if c.is_ascii() { c.to_digit(36) } else { None })
    {
        if multiplier == GROUP {
            value = value * f64::from(multiplier) + f64::from(part);
            (part, multiplier) = (0, 1);
        }
        part = part * 36 + d;
        multiplier *= 36;
        any = true;
    }
    value = value * f64::from(multiplier) + f64::from(part);
    match (any, neg) {
        (false, _) => f64::NAN,
        (true, true) => -value,
        (true, false) => value,
    }
}

/// `x.toString(36)` for an integral number (`Math.floor` output). Like V8 (`DoubleToRadixCString`),
/// it prints `0` for each digit that a double cannot hold.
pub fn to_string36(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.into();
    }
    let neg = x < 0.0;
    let mut n = x.abs().trunc();
    let mut digits = Vec::new();
    while n / 36.0 >= 9_007_199_254_740_992.0 {
        n /= 36.0;
        digits.push(b'0');
    }
    loop {
        let rest = n % 36.0;
        digits.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[rest as usize]);
        n = (n - rest) / 36.0;
        if n <= 0.0 {
            break;
        }
    }
    if neg {
        digits.push(b'-');
    }
    digits.reverse();
    String::from_utf8(digits).unwrap_or_default()
}

/// `Math.min(a, b)`.
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// `Math.max(a, b)`.
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b || (a == b && b.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// `Date.now()`.
pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_as_js() {
        for (x, want) in [
            (1.0, "1"),
            (-0.5, "-0.5"),
            (1e21, "1e+21"),
            (1e-7, "1e-7"),
            (123456789012345680000.0, "123456789012345680000"),
            (0.000001, "0.000001"),
            (1.5e300, "1.5e+300"),
        ] {
            assert_eq!(number_to_string(x), want);
        }
    }

    #[test]
    fn stringify_orders_index_keys_first() {
        let v: Value = serde_json::from_str(r#"{"b":1,"2":2,"a":1.0,"10":3}"#).unwrap();
        assert_eq!(stringify(&v), r#"{"2":2,"10":3,"b":1,"a":1}"#);
    }

    #[test]
    fn base36_round_trips() {
        assert_eq!(
            parse_int36(&to_string36(1_795_417_631_295.0)),
            1_795_417_631_295.0
        );
        assert!(parse_int36("").is_nan());
        assert_eq!(parse_int36(" -zz!"), -1295.0);
    }
}
