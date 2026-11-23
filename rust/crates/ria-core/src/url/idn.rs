// Ported from ardia packages/core/src/idn.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Punycode, IDN display of hostnames and homograph spoof checks (UTS 39 and the Chromium rules).

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

const BASE: f64 = 36.0;
const T_MIN: f64 = 1.0;
const T_MAX: f64 = 26.0;
const SKEW: f64 = 38.0;
const DAMP: f64 = 700.0;
const INITIAL_BIAS: f64 = 72.0;
const INITIAL_N: f64 = 128.0;
const MAX_CODE_POINT: i64 = 0x10ffff;

// The tables below are the regex sources and constants of idn.ts, with `\uXXXX` written `\x{XXXX}`.
const IDENTIFIER: &str = r#"^[\p{Ll}\p{Lo}\p{Lm}\p{Mn}\p{Mc}\p{Nd}-]$"#;
const EXCLUDED: &str = r#"[\x{0138}\x{0149}\x{017f}\x{01c4}-\x{01cc}\x{01dd}\x{0180}-\x{019f}\x{01a2}-\x{01ae}\x{01b1}-\x{01cc}\x{01f1}-\x{01f3}\x{01f6}\x{01f7}\x{021c}\x{021d}\x{0220}-\x{0225}\x{0234}-\x{024f}\x{0250}-\x{0258}\x{025a}-\x{02ff}\x{0305}\x{030d}\x{030e}\x{0310}\x{0312}\x{0315}-\x{031a}\x{031c}-\x{0322}\x{0329}-\x{032c}\x{032f}\x{0332}-\x{0334}\x{0336}\x{0337}\x{033a}-\x{036f}\x{0370}-\x{038f}\x{03cf}-\x{03ff}\x{1f00}-\x{1fff}\x{0482}-\x{0489}\x{1d00}-\x{1dbf}\x{1dc0}-\x{1dff}\x{2c60}-\x{2c7f}\x{a720}-\x{a7ff}\x{ab30}-\x{ab6f}\x{10a0}-\x{10cf}\x{1c90}-\x{1cbf}\x{0964}\x{0965}\x{0970}\x{0e2f}\x{0e46}\x{0eaf}\x{0ec6}\x{0f01}-\x{0f1f}\x{0f2a}-\x{0f3f}\x{104a}-\x{104f}\x{17d4}-\x{17d6}\x{17d8}-\x{17db}\x{3031}-\x{3035}\x{303b}\x{303c}\x{3099}\x{309a}\x{309b}\x{309c}\x{309f}\x{30a0}\x{30ff}\x{3131}-\x{318e}]"#;
const ALLOWED_COMMON: &str = r#"^[-\x{00b7}\x{30fb}\x{30fc}\x{3005}-\x{3007}\x{0300}-\x{0304}\x{0306}-\x{030c}\x{030f}\x{0311}\x{0313}\x{0314}\x{031b}\x{0323}-\x{0328}\x{032d}\x{032e}\x{0330}\x{0331}\x{0335}\x{0338}\x{0339}]$"#;
const DEVIATIONS: &str = r#"[\x{00df}\x{03c2}\x{200c}\x{200d}]"#;
const DIGIT_LOOKALIKES: &str = r#"[θ२২੨૨೩೭Շзҙӡउওਤ੩૩౩ဒვპ੫丩ㄐճ৪੪୫૭୨౨]"#;
const KANA_EXCEPTIONS: &str = r#"[\x{3078}-\x{307a}\x{30d8}-\x{30da}\x{30fb}-\x{30fe}]"#;
const COMBINING_DIACRITICS: &str = r#"[\x{0300}-\x{0339}]"#;
const LGC_OR_ASCII: &str =
    r#"^[\p{Script=Latin}\p{Script=Greek}\p{Script=Cyrillic}0-9._\x{0300}-\x{0339}-]*$"#;
const DANGEROUS: &str = r#"[^\p{Script_Extensions=Katakana}\p{Script_Extensions=Hiragana}\p{Script_Extensions=Han}][\x{30ce}\x{30f3}\x{30bd}\x{30be}\x{4e36}\x{4e40}\x{4e41}\x{4e3f}]|[\x{30ce}\x{30f3}\x{30bd}\x{30be}\x{4e36}\x{4e40}\x{4e41}\x{4e3f}][^\p{Script_Extensions=Katakana}\p{Script_Extensions=Hiragana}\p{Script_Extensions=Han}]|^[\p{Script_Extensions=Katakana}]+[\x{3078}-\x{307a}][\p{Script_Extensions=Katakana}]+$|^[\p{Script_Extensions=Hiragana}]+[\x{30d8}-\x{30da}][\p{Script_Extensions=Hiragana}]+$|[^\p{Script_Extensions=Katakana}\p{Script_Extensions=Hiragana}]\x{30fc}|^\x{30fc}|[^\p{Script_Extensions=Katakana}][\x{30fd}\x{30fe}]|^[\x{30fd}\x{30fe}]|[a-z]\x{30fb}|\x{30fb}[a-z]|[^\p{Script_Extensions=Han}]\x{4e00}|\x{4e00}[^\p{Script_Extensions=Han}]|[^\p{Script_Extensions=Bopomofo}]\x{3127}|\x{3127}[^\p{Script_Extensions=Bopomofo}]|[^\p{Script_Extensions=Latin}\p{Script_Extensions=Greek}\p{Script_Extensions=Cyrillic}][\x{0300}-\x{0339}]|\x{0131}[\x{0300}-\x{0339}]|[ijl\x{0131}]\x{0307}|[a-z][\x{0585}\x{0581}]|[\x{0585}\x{0581}][a-z]|[\p{Script_Extensions=Armenian}][og]|[og][\p{Script_Extensions=Armenian}]|[^\p{Script_Extensions=Arabic}][\x{064b}-\x{0652}\x{0670}]|[^\p{Script_Extensions=Hebrew}]\x{05b4}"#;

const WHOLE_SCRIPT_CONFUSABLES: &[(&str, &str, &[&str])] = &[
    (r#"\p{Script=Armenian}"#, r#"[ագզէլհյնոսւօՙ]"#, &["am"]),
    (
        r#"\p{Script=Cyrillic}"#,
        r#"[асԁеһіјӏорԗԛѕԝхуъЬҽпгѵѡ]"#,
        &["bg", "by", "kz", "pyc", "ru", "su", "ua", "uz"],
    ),
    (
        r#"\p{Script=Ethiopic}"#,
        r#"[ሀሁሃሠሡሰሱሲቀቁበቡነኑኒከኮዐዑዕዖፀፁፐ]"#,
        &["er", "et"],
    ),
    (r#"\p{Script=Georgian}"#, r#"[იოყჽ]"#, &["ge"]),
    (r#"\p{Script=Greek}"#, r#"[αικνρυωϲοτχ]"#, &["gr"]),
    (r#"\p{Script=Hebrew}"#, r#"[דוחיןסם]"#, &["il"]),
    (r#"\p{Script=Khmer}"#, r#"[ឧ]"#, &["kh"]),
    (r#"\p{Script=Lao}"#, r#"[ບປຍຮຣຨຫອພຟຝ໐]"#, &["la"]),
    (r#"\p{Script=Sinhala}"#, r#"[ථ]"#, &["lk"]),
    (r#"\p{Script=Thai}"#, r#"[ทนบพรหเแ๐ดลปฟม]"#, &["th"]),
];

const TOP_DOMAINS: &str = "google.com youtube.com facebook.com instagram.com twitter.com x.com whatsapp.com wikipedia.org amazon.com apple.com icloud.com microsoft.com live.com outlook.com office.com bing.com linkedin.com netflix.com yahoo.com reddit.com tiktok.com github.com gitlab.com paypal.com ebay.com dropbox.com adobe.com spotify.com twitch.tv discord.com slack.com zoom.us notion.so figma.com openai.com chatgpt.com anthropic.com claude.ai stripe.com shopify.com coinbase.com binance.com kraken.com blockchain.com metamask.io chase.com bankofamerica.com wellsfargo.com citi.com americanexpress.com capitalone.com hsbc.com barclays.co.uk santander.com revolut.com wise.com venmo.com cash.app booking.com airbnb.com expedia.com uber.com lyft.com doordash.com walmart.com target.com bestbuy.com costco.com etsy.com aliexpress.com alibaba.com temu.com shein.com baidu.com qq.com weibo.com yandex.ru vk.com mail.ru ok.ru naver.com kakao.com line.me rakuten.co.jp yahoo.co.jp amazon.co.jp amazon.co.uk amazon.de amazon.fr amazon.in bbc.co.uk bbc.com cnn.com nytimes.com theguardian.com washingtonpost.com wsj.com bloomberg.com reuters.com forbes.com medium.com substack.com wordpress.com blogger.com tumblr.com pinterest.com quora.com stackoverflow.com imdb.com steampowered.com steamcommunity.com epicgames.com roblox.com playstation.com xbox.com nintendo.com ea.com battle.net salesforce.com atlassian.com trello.com asana.com canva.com docusign.com okta.com cloudflare.com godaddy.com squarespace.com wix.com mozilla.org duckduckgo.com proton.me protonmail.com gmail.com googledrive.com fedex.com ups.com usps.com dhl.com fidelity.com schwab.com vanguard.com robinhood.com etrade.com morganstanley.com goldmansachs.com jpmorgan.com usbank.com pnc.com truist.com tdbank.com citibank.com discover.com ally.com sofi.com chime.com usaa.com navyfederal.org ameriprise.com edwardjones.com merrilledge.com interactivebrokers.com webull.com intuit.com turbotax.com creditkarma.com experian.com equifax.com transunion.com zellepay.com klarna.com affirm.com gemini.com crypto.com bitstamp.net okx.com bybit.com kucoin.com ledger.com trezor.io irs.gov ssa.gov gov.uk";

const CONFUSABLES: &[(char, &str)] = &[
    ('0', "o"),
    ('1', "l"),
    ('ø', "o"),
    ('đ', "d"),
    ('ħ', "h"),
    ('ı', "i"),
    ('ł', "l"),
    ('ŧ', "t"),
    ('ð', "d"),
    ('þ', "p"),
    ('ə', "e"),
    ('ß', "ss"),
    ('œ', "oe"),
    ('æ', "ae"),
    ('а', "a"),
    ('в', "b"),
    ('е', "e"),
    ('з', "3"),
    ('и', "u"),
    ('к', "k"),
    ('м', "m"),
    ('н', "h"),
    ('о', "o"),
    ('п', "n"),
    ('р', "p"),
    ('с', "c"),
    ('т', "t"),
    ('у', "y"),
    ('х', "x"),
    ('ь', "b"),
    ('ѕ', "s"),
    ('і', "i"),
    ('ј', "j"),
    ('ԁ', "d"),
    ('һ', "h"),
    ('ӏ', "l"),
    ('ԛ', "q"),
    ('ԝ', "w"),
    ('ҽ', "e"),
    ('ѵ', "v"),
    ('ѡ', "w"),
    ('г', "r"),
    ('α', "a"),
    ('ι', "i"),
    ('κ', "k"),
    ('ν', "v"),
    ('ο', "o"),
    ('ρ', "p"),
    ('τ', "t"),
    ('υ', "u"),
    ('χ', "x"),
    ('ω', "w"),
    ('ϲ', "c"),
    ('օ', "o"),
    ('ս', "u"),
    ('հ', "h"),
    ('ց', "g"),
    ('ք', "p"),
    ('զ', "q"),
    ('ո', "n"),
    ('m', "rn"),
    ('w', "vv"),
];

const SCRIPTS: &[&str] = &[
    "Latin",
    "Greek",
    "Cyrillic",
    "Armenian",
    "Hebrew",
    "Arabic",
    "Thaana",
    "Devanagari",
    "Bengali",
    "Gurmukhi",
    "Gujarati",
    "Oriya",
    "Tamil",
    "Telugu",
    "Kannada",
    "Malayalam",
    "Sinhala",
    "Thai",
    "Lao",
    "Tibetan",
    "Myanmar",
    "Georgian",
    "Hangul",
    "Ethiopic",
    "Khmer",
    "Hiragana",
    "Katakana",
    "Bopomofo",
    "Han",
];

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static regex")
}

static IDENTIFIER_RE: LazyLock<Regex> = LazyLock::new(|| re(IDENTIFIER));
static EXCLUDED_RE: LazyLock<Regex> = LazyLock::new(|| re(EXCLUDED));
static ALLOWED_COMMON_RE: LazyLock<Regex> = LazyLock::new(|| re(ALLOWED_COMMON));
static DEVIATIONS_RE: LazyLock<Regex> = LazyLock::new(|| re(DEVIATIONS));
static DIGIT_LOOKALIKES_RE: LazyLock<Regex> = LazyLock::new(|| re(DIGIT_LOOKALIKES));
static KANA_EXCEPTIONS_RE: LazyLock<Regex> = LazyLock::new(|| re(KANA_EXCEPTIONS));
static COMBINING_DIACRITICS_RE: LazyLock<Regex> = LazyLock::new(|| re(COMBINING_DIACRITICS));
static LGC_OR_ASCII_RE: LazyLock<Regex> = LazyLock::new(|| re(LGC_OR_ASCII));
static DANGEROUS_RE: LazyLock<Regex> = LazyLock::new(|| re(DANGEROUS));
/// `(?![a-z])\p{Script=Latin}`.
static NON_ASCII_LATIN_RE: LazyLock<Regex> = LazyLock::new(|| re(r"[\p{Script=Latin}&&[^a-z]]"));
static COMMON_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"[\p{Script=Common}\p{Script=Inherited}]"));
static MARK_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\p{M}"));
static DECIMAL_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\p{Nd}"));
static MIDDLE_DOT_RE: LazyLock<Regex> = LazyLock::new(|| re(r"[^l]·|·[^l]|^·|·$"));
/// One test for each of `SCRIPTS`, on `\p{Script_Extensions=name}`.
static SCRIPT_TESTS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    SCRIPTS
        .iter()
        .map(|name| re(&format!(r"\p{{Script_Extensions={name}}}")))
        .collect()
});
static WHOLE_SCRIPTS: LazyLock<Vec<(Regex, Regex, &'static [&'static str])>> =
    LazyLock::new(|| {
        WHOLE_SCRIPT_CONFUSABLES
            .iter()
            .map(|(s, l, tlds)| (re(s), re(l), *tlds))
            .collect()
    });
static TOP_SKELETONS: LazyLock<HashMap<String, &'static str>> =
    LazyLock::new(|| TOP_DOMAINS.split(' ').map(|d| (skeleton(d), d)).collect());

fn matches_char(regex: &Regex, c: char) -> bool {
    regex.is_match(c.encode_utf8(&mut [0; 4]))
}

fn to_int32(x: f64) -> i32 {
    x as i64 as i32
}

fn adapt(delta: f64, points: f64, first: bool) -> f64 {
    let mut delta = if first {
        (delta / DAMP).floor()
    } else {
        f64::from(to_int32(delta) >> 1)
    };
    delta += (delta / points).floor();
    let mut k = 0.0;
    while delta > f64::from(((BASE - T_MIN) as i32 * T_MAX as i32) >> 1) {
        delta = (delta / (BASE - T_MIN)).floor();
        k += BASE;
    }
    k + (((BASE - T_MIN + 1.0) * delta) / (delta + SKEW)).floor()
}

fn digit_value(c: u16) -> i64 {
    match c {
        0x30..=0x39 => i64::from(c) - 22,
        0x41..=0x5a => i64::from(c) - 0x41,
        0x61..=0x7a => i64::from(c) - 0x61,
        _ => BASE as i64,
    }
}

fn digit_char(d: f64) -> char {
    char::from((if d < 26.0 { d + 97.0 } else { d + 22.0 }) as u8)
}

fn threshold(k: f64, bias: f64) -> f64 {
    if k <= bias {
        T_MIN
    } else if k >= bias + T_MAX {
        T_MAX
    } else {
        k - bias
    }
}

/// `punycodeDecode`: `None` where the original returns `null`.
fn punycode_decode(input: &str) -> Option<String> {
    let units: Vec<u16> = input.encode_utf16().collect();
    let delimiter = units
        .iter()
        .rposition(|&c| c == u16::from(b'-'))
        .map_or(-1, |p| p as i64);
    let mut output: Vec<u32> = Vec::new();
    for &c in &units[..delimiter.max(0) as usize] {
        if c >= 0x80 {
            return None;
        }
        output.push(u32::from(c));
    }
    let (mut n, mut bias, mut i) = (INITIAL_N as i64, INITIAL_BIAS, 0_i64);
    let mut index = if delimiter > 0 {
        delimiter as usize + 1
    } else {
        0
    };
    while index < units.len() {
        let old_i = i;
        let (mut w, mut k) = (1_i64, BASE);
        loop {
            let unit = *units.get(index)?;
            index += 1;
            let digit = digit_value(unit);
            if digit >= BASE as i64 || digit > (MAX_CODE_POINT - i) / w {
                return None;
            }
            i += digit * w;
            let t = threshold(k, bias);
            if (digit as f64) < t {
                break;
            }
            w *= BASE as i64 - t as i64;
            k += BASE;
        }
        let points = output.len() as i64 + 1;
        bias = adapt((i - old_i) as f64, points as f64, old_i == 0);
        n += i / points;
        i %= points;
        if n > MAX_CODE_POINT || (0xd800..=0xdfff).contains(&n) {
            return None;
        }
        output.insert(i as usize, n as u32);
        i += 1;
    }
    output.into_iter().map(char::from_u32).collect()
}

/// Port of `punycodeEncode` (`idn.ts:54`).
pub fn punycode_encode(input: &str) -> String {
    let points: Vec<u32> = input.chars().map(u32::from).collect();
    let mut output: String = points
        .iter()
        .filter(|&&c| c < 0x80)
        .map(|&c| char::from(c as u8))
        .collect();
    let basic = output.len();
    let mut handled = basic;
    if basic > 0 {
        output.push('-');
    }
    let (mut n, mut delta, mut bias) = (INITIAL_N, 0.0_f64, INITIAL_BIAS);
    while handled < points.len() {
        let m = points
            .iter()
            .filter(|&&c| f64::from(c) >= n)
            .min()
            .map_or(f64::INFINITY, |&c| f64::from(c));
        delta += (m - n) * (handled as f64 + 1.0);
        n = m;
        for &c in &points {
            let c = f64::from(c);
            if c < n {
                delta += 1.0;
            }
            if c != n {
                continue;
            }
            let mut q = delta;
            let mut k = BASE;
            loop {
                let t = threshold(k, bias);
                if q < t {
                    break;
                }
                output.push(digit_char(t + (q - t) % (BASE - t)));
                q = ((q - t) / (BASE - t)).floor();
                k += BASE;
            }
            output.push(digit_char(q));
            bias = adapt(delta, handled as f64 + 1.0, handled == basic);
            delta = 0.0;
            handled += 1;
        }
        delta += 1.0;
        n += 1.0;
    }
    output
}

/// Bit of `SCRIPTS[i]`, then `Hanb`, `Jpan` and `Kore`.
const HANB: u64 = 1 << 29;
const JPAN: u64 = 1 << 30;
const KORE: u64 = 1 << 31;

fn script_bit(name: &str) -> u64 {
    1 << SCRIPTS
        .iter()
        .position(|s| *s == name)
        .expect("script name")
}

/// `scriptsOf`: the scripts of `c` (`Script_Extensions`) as a bit set; `None` when `c` has none.
fn scripts_of(c: char) -> Option<u64> {
    let mask = SCRIPT_TESTS
        .iter()
        .enumerate()
        .filter(|(_, re)| matches_char(re, c))
        .fold(0_u64, |acc, (i, _)| acc | 1 << i);
    if mask != 0 {
        Some(mask)
    } else {
        matches_char(&COMMON_RE, c).then_some(0)
    }
}

fn augmented(mut set: u64) -> u64 {
    let (han, hira, kata) = (
        script_bit("Han"),
        script_bit("Hiragana"),
        script_bit("Katakana"),
    );
    if set & han != 0 {
        set |= HANB | JPAN | KORE;
    }
    if set & (hira | kata) != 0 {
        set |= JPAN;
    }
    if set & script_bit("Hangul") != 0 {
        set |= KORE;
    }
    if set & script_bit("Bopomofo") != 0 {
        set |= HANB;
    }
    set
}

fn is_allowed(c: char) -> bool {
    if matches_char(&ALLOWED_COMMON_RE, c) {
        return true;
    }
    if !matches_char(&IDENTIFIER_RE, c) || matches_char(&EXCLUDED_RE, c) {
        return false;
    }
    let s = c.to_string();
    if s.nfkc().collect::<String>() != s || s.to_lowercase() != s {
        return false;
    }
    scripts_of(c).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Restriction {
    Ascii,
    Single,
    Highly,
    Moderately,
    Minimal,
}

/// Intersection of all sets; `None` for no sets.
fn intersect(sets: impl Iterator<Item = u64>) -> Option<u64> {
    sets.reduce(|a, b| a & b)
}

fn restriction_level(chars: &[char]) -> Restriction {
    if chars.iter().all(char::is_ascii) {
        return Restriction::Ascii;
    }
    let sets: Vec<u64> = chars
        .iter()
        .map(|&c| augmented(scripts_of(c).unwrap_or(0)))
        .filter(|&s| s != 0)
        .collect();
    match intersect(sets.iter().copied()) {
        Some(0) => {}
        _ => return Restriction::Single,
    }
    let latin = script_bit("Latin");
    let non_latin = match intersect(sets.iter().copied().filter(|s| s & latin == 0)) {
        Some(s) if s != 0 => s,
        _ => return Restriction::Minimal,
    };
    if non_latin & (HANB | JPAN | KORE) != 0 {
        Restriction::Highly
    } else if non_latin & (script_bit("Cyrillic") | script_bit("Greek")) == 0 {
        Restriction::Moderately
    } else {
        Restriction::Minimal
    }
}

/// The number of different digit systems (by the code point of their zero) in `chars`.
fn digit_systems(chars: &[char]) -> usize {
    let mut zeros: Vec<u32> = Vec::new();
    for &c in chars.iter().filter(|&&c| matches_char(&DECIMAL_RE, c)) {
        let cp = u32::from(c);
        let mut zero = cp;
        while zero > 0
            && char::from_u32(zero - 1).is_some_and(|p| matches_char(&DECIMAL_RE, p))
            && cp - zero < 9
        {
            zero -= 1;
        }
        if !zeros.contains(&zero) {
            zeros.push(zero);
        }
    }
    zeros.len()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpoofReason {
    Invalid,
    DisallowedCharacter,
    Invisible,
    MixedNumbers,
    MixedScripts,
    DeviationCharacter,
    TldSpecificCharacter,
    UnsafeMiddleDot,
    DigitLookalikes,
    WholeScriptConfusable,
    NonAsciiLatinMixed,
    DangerousPattern,
    TopDomainLookalike,
}

impl SpoofReason {
    /// The string of the TypeScript `SpoofReason` union.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::DisallowedCharacter => "disallowedCharacter",
            Self::Invisible => "invisible",
            Self::MixedNumbers => "mixedNumbers",
            Self::MixedScripts => "mixedScripts",
            Self::DeviationCharacter => "deviationCharacter",
            Self::TldSpecificCharacter => "tldSpecificCharacter",
            Self::UnsafeMiddleDot => "unsafeMiddleDot",
            Self::DigitLookalikes => "digitLookalikes",
            Self::WholeScriptConfusable => "wholeScriptConfusable",
            Self::NonAsciiLatinMixed => "nonAsciiLatinMixed",
            Self::DangerousPattern => "dangerousPattern",
            Self::TopDomainLookalike => "topDomainLookalike",
        }
    }
}

fn label_spoof_reason(label: &str, tld: &str) -> Option<SpoofReason> {
    use SpoofReason::*;
    let chars: Vec<char> = label.chars().collect();
    if chars.is_empty() {
        return Some(Invalid);
    }
    if !chars.iter().all(|&c| is_allowed(c)) {
        return Some(DisallowedCharacter);
    }
    if chars
        .windows(2)
        .any(|w| w[0] == w[1] && matches_char(&MARK_RE, w[1]))
    {
        return Some(Invisible);
    }
    if digit_systems(&chars) > 1 {
        return Some(MixedNumbers);
    }
    let level = restriction_level(&chars);
    if level == Restriction::Minimal {
        return Some(MixedScripts);
    }
    if DEVIATIONS_RE.is_match(label) {
        return Some(DeviationCharacter);
    }
    if chars.len() > 1 && tld != "is" && label.contains(['þ', 'ð']) {
        return Some(TldSpecificCharacter);
    }
    if chars.len() > 1 && tld != "az" && label.contains('ə') {
        return Some(TldSpecificCharacter);
    }
    if label.contains('·') && (tld != "cat" || MIDDLE_DOT_RE.is_match(label)) {
        return Some(UnsafeMiddleDot);
    }
    if level == Restriction::Ascii {
        return None;
    }
    let lookalike = |c: char| matches_char(&DIGIT_LOOKALIKES_RE, c);
    if DIGIT_LOOKALIKES_RE.is_match(label)
        && chars.iter().all(|&c| c.is_ascii_digit() || lookalike(c))
    {
        return Some(DigitLookalikes);
    }
    if level == Restriction::Single
        && !KANA_EXCEPTIONS_RE.is_match(label)
        && !COMBINING_DIACRITICS_RE.is_match(label)
    {
        let unicode_tld = decode_label(tld).unwrap_or_else(|| tld.to_string());
        for (script, lookalikes, tlds) in WHOLE_SCRIPTS.iter() {
            let mut letters = chars
                .iter()
                .filter(|&&c| matches_char(script, c))
                .peekable();
            if letters.peek().is_none() || !letters.all(|&c| matches_char(lookalikes, c)) {
                continue;
            }
            let tld_uses_script = unicode_tld.chars().all(|c| matches_char(script, c));
            if !tlds.contains(&tld) && !tld_uses_script {
                return Some(WholeScriptConfusable);
            }
        }
        return None;
    }
    if NON_ASCII_LATIN_RE.is_match(label) && !LGC_OR_ASCII_RE.is_match(label) {
        return Some(NonAsciiLatinMixed);
    }
    if DANGEROUS_RE.is_match(label) {
        return Some(DangerousPattern);
    }
    None
}

/// `decodeLabel`: the Unicode form of an `xn--` label that round-trips, else `None`.
fn decode_label(label: &str) -> Option<String> {
    if !label
        .get(..4)
        .is_some_and(|p| p.eq_ignore_ascii_case("xn--"))
    {
        return None;
    }
    let ascii = label[4..].to_lowercase();
    let unicode = punycode_decode(&ascii)?;
    if unicode.is_ascii() {
        return None;
    }
    if unicode.nfc().collect::<String>() != unicode
        || unicode.to_lowercase() != unicode
        || punycode_encode(&unicode) != ascii
    {
        return None;
    }
    Some(unicode)
}

fn skeleton(text: &str) -> String {
    let stripped: String = text.nfd().filter(|&c| !matches_char(&MARK_RE, c)).collect();
    stripped
        .to_lowercase()
        .chars()
        .map(|c| match CONFUSABLES.iter().rfind(|(k, _)| *k == c) {
            Some((_, to)) => (*to).to_string(),
            None => c.to_string(),
        })
        .collect()
}

fn similar_top_domain(host: &str) -> Option<&'static str> {
    let bare = host.strip_prefix("www.").unwrap_or(host);
    let bare = bare.strip_suffix('.').unwrap_or(bare);
    let labels: Vec<&str> = bare.split('.').collect();
    let tail = |n: usize| labels[labels.len().saturating_sub(n)..].join(".");
    [bare.to_string(), tail(2), tail(3)]
        .iter()
        .find_map(|candidate| {
            TOP_SKELETONS
                .get(&skeleton(candidate))
                .copied()
                .filter(|m| m != candidate)
        })
}

/// `/(^|\.)xn--/` on `host`, with or without case.
fn has_xn_label(host: &str, ignore_case: bool) -> bool {
    host.split('.').any(|l| {
        l.get(..4).is_some_and(|p| {
            if ignore_case {
                p.eq_ignore_ascii_case("xn--")
            } else {
                p == "xn--"
            }
        })
    })
}

/// Port of `displayHost` (`idn.ts:319`).
///
/// shortcut: the original caches results in a `Map` of 500 entries; this recomputes.
pub fn display_host(host: &str) -> String {
    if !has_xn_label(host, true) {
        return host.to_string();
    }
    let lower = host.to_lowercase();
    let labels: Vec<&str> = lower.split('.').collect();
    let last = |n: usize| labels.len().checked_sub(n).map_or("", |i| labels[i]);
    let tld = [last(1), last(2)]
        .into_iter()
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let mut changed = false;
    let shown: Vec<String> = labels
        .iter()
        .map(|&label| match decode_label(label) {
            Some(unicode) if label_spoof_reason(&unicode, tld).is_none() => {
                changed = true;
                unicode
            }
            _ => label.to_string(),
        })
        .collect();
    if changed && similar_top_domain(&shown.join(".")).is_none() {
        shown.join(".")
    } else {
        host.to_string()
    }
}

/// Port of `hostSpoofReason` (`idn.ts:339`).
pub fn host_spoof_reason(host: &str) -> Option<SpoofReason> {
    let lower = host.to_lowercase();
    let labels: Vec<&str> = lower.split('.').collect();
    let tld = labels.last().copied().unwrap_or("");
    for label in labels.iter().filter(|l| l.starts_with("xn--")) {
        let Some(unicode) = decode_label(label) else {
            return Some(SpoofReason::Invalid);
        };
        if let Some(reason) = label_spoof_reason(&unicode, tld) {
            return Some(reason);
        }
    }
    (display_host(host) == host && has_xn_label(&lower, false))
        .then_some(SpoofReason::TopDomainLookalike)
}
