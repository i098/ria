// Ported from ardia packages/ardiacore/ios/Origins.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The origin text of a URL: scheme, host, and a port that is not the default of `http` or `https`.

use url::Url;

fn bracketed(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_owned()
    }
}

/// `Origins.of(_ url: URL)`: `None` when the URL has no scheme or no host.
pub fn of_url(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    let host = url.host_str()?.to_lowercase();
    if host.is_empty() {
        return None;
    }
    let scheme = url.scheme().to_lowercase();
    // `Url::port` is `None` for the default port of a special scheme; `port_or_known_default` would add it back.
    let port = url
        .port()
        .filter(|port| !matches!((scheme.as_str(), port), ("http", 80) | ("https", 443)));
    Some(format!(
        "{scheme}://{}{}",
        bracketed(&host),
        port.map(|p| format!(":{p}")).unwrap_or_default()
    ))
}

/// `Origins.of(_ origin: WKSecurityOrigin)`: a port of 0 or less is no port.
pub fn of_parts(protocol: &str, host: &str, port: i64) -> String {
    let port = if port > 0 {
        format!(":{port}")
    } else {
        String::new()
    };
    format!(
        "{}://{}{port}",
        protocol.to_lowercase(),
        bracketed(&host.to_lowercase())
    )
}

/// `Origins.site`: `None` when the protocol or the host is empty.
pub fn site(protocol: &str, host: &str, port: i64) -> Option<String> {
    if protocol.is_empty() || host.is_empty() {
        None
    } else {
        Some(of_parts(protocol, host, port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_drop_default_ports_and_bracket_ipv6() {
        assert_eq!(
            of_url("HTTPS://Example.COM:443/a?b"),
            Some("https://example.com".into())
        );
        assert_eq!(
            of_url("http://example.com:8080/"),
            Some("http://example.com:8080".into())
        );
        assert_eq!(of_url("http://[::1]:80/"), Some("http://[::1]".into()));
        assert_eq!(
            of_url("https://[::1]:9000/"),
            Some("https://[::1]:9000".into())
        );
        assert_eq!(of_url("mailto:a@b.c"), None);
        assert_eq!(of_url("not a url"), None);
        assert_eq!(of_parts("HTTPS", "A.b", 0), "https://a.b");
        assert_eq!(of_parts("http", "::1", 81), "http://[::1]:81");
        assert_eq!(site("", "a", 0), None);
        assert_eq!(site("https", "a", 443), Some("https://a:443".into()));
    }
}
