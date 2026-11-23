// Ria original, no Ardia source.

//! `Date.parse` as V8 does it (`src/date/dateparser*.{h,cc}`, `ParseDateTimeString` in
//! `builtins-date.cc`): the ECMAScript date-time format, then the legacy format that Safari
//! accepts. `eventRecord` (`telemetryWire.ts:43`) calls `Date.parse` on a stored string.
//!
//! shortcut: a string with no offset counts as UTC, where V8 uses the local time zone; upgrade
//! when a caller needs local time (Ardia writes only `toISOString()` text, with `Z`).

use crate::search::js_text::is_js_space;

const NONE: i64 = i32::MAX as i64;
/// V8 reads at most this many digits of a numeral.
const MAX_SIGNIFICANT_DIGITS: usize = 9;
const MAX_TIME_MS: f64 = 8.64e15;

fn between(x: i64, lo: i64, hi: i64) -> bool {
    (lo..=hi).contains(&x)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Keyword {
    Month,
    AmPm,
    Zone,
    TimeSeparator,
    Garbage,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Token {
    Invalid,
    Unknown,
    Number {
        n: i64,
        len: usize,
    },
    Symbol(char),
    WhiteSpace,
    Keyword {
        kind: Keyword,
        value: i64,
        len: usize,
    },
    End,
}

impl Token {
    fn is_symbol(self, c: char) -> bool {
        self == Token::Symbol(c)
    }

    fn is_sign(self) -> bool {
        self.is_symbol('+') || self.is_symbol('-')
    }

    fn sign(self) -> i64 {
        if self.is_symbol('+') { 1 } else { -1 }
    }

    fn is_number(self) -> bool {
        matches!(self, Token::Number { .. })
    }

    fn is_fixed_number(self, width: usize) -> bool {
        matches!(self, Token::Number { len, .. } if len == width)
    }

    fn number(self) -> i64 {
        match self {
            Token::Number { n, .. } => n,
            _ => 0,
        }
    }

    fn is_keyword_z(self) -> bool {
        matches!(
            self,
            Token::Keyword {
                kind: Keyword::Zone,
                len: 1,
                ..
            }
        )
    }
}

/// The keyword table: a word matches on its first 3 letters (the rest only for months).
const KEYWORDS: [(&str, Keyword, i64); 28] = [
    ("jan", Keyword::Month, 1),
    ("feb", Keyword::Month, 2),
    ("mar", Keyword::Month, 3),
    ("apr", Keyword::Month, 4),
    ("may", Keyword::Month, 5),
    ("jun", Keyword::Month, 6),
    ("jul", Keyword::Month, 7),
    ("aug", Keyword::Month, 8),
    ("sep", Keyword::Month, 9),
    ("oct", Keyword::Month, 10),
    ("nov", Keyword::Month, 11),
    ("dec", Keyword::Month, 12),
    ("am", Keyword::AmPm, 0),
    ("pm", Keyword::AmPm, 12),
    ("ut", Keyword::Zone, 0),
    ("utc", Keyword::Zone, 0),
    ("z", Keyword::Zone, 0),
    ("gmt", Keyword::Zone, 0),
    ("cdt", Keyword::Zone, -5),
    ("cst", Keyword::Zone, -6),
    ("edt", Keyword::Zone, -4),
    ("est", Keyword::Zone, -5),
    ("mdt", Keyword::Zone, -6),
    ("mst", Keyword::Zone, -7),
    ("pdt", Keyword::Zone, -7),
    ("pst", Keyword::Zone, -8),
    ("t", Keyword::TimeSeparator, 0),
    ("", Keyword::Garbage, 0),
];

fn lookup(prefix: &[u8; 3], len: usize) -> (Keyword, i64) {
    for (word, kind, value) in KEYWORDS {
        let mut padded = [0u8; 3];
        padded[..word.len()].copy_from_slice(word.as_bytes());
        if !word.is_empty() && &padded == prefix && (len <= 3 || kind == Keyword::Month) {
            return (kind, value);
        }
    }
    (Keyword::Garbage, 0)
}

/// V8's `DateStringTokenizer` over UTF-16 units; a NUL unit ends the input.
struct Scanner<'a> {
    units: &'a [u16],
    pos: usize,
    tok: Token,
}

impl<'a> Scanner<'a> {
    fn new(units: &'a [u16]) -> Self {
        let mut s = Scanner {
            units,
            pos: 0,
            tok: Token::End,
        };
        s.tok = s.scan();
        s
    }

    fn ch(&self) -> u16 {
        self.units.get(self.pos).copied().unwrap_or(0)
    }

    fn is_space(&self) -> bool {
        char::from_u32(u32::from(self.ch())).is_some_and(is_js_space)
    }

    fn is_word_char(&self) -> bool {
        self.ch() >= u16::from(b'A') && !self.is_space()
    }

    fn scan(&mut self) -> Token {
        let start = self.pos;
        let ch = self.ch();
        if ch == 0 {
            return Token::End;
        }
        if (u16::from(b'0')..=u16::from(b'9')).contains(&ch) {
            let (mut n, mut digits) = (0i64, 0usize);
            while self.ch() == u16::from(b'0') {
                self.pos += 1;
            }
            while (u16::from(b'0')..=u16::from(b'9')).contains(&self.ch()) {
                if digits < MAX_SIGNIFICANT_DIGITS {
                    n = n * 10 + i64::from(self.ch() - u16::from(b'0'));
                }
                digits += 1;
                self.pos += 1;
            }
            return Token::Number {
                n,
                len: self.pos - start,
            };
        }
        if let Some(c) = [':', '-', '+', '.', ')']
            .into_iter()
            .find(|&c| u16::from(c as u8) == ch)
        {
            self.pos += 1;
            return Token::Symbol(c);
        }
        if self.is_word_char() {
            let (mut prefix, mut len) = ([0u8; 3], 0usize);
            while self.is_word_char() {
                if len < 3 {
                    // `AsciiAlphaToLower`: a unit above 0xff never equals a table letter.
                    prefix[len] = u8::try_from(self.ch() | 0x20).unwrap_or(0xff);
                }
                self.pos += 1;
                len += 1;
            }
            let (kind, value) = lookup(&prefix, len);
            return Token::Keyword { kind, value, len };
        }
        if self.is_space() {
            self.pos += 1;
            return Token::WhiteSpace;
        }
        if ch == u16::from(b'(') {
            let mut balance = 0;
            loop {
                match self.ch() {
                    c if c == u16::from(b')') => balance -= 1,
                    c if c == u16::from(b'(') => balance += 1,
                    _ => {}
                }
                self.pos += 1;
                if balance <= 0 || self.ch() == 0 {
                    break;
                }
            }
            return Token::Unknown;
        }
        self.pos += 1;
        Token::Unknown
    }

    fn peek(&self) -> Token {
        self.tok
    }

    fn next(&mut self) -> Token {
        let token = self.tok;
        self.tok = self.scan();
        token
    }

    fn skip_symbol(&mut self, c: char) -> bool {
        if self.tok.is_symbol(c) {
            self.tok = self.scan();
            true
        } else {
            false
        }
    }
}

#[derive(Default)]
struct Day {
    comp: [i64; 3],
    index: usize,
    named_month: Option<i64>,
    iso: bool,
}

impl Day {
    fn add(&mut self, n: i64) -> bool {
        if self.index < 3 {
            self.comp[self.index] = n;
            self.index += 1;
            true
        } else {
            false
        }
    }

    /// `[year, month (0-based), day]`.
    fn write(&mut self) -> Option<[i64; 3]> {
        if self.index < 1 {
            return None;
        }
        // V8 fills the missing parts with 1 and then reads `index` as 3.
        while self.index < 3 {
            self.comp[self.index] = 1;
            self.index += 1;
        }
        let c = self.comp;
        let (mut year, month, day);
        if let Some(named) = self.named_month {
            month = named;
            if !between(c[0], 1, 31) {
                (year, day) = (c[0], c[1]);
            } else {
                (day, year) = (c[0], c[1]);
            }
        } else if self.iso || !between(c[0], 1, 31) {
            (year, month, day) = (c[0], c[1], c[2]);
        } else {
            (month, day, year) = (c[0], c[1], c[2]);
        }
        if !self.iso {
            if between(year, 0, 49) {
                year += 2000;
            } else if between(year, 50, 99) {
                year += 1900;
            }
        }
        (between(month, 1, 12) && between(day, 1, 31)).then_some([year, month - 1, day])
    }
}

struct Time {
    comp: [i64; 4],
    index: usize,
    hour_offset: i64,
}

impl Time {
    fn new() -> Self {
        Time {
            comp: [0; 4],
            index: 0,
            hour_offset: NONE,
        }
    }

    fn is_expecting(&self, n: i64) -> bool {
        (self.index == 1 && between(n, 0, 59))
            || (self.index == 2 && between(n, 0, 59))
            || (self.index == 3 && between(n, 0, 999))
    }

    fn add(&mut self, n: i64) -> bool {
        if self.index < 4 {
            self.comp[self.index] = n;
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn add_final(&mut self, n: i64) -> bool {
        if !self.add(n) {
            return false;
        }
        self.index = 4;
        true
    }

    /// `[hour, minute, second, millisecond]`.
    fn write(&mut self) -> Option<[i64; 4]> {
        self.index = 4;
        let [mut hour, minute, second, ms] = self.comp;
        if self.hour_offset != NONE {
            if !between(hour, 0, 12) {
                return None;
            }
            hour = hour % 12 + self.hour_offset;
        }
        let valid = between(hour, 0, 23)
            && between(minute, 0, 59)
            && between(second, 0, 59)
            && between(ms, 0, 999);
        (valid || (hour == 24 && minute == 0 && second == 0 && ms == 0))
            .then_some([hour, minute, second, ms])
    }
}

struct Zone {
    sign: i64,
    hour: i64,
    minute: i64,
}

impl Zone {
    fn new() -> Self {
        Zone {
            sign: NONE,
            hour: NONE,
            minute: NONE,
        }
    }

    fn set(&mut self, hours: i64) {
        self.sign = if hours < 0 { -1 } else { 1 };
        self.hour = hours * self.sign;
        self.minute = 0;
    }

    fn is_expecting(&self, n: i64) -> bool {
        self.hour != NONE && self.minute == NONE && between(n, 0, 59)
    }

    fn is_utc(&self) -> bool {
        self.hour == 0 && self.minute == 0
    }

    fn is_empty(&self) -> bool {
        self.hour == NONE
    }

    /// The offset in seconds; `None` for the local time zone.
    fn write(&self) -> Result<Option<i64>, ()> {
        if self.sign == NONE {
            return Ok(None);
        }
        let hour = if self.hour == NONE { 0 } else { self.hour };
        let minute = if self.minute == NONE { 0 } else { self.minute };
        // V8 adds in `unsigned`: 32 bits, wrapping; a Smi holds up to 2^31 - 1.
        let total = (hour as u32)
            .wrapping_mul(3600)
            .wrapping_add((minute as u32).wrapping_mul(60));
        if total > i32::MAX as u32 {
            return Err(());
        }
        Ok(Some(i64::from(total) * self.sign))
    }
}

/// `ReadMilliseconds`: the first three significant digits of a numeral.
fn milliseconds(n: i64, len: usize) -> i64 {
    match len {
        1 => n * 100,
        2 => n * 10,
        0..=3 => n,
        _ => {
            let digits = len.min(MAX_SIGNIFICANT_DIGITS);
            n / 10i64.pow((digits - 3) as u32)
        }
    }
}

/// `ParseES5DateTime`: the token the legacy parser starts with, or `Invalid`.
fn parse_es5(s: &mut Scanner, day: &mut Day, time: &mut Time, zone: &mut Zone) -> Token {
    if s.peek().is_sign() {
        let sign_token = s.next();
        if !s.peek().is_fixed_number(6) {
            return sign_token;
        }
        let year = s.next().number();
        if sign_token.sign() < 0 && year == 0 {
            return sign_token;
        }
        day.add(sign_token.sign() * year);
    } else if s.peek().is_fixed_number(4) {
        day.add(s.next().number());
    } else {
        return s.next();
    }
    if s.skip_symbol('-') {
        if !s.peek().is_fixed_number(2) || !between(s.peek().number(), 1, 12) {
            return s.next();
        }
        day.add(s.next().number());
        if s.skip_symbol('-') {
            if !s.peek().is_fixed_number(2) || !between(s.peek().number(), 1, 31) {
                return s.next();
            }
            day.add(s.next().number());
        }
    }
    let is_t = matches!(
        s.peek(),
        Token::Keyword {
            kind: Keyword::TimeSeparator,
            ..
        }
    );
    if !is_t {
        if s.peek() != Token::End {
            return s.next();
        }
    } else {
        s.next();
        let hour_is_24 = s.peek().number() == 24;
        let two =
            |s: &Scanner, hi: i64| s.peek().is_fixed_number(2) && between(s.peek().number(), 0, hi);
        if !two(s, 24) {
            return Token::Invalid;
        }
        time.add(s.next().number());
        if !s.skip_symbol(':') {
            return Token::Invalid;
        }
        if !two(s, 59) || (hour_is_24 && s.peek().number() > 0) {
            return Token::Invalid;
        }
        time.add(s.next().number());
        if s.skip_symbol(':') {
            if !two(s, 59) || (hour_is_24 && s.peek().number() > 0) {
                return Token::Invalid;
            }
            time.add(s.next().number());
            if s.skip_symbol('.') {
                if !s.peek().is_number() || (hour_is_24 && s.peek().number() > 0) {
                    return Token::Invalid;
                }
                let Token::Number { n, len } = s.next() else {
                    return Token::Invalid;
                };
                time.add(milliseconds(n, len));
            }
        }
        if s.peek().is_keyword_z() {
            s.next();
            zone.set(0);
        } else if s.peek().is_sign() {
            zone.sign = s.next().sign();
            if s.peek().is_fixed_number(4) {
                let hour_min = s.next().number();
                let (hour, min) = (hour_min / 100, hour_min % 100);
                if !between(hour, 0, 23) || !between(min, 0, 59) {
                    return Token::Invalid;
                }
                zone.hour = hour;
                zone.minute = min;
            } else {
                if !two(s, 23) {
                    return Token::Invalid;
                }
                zone.hour = s.next().number();
                if !s.skip_symbol(':') || !two(s, 59) {
                    return Token::Invalid;
                }
                zone.minute = s.next().number();
            }
        }
        if s.peek() != Token::End {
            return Token::Invalid;
        }
    }
    if zone.is_empty() && time.index == 0 {
        zone.set(0);
    }
    day.iso = true;
    Token::End
}

/// `DateParser::Parse` and `ParseDateTimeString`: the time value in ms, or `None` for NaN.
pub fn parse_date(text: &str) -> Option<f64> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut s = Scanner::new(&units);
    let (mut day, mut time, mut zone) = (Day::default(), Time::new(), Zone::new());
    let mut token = parse_es5(&mut s, &mut day, &mut time, &mut zone);
    if token == Token::Invalid {
        return None;
    }
    let mut has_number = day.index > 0;
    while token != Token::End {
        match token {
            Token::Number { n, .. } => {
                has_number = true;
                if s.skip_symbol(':') {
                    if s.skip_symbol(':') {
                        if time.index != 0 {
                            return None;
                        }
                        time.add(n);
                        time.add(0);
                    } else {
                        if !time.add(n) {
                            return None;
                        }
                        if s.peek().is_symbol('.') {
                            s.next();
                        }
                    }
                } else if s.skip_symbol('.') && time.is_expecting(n) {
                    time.add(n);
                    let Token::Number { n: ms, len } = s.peek() else {
                        return None;
                    };
                    s.next();
                    time.add_final(milliseconds(ms, len));
                } else if zone.is_expecting(n) {
                    zone.minute = n;
                } else if time.is_expecting(n) {
                    time.add_final(n);
                    let peek = s.peek();
                    if !(peek == Token::End
                        || peek == Token::WhiteSpace
                        || peek.is_keyword_z()
                        || peek.is_sign())
                    {
                        return None;
                    }
                } else {
                    if !day.add(n) {
                        return None;
                    }
                    s.skip_symbol('-');
                }
            }
            Token::Keyword { kind, value, .. } => match kind {
                Keyword::AmPm if time.index != 0 => time.hour_offset = value,
                Keyword::Month => {
                    day.named_month = Some(value);
                    s.skip_symbol('-');
                }
                Keyword::Zone if has_number => zone.set(value),
                _ => {
                    if has_number || s.peek().is_number() {
                        return None;
                    }
                }
            },
            t if t.is_sign() && (zone.is_utc() || time.index != 0) => {
                zone.sign = t.sign();
                let (mut n, mut len) = (0, 0);
                if let Token::Number { n: v, len: l } = s.peek() {
                    s.next();
                    (n, len) = (v, l);
                }
                has_number = true;
                if s.peek().is_symbol(':') {
                    zone.hour = n;
                    zone.minute = NONE;
                } else if len == 1 || len == 2 {
                    zone.hour = n;
                    zone.minute = 0;
                } else if len == 3 || len == 4 {
                    zone.hour = n / 100;
                    zone.minute = n % 100;
                } else {
                    return None;
                }
            }
            t if (t.is_sign() || t.is_symbol(')')) && has_number => return None,
            _ => {}
        }
        token = s.next();
    }
    let [year, month, d] = day.write()?;
    let [hour, minute, second, ms] = time.write()?;
    let offset = zone.write().ok()?;
    if !(-1_000_000..=1_000_000).contains(&year) {
        return None;
    }
    let days = days_from_civil(year, month + 1, d) as f64;
    let clock =
        hour as f64 * 3_600_000.0 + minute as f64 * 60_000.0 + second as f64 * 1000.0 + ms as f64;
    let mut date = days * 86_400_000.0 + clock;
    match offset {
        None if date.abs() > MAX_TIME_MS + 86_400_000.0 => return None,
        None => {}
        Some(seconds) => date -= seconds as f64 * 1000.0,
    }
    (date.abs() <= MAX_TIME_MS).then_some(date)
}

/// Days from 1970-01-01 to a proleptic Gregorian date; `day` may overflow its month, as V8's
/// `MakeDay` lets it.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

#[cfg(test)]
mod tests {
    use super::parse_date;

    #[test]
    fn iso_and_legacy_forms() {
        assert_eq!(
            parse_date("2026-11-14T10:00:00.000Z"),
            Some(1_794_650_400_000.0)
        );
        assert_eq!(parse_date("2001-01-01"), Some(978_307_200_000.0));
        // V8 reads the 1 as the year 2001 and defaults month and day to 1.
        assert_eq!(parse_date("key_name-1"), Some(978_307_200_000.0));
        assert_eq!(
            parse_date("Jan 5 2020 3:04 PM GMT+0100"),
            Some(1_578_233_040_000.0)
        );
        assert_eq!(parse_date("2026-13-01"), None);
        // V8 does not check the length of the month.
        assert_eq!(
            parse_date("2026-02-30T00:00:00Z"),
            Some(1_772_409_600_000.0)
        );
        assert_eq!(parse_date(""), None);
    }
}
