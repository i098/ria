// Ported from ardia packages/core/src/telemetryWire.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The OTLP log wire format of the telemetry senders.
//!
//! `ria-core` has no JSON crate, so this file holds the small JSON value that
//! `Record<string, unknown>` needs, with the text rules of `JSON.stringify`.

use std::fmt::Write;

/// A JSON value. An object keeps its entries in insertion order; `js_order` gives the order
/// that JavaScript uses for `Object.entries` and `JSON.stringify`.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

/// Sets `key`, as an object spread or assignment does: a repeated key keeps its first place.
pub fn set_entry(entries: &mut Vec<(String, Json)>, key: &str, value: Json) {
    match entries.iter_mut().find(|(k, _)| k == key) {
        Some(entry) => entry.1 = value,
        None => entries.push((key.to_owned(), value)),
    }
}

/// `true` for a canonical array index (`"0"`, `"17"`; not `"01"`), which JS orders first.
fn array_index(key: &str) -> Option<u32> {
    let canonical = key == "0" || (!key.starts_with('0') && !key.is_empty());
    if !canonical || !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    key.parse::<u32>().ok().filter(|&n| n != u32::MAX)
}

/// The entries in the order of `Object.entries`: array indexes ascending, then the rest in
/// insertion order.
pub fn js_order(entries: &[(String, Json)]) -> Vec<&(String, Json)> {
    let mut indexed: Vec<_> = entries
        .iter()
        .filter_map(|e| Some((array_index(&e.0)?, e)))
        .collect();
    indexed.sort_by_key(|(n, _)| *n);
    let rest = entries.iter().filter(|e| array_index(&e.0).is_none());
    indexed.into_iter().map(|(_, e)| e).chain(rest).collect()
}

/// `Number.prototype.toString` for a finite or non-finite `f64`.
pub fn js_number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.into();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    // `{:e}` gives the shortest round-trip digits: "d.ddde-7".
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').expect("LowerExp has an exponent");
    let digits: String = mantissa.chars().filter(|&c| c != '.').collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().expect("exponent is an integer") + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let signed = if e < 0 {
            format!("-{}", -e)
        } else {
            format!("+{e}")
        };
        if k == 1 {
            format!("{digits}e{signed}")
        } else {
            format!("{}.{}e{signed}", &digits[..1], &digits[1..])
        }
    };
    format!("{sign}{body}")
}

/// `JSON.stringify` of a string.
fn quote(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if c < ' ' => write!(out, "\\u{:04x}", u32::from(c)).expect("write to String"),
            c => out.push(c),
        }
    }
    out.push('"');
}

impl Json {
    /// `JSON.stringify(self)`.
    pub fn stringify(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Number(n) if n.is_finite() => out.push_str(&js_number_to_string(*n)),
            Self::Number(_) => out.push_str("null"),
            Self::String(s) => quote(out, s),
            Self::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Self::Object(entries) => {
                out.push('{');
                for (i, (key, value)) in js_order(entries)
                    .into_iter()
                    .map(|e| (&e.0, &e.1))
                    .enumerate()
                {
                    if i > 0 {
                        out.push(',');
                    }
                    quote(out, key);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OtlpValue {
    String(String),
    /// A safe integer, as the decimal string that `String(value)` gives.
    Int(String),
    Double(f64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OtlpAttribute {
    pub key: String,
    pub value: OtlpValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OtlpLogRecord {
    pub time_unix_nano: String,
    pub severity_number: u8,
    pub severity_text: &'static str,
    pub body: String,
    pub attributes: Vec<OtlpAttribute>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueuedEvent {
    pub uuid: String,
    pub event: String,
    pub distinct_id: String,
    pub timestamp: String,
    pub properties: Vec<(String, Json)>,
}

pub const EVENTS_SCOPE: &str = "ardia.analytics";
pub const LOGS_SCOPE: &str = "ardia.app";

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Port of `otlpValue` (`telemetryWire.ts:21`).
fn otlp_value(value: &Json) -> Option<OtlpValue> {
    match value {
        Json::String(s) => Some(OtlpValue::String(s.clone())),
        Json::Bool(b) => Some(OtlpValue::Bool(*b)),
        Json::Number(n) if !n.is_finite() => None,
        Json::Number(n) if n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER => {
            Some(OtlpValue::Int(js_number_to_string(*n)))
        }
        Json::Number(n) => Some(OtlpValue::Double(*n)),
        Json::Null => None,
        Json::Array(_) | Json::Object(_) => Some(OtlpValue::String(value.stringify())),
    }
}

/// Port of `otlpAttributes` (`telemetryWire.ts:36`).
fn otlp_attributes(values: &[(String, Json)]) -> Vec<OtlpAttribute> {
    js_order(values)
        .into_iter()
        .filter_map(|(key, v)| {
            Some(OtlpAttribute {
                key: key.clone(),
                value: otlp_value(v)?,
            })
        })
        .collect()
}

/// `nanos` (`telemetryWire.ts:41`).
fn nanos(ms: f64) -> String {
    format!(
        "{}000000",
        js_number_to_string(crate::identity::js::js_round(ms))
    )
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A fixed-width run of ASCII digits at the front of `s`.
fn take_digits(s: &mut &str, width: usize) -> Option<i64> {
    let head = s.get(..width)?;
    if !head.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    *s = &s[width..];
    head.parse().ok()
}

fn take_char(s: &mut &str, c: char) -> bool {
    match s.strip_prefix(c) {
        Some(rest) => {
            *s = rest;
            true
        }
        None => false,
    }
}

/// `Date.parse` for the ECMAScript date-time string format (`YYYY-MM-DDTHH:mm:ss.sssZ`
/// and its shorter forms); `None` for anything else.
///
/// shortcut: V8's legacy formats ("Jan 1 2020", "2020/01/01") give `None`, and a date-time
/// with no offset counts as UTC, not local time; upgrade when an event sender writes these.
pub fn parse_date(text: &str) -> Option<f64> {
    let mut s = text;
    let year = if take_char(&mut s, '+') {
        take_digits(&mut s, 6)?
    } else if take_char(&mut s, '-') {
        let y = take_digits(&mut s, 6)?;
        if y == 0 {
            return None;
        }
        -y
    } else {
        take_digits(&mut s, 4)?
    };
    let (mut month, mut day) = (1, 1);
    if take_char(&mut s, '-') {
        month = take_digits(&mut s, 2)?;
        if take_char(&mut s, '-') {
            day = take_digits(&mut s, 2)?;
        }
    }
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return None;
    }
    let (mut hour, mut minute, mut second, mut millis, mut offset) = (0, 0, 0, 0, 0);
    if take_char(&mut s, 'T') {
        hour = take_digits(&mut s, 2)?;
        if !take_char(&mut s, ':') {
            return None;
        }
        minute = take_digits(&mut s, 2)?;
        if take_char(&mut s, ':') {
            second = take_digits(&mut s, 2)?;
            if take_char(&mut s, '.') {
                let len = s.bytes().take_while(u8::is_ascii_digit).count();
                if len == 0 {
                    return None;
                }
                let fraction = format!("{:0<3}", &s[..len.min(3)]);
                millis = fraction.parse().ok()?;
                s = &s[len..];
            }
        }
        if take_char(&mut s, 'Z') {
        } else if s.starts_with(['+', '-']) {
            let sign = if take_char(&mut s, '+') { 1 } else { -1 };
            if sign < 0 {
                s = &s[1..];
            }
            let oh = take_digits(&mut s, 2)?;
            if !take_char(&mut s, ':') {
                return None;
            }
            let om = take_digits(&mut s, 2)?;
            if oh > 23 || om > 59 {
                return None;
            }
            offset = sign * (oh * 60 + om);
        }
        let valid_time = hour < 24 && minute < 60 && second < 60;
        if !(valid_time || (hour == 24 && minute == 0 && second == 0 && millis == 0)) {
            return None;
        }
    }
    if !s.is_empty() {
        return None;
    }
    let minutes = days_from_civil(year, month, day) * 1440 + hour * 60 + minute - offset;
    let ms = (minutes * 60 + second) * 1000 + millis;
    (ms.abs() <= 8_640_000_000_000_000).then_some(ms as f64)
}

/// Port of `eventRecord` (`telemetryWire.ts:43`). `now_ms` is `Date.now()`, used when the
/// timestamp does not parse.
pub fn event_record(e: &QueuedEvent, now_ms: f64) -> OtlpLogRecord {
    let mut values = vec![
        ("event".to_owned(), Json::String(e.event.clone())),
        ("uuid".to_owned(), Json::String(e.uuid.clone())),
        (
            "distinct_id".to_owned(),
            Json::String(e.distinct_id.clone()),
        ),
    ];
    for (key, value) in &e.properties {
        set_entry(&mut values, key, value.clone());
    }
    OtlpLogRecord {
        time_unix_nano: nanos(parse_date(&e.timestamp).unwrap_or(now_ms)),
        severity_number: 9,
        severity_text: "INFO",
        body: e.event.clone(),
        attributes: otlp_attributes(&values),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Warn,
    Error,
}

/// Port of `logRecord` (`telemetryWire.ts:54`); `at_ms` is the `at` argument, `Date.now()` by default.
pub fn log_record(
    level: LogLevel,
    message: &str,
    attributes: &[(String, Json)],
    at_ms: f64,
) -> OtlpLogRecord {
    let error = level == LogLevel::Error;
    OtlpLogRecord {
        time_unix_nano: nanos(at_ms),
        severity_number: if error { 17 } else { 13 },
        severity_text: if error { "ERROR" } else { "WARN" },
        body: message.to_owned(),
        attributes: otlp_attributes(attributes),
    }
}

fn attribute_json(attributes: &[OtlpAttribute]) -> Json {
    let item = |a: &OtlpAttribute| {
        let value = match &a.value {
            OtlpValue::String(s) => ("stringValue", Json::String(s.clone())),
            OtlpValue::Int(s) => ("intValue", Json::String(s.clone())),
            OtlpValue::Double(n) => ("doubleValue", Json::Number(*n)),
            OtlpValue::Bool(b) => ("boolValue", Json::Bool(*b)),
        };
        Json::Object(vec![
            ("key".into(), Json::String(a.key.clone())),
            (
                "value".into(),
                Json::Object(vec![(value.0.into(), value.1)]),
            ),
        ])
    };
    Json::Array(attributes.iter().map(item).collect())
}

impl OtlpLogRecord {
    /// The record as the JSON value that `logs_request` sends.
    pub fn to_json(&self) -> Json {
        Json::Object(vec![
            (
                "timeUnixNano".into(),
                Json::String(self.time_unix_nano.clone()),
            ),
            (
                "severityNumber".into(),
                Json::Number(f64::from(self.severity_number)),
            ),
            (
                "severityText".into(),
                Json::String(self.severity_text.into()),
            ),
            (
                "body".into(),
                Json::Object(vec![(
                    "stringValue".into(),
                    Json::String(self.body.clone()),
                )]),
            ),
            ("attributes".into(), attribute_json(&self.attributes)),
        ])
    }
}

/// Port of `logsRequest` (`telemetryWire.ts:64`). The records pass through unchanged, as in
/// the original; build them with `OtlpLogRecord::to_json`.
pub fn logs_request(resource: &[(String, Json)], scope: &str, records: &[Json]) -> String {
    let scope_logs = Json::Object(vec![
        (
            "scope".into(),
            Json::Object(vec![("name".into(), Json::String(scope.into()))]),
        ),
        ("logRecords".into(), Json::Array(records.to_vec())),
    ]);
    let resource_logs = Json::Object(vec![
        (
            "resource".into(),
            Json::Object(vec![(
                "attributes".into(),
                attribute_json(&otlp_attributes(resource)),
            )]),
        ),
        ("scopeLogs".into(), Json::Array(vec![scope_logs])),
    ]);
    Json::Object(vec![(
        "resourceLogs".into(),
        Json::Array(vec![resource_logs]),
    )])
    .stringify()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Sent,
    Drop,
    Split,
    Retry,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::Drop => "drop",
            Self::Split => "split",
            Self::Retry => "retry",
        }
    }
}

/// Port of `outcome` (`telemetryWire.ts:72`).
pub fn outcome(status: f64, count: f64) -> Outcome {
    if (200.0..300.0).contains(&status) {
        Outcome::Sent
    } else if status == 400.0 {
        Outcome::Drop
    } else if status == 413.0 {
        if count > 1.0 {
            Outcome::Split
        } else {
            Outcome::Drop
        }
    } else {
        Outcome::Retry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_text_follows_javascript() {
        for (x, want) in [
            (1e21, "1e+21"),
            (123456789012345680000.0, "123456789012345680000"),
            (1.5e-7, "1.5e-7"),
            (0.000001, "0.000001"),
            (-0.5, "-0.5"),
            (100.0, "100"),
            (f64::MAX, "1.7976931348623157e+308"),
        ] {
            assert_eq!(js_number_to_string(x), want);
        }
    }

    #[test]
    fn dates_parse_as_iso_and_reject_the_rest() {
        assert_eq!(parse_date("1970-01-01T00:00:01.5Z"), Some(1500.0));
        assert_eq!(parse_date("2024-02-29"), Some(1_709_164_800_000.0));
        assert_eq!(
            parse_date("2024-01-01T00:00:00+01:00"),
            Some(1_704_063_600_000.0)
        );
        assert_eq!(parse_date("2023-02-29"), None);
        assert_eq!(parse_date("2024-01-01T24:00:01Z"), None);
        assert_eq!(parse_date("yesterday"), None);
    }

    #[test]
    fn object_keys_put_array_indexes_first() {
        let o = Json::Object(vec![
            ("b".into(), Json::Null),
            ("10".into(), Json::Bool(true)),
            ("2".into(), Json::Null),
        ]);
        assert_eq!(o.stringify(), r#"{"2":null,"10":true,"b":null}"#);
    }
}
