// Ported from ardia packages/core/src/cleanUrl.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Tracking-parameter removal from http(s) URLs.

use ::url::Url;

const GLOBAL_PARAMS: &[&str] = &[
    "gclid",
    "gclsrc",
    "dclid",
    "gbraid",
    "wbraid",
    "gad_source",
    "gad_campaignid",
    "_ga",
    "_gl",
    "srsltid",
    "fbclid",
    "igshid",
    "msclkid",
    "twclid",
    "ttclid",
    "li_fat_id",
    "yclid",
    "ysclid",
    "rdt_cid",
    "epik",
    "mc_cid",
    "mc_eid",
    "_hsenc",
    "_hsmi",
    "__hssc",
    "__hstc",
    "__hsfp",
    "hsctatracking",
    "mkt_tok",
    "elqtrackid",
    "elqtrack",
    "vero_id",
    "vero_conv",
    "_kx",
    "__s",
    "oly_anon_id",
    "oly_enc_id",
    "_openstat",
    "wickedid",
    "rb_clickid",
    "s_cid",
    "irclickid",
    "cjevent",
    "sscid",
    "zanpid",
    "ranmid",
    "raneaid",
    "ransiteid",
    "trk_contact",
    "trk_msg",
    "trk_module",
    "trk_sid",
];

const GLOBAL_PREFIXES: &[&str] = &["utm_", "pk_", "mtm_", "piwik_", "matomo_", "hsa_", "stm_"];

struct SiteRule {
    hosts: &'static [&'static str],
    params: &'static [&'static str],
    prefixes: &'static [&'static str],
}

const SITE_PARAMS: &[SiteRule] = &[
    SiteRule {
        hosts: &["youtube.com", "youtu.be", "music.youtube.com"],
        params: &["si", "pp", "feature"],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["open.spotify.com"],
        params: &["si", "nd", "context"],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["instagram.com"],
        params: &["igsh", "img_index"],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["x.com", "twitter.com"],
        params: &["s", "t", "ref_src", "ref_url"],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["threads.net", "threads.com"],
        params: &["xmt", "slof"],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["tiktok.com"],
        params: &[
            "is_from_webapp",
            "sender_device",
            "sender_web_id",
            "_r",
            "_t",
        ],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["linkedin.com"],
        params: &[
            "trk",
            "trackingid",
            "lipi",
            "midtoken",
            "midsig",
            "trkemail",
            "eid",
        ],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["reddit.com"],
        params: &["share_id", "ref_source", "ref_campaign"],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["medium.com"],
        params: &["source"],
        prefixes: &[],
    },
    SiteRule {
        hosts: &["facebook.com"],
        params: &["mibextid", "sfnsn", "rdid", "share_url"],
        prefixes: &[],
    },
    SiteRule {
        hosts: &[
            "amazon.com",
            "amazon.co.uk",
            "amazon.de",
            "amazon.fr",
            "amazon.it",
            "amazon.es",
            "amazon.ca",
            "amazon.co.jp",
            "amazon.in",
            "amazon.com.au",
        ],
        params: &[
            "_encoding",
            "psc",
            "ref",
            "ref_",
            "tag",
            "linkcode",
            "linkid",
            "camp",
            "creative",
            "creativeasin",
            "content-id",
            "crid",
            "sprefix",
            "qid",
            "sr",
            "dib",
            "dib_tag",
        ],
        prefixes: &["pd_rd_", "pf_rd_"],
    },
];

/// The hostname rules of `siteRules`: `host` is lowercase, without a leading `www.`.
fn site_rules(hostname: &str) -> impl Iterator<Item = &'static SiteRule> + '_ {
    let lower = hostname.to_lowercase();
    let host = lower.strip_prefix("www.").unwrap_or(&lower).to_string();
    SITE_PARAMS.iter().filter(move |r| {
        r.hosts
            .iter()
            .any(|h| host == *h || host.strip_suffix(h).is_some_and(|p| p.ends_with('.')))
    })
}

/// JS `decodeURIComponent`: `None` where it throws (a bad `%` escape or invalid UTF-8).
fn decode_uri_component(raw: &str) -> Option<String> {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = b.get(i + 1..i + 3)?;
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            out.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn decode_name(raw: &str) -> String {
    decode_uri_component(&raw.replace('+', " "))
        .map_or_else(|| raw.to_lowercase(), |s| s.to_lowercase())
}

fn is_tracking_param(name: &str, hostname: &str) -> bool {
    let key = name.to_lowercase();
    if GLOBAL_PARAMS.contains(&key.as_str()) || GLOBAL_PREFIXES.iter().any(|p| key.starts_with(p)) {
        return true;
    }
    site_rules(hostname)
        .any(|r| r.params.contains(&key.as_str()) || r.prefixes.iter().any(|p| key.starts_with(p)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanedUrl {
    pub url: String,
    pub removed_tracker: bool,
}

/// Port of `cleanUrlReport` (`cleanUrl.ts:52`).
pub fn clean_url_report(url: &str) -> CleanedUrl {
    let unchanged = || CleanedUrl {
        url: url.to_string(),
        removed_tracker: false,
    };
    // `/^([^?#]*)(\?[^#]*)?(#.*)?$/`: base, then an optional query, then an optional
    // fragment without line terminators; no match, or no query, leaves the URL as it is.
    let base_len = url.find(['?', '#']).unwrap_or(url.len());
    let Some(after) = url[base_len..].strip_prefix('?') else {
        return unchanged();
    };
    let query_len = after.find('#').unwrap_or(after.len());
    let (query, hash) = after.split_at(query_len);
    if hash.contains(super::is_line_terminator) {
        return unchanged();
    }
    let base = &url[..base_len];
    let Ok(parsed) = Url::parse(url) else {
        return unchanged();
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return unchanged();
    }
    let hostname = parsed.host_str().unwrap_or("");
    let pairs: Vec<&str> = query.split('&').collect();
    let tracks = |pair: &str| {
        is_tracking_param(&decode_name(pair.split('=').next().unwrap_or("")), hostname)
    };
    let kept: Vec<&str> = pairs
        .iter()
        .copied()
        .filter(|p| !p.is_empty() && !tracks(p))
        .collect();
    if kept.len() == pairs.len() {
        return unchanged();
    }
    let query = if kept.is_empty() {
        String::new()
    } else {
        format!("?{}", kept.join("&"))
    };
    CleanedUrl {
        url: format!("{base}{query}{hash}"),
        removed_tracker: pairs.iter().any(|p| !p.is_empty() && tracks(p)),
    }
}

/// Port of `cleanUrl` (`cleanUrl.ts:72`).
pub fn clean_url(url: &str) -> String {
    clean_url_report(url).url
}

/// Port of `markdownLink` (`cleanUrl.ts:76`).
pub fn markdown_link(title: &str, url: &str) -> String {
    let text = if title.is_empty() { url } else { title };
    let escaped = text.replace('[', "\\[").replace(']', "\\]");
    format!("[{escaped}]({})", clean_url(url))
}
