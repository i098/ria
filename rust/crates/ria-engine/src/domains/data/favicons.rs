// Ported from ardia packages/ardiacore/ios/ArdiaFavicons.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Fetches the icon of a page and answers with a 64 px PNG as a `file:` URI (a normal profile with a safe name) or
//! as a `data:` URI. Ardia reads an `NSImage` and draws it with AppKit; this port decodes with the `image` crate
//! and scales with a Lanczos filter, so the pixels differ slightly (a known difference, not a behavior change).

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use image::imageops::{self, FilterType};
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde_json::{Value, json};

use super::profiles::{ProfileRegistry, profile_directory_name};

const FAVICON_SIZE: u32 = 32;
const MAX_BYTES: usize = 2 * 1024 * 1024;
const CACHE_LIMIT: usize = 512;

fn safe_name(name: Option<&str>) -> Option<&str> {
    name.filter(|n| {
        (1..=80).contains(&n.len())
            && n.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    })
}

/// `PNG(image, pixels, ...)`: the image fitted in a square of `min(pixels, longest side)` px, centered, as PNG.
fn png(image: &DynamicImage, pixels: u32) -> Option<(Vec<u8>, u32)> {
    let (w, h) = (image.width(), image.height());
    if w == 0 || h == 0 {
        return None;
    }
    let side = pixels.min(w.max(h));
    let scale = f64::from(side) / f64::from(w.max(h));
    let (nw, nh) = (
        ((f64::from(w) * scale).round() as u32).max(1),
        ((f64::from(h) * scale).round() as u32).max(1),
    );
    let scaled = imageops::resize(&image.to_rgba8(), nw, nh, FilterType::Lanczos3);
    let mut canvas = RgbaImage::new(side, side);
    imageops::overlay(
        &mut canvas,
        &scaled,
        i64::from((side - nw.min(side)) / 2),
        i64::from((side - nh.min(side)) / 2),
    );
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .ok()?;
    Some((out, side))
}

/// The bytes of a `data:` URL: base64 when the header says so, otherwise percent-encoded.
fn data_url_bytes(url: &str) -> Option<Vec<u8>> {
    let (header, body) = url.strip_prefix("data:")?.split_once(',')?;
    if !header.ends_with(";base64") {
        return Some(percent_decode(body));
    }
    let decoded;
    let body = if body.contains('%') {
        decoded = percent_decode(body);
        std::str::from_utf8(&decoded).ok()?
    } else {
        body
    };
    if body.chars().any(char::is_whitespace) {
        let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
        STANDARD.decode(compact).ok()
    } else {
        STANDARD.decode(body).ok()
    }
}

fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%'
            && let (Some(&a), Some(&b)) = (bytes.get(i + 1), bytes.get(i + 2))
            && let (Some(a), Some(b)) = (hex(a), hex(b))
        {
            out.push((a * 16 + b) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn cache_key(profile: Option<&str>, url: &str) -> String {
    format!("{}\n{url}", profile.unwrap_or_default())
}

/// `ArdiaFavicons`. Main thread only.
pub struct Favicons {
    registry: ProfileRegistry,
    data_directory: PathBuf,
    seen: RefCell<HashMap<String, Rc<DynamicImage>>>,
}

impl Favicons {
    pub fn new(registry: ProfileRegistry, data_directory: PathBuf) -> Self {
        Self {
            registry,
            data_directory,
            seen: RefCell::default(),
        }
    }

    /// `noteImage:forURL:profile:`: the host gives an image that it already has for the URL.
    pub fn note_image(&self, image: DynamicImage, url: &str, profile: Option<&str>) {
        if url.is_empty() {
            return;
        }
        let mut seen = self.seen.borrow_mut();
        if seen.len() >= CACHE_LIMIT
            && let Some(old) = seen.keys().next().cloned()
        {
            seen.remove(&old);
        }
        seen.insert(cache_key(profile, url), Rc::new(image));
    }

    fn store(
        &self,
        image: Option<&DynamicImage>,
        profile: Option<&str>,
        name: Option<&str>,
    ) -> Option<Value> {
        let (png, side) = png(image?, FAVICON_SIZE * 2)?;
        if let Some(profile) = profile
            && !self.registry.is_private_profile(profile)
            && let Some(name) = safe_name(name)
        {
            let dir = self
                .data_directory
                .join(profile_directory_name(profile))
                .join("Ardia Favicons");
            std::fs::create_dir_all(&dir).ok()?;
            let path = dir.join(format!("{name}.png"));
            let temporary = dir.join(format!("{name}.png.tmp"));
            std::fs::write(&temporary, &png).ok()?;
            std::fs::rename(&temporary, &path).ok()?;
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis());
            let uri = format!("{}?v={millis}", url::Url::from_file_path(&path).ok()?);
            return Some(json!({ "uri": uri, "width": side, "height": side }));
        }
        let uri = format!("data:image/png;base64,{}", STANDARD.encode(&png));
        Some(json!({ "uri": uri, "width": side, "height": side }))
    }

    fn download(&self, url: &str, profile: &str, done: Box<dyn FnOnce(Option<Vec<u8>>)>) {
        let url = url.to_owned();
        let fetch = move |loaded: Option<super::host::Profile>| match loaded {
            Some(p) if !p.destroyed() => p.fetch_icon(&url, MAX_BYTES, done),
            _ => done(None),
        };
        if self.registry.is_private_profile(profile) {
            fetch(self.registry.loaded_profile(profile));
        } else {
            self.registry.with_profile(profile, fetch);
        }
    }

    /// `fetch:profile:name:completion:`. `done` gets `{uri, width, height}` or `None`.
    pub fn fetch(
        self: &Rc<Self>,
        url: &str,
        profile: &str,
        name: Option<&str>,
        done: impl FnOnce(Option<Value>) + 'static,
    ) {
        let seen = self
            .seen
            .borrow()
            .get(&cache_key(Some(profile), url))
            .cloned();
        if let Some(seen) = seen {
            return done(self.store(Some(&seen), Some(profile), name));
        }
        if url.starts_with("data:image/") {
            let image = data_url_bytes(url).and_then(|d| image::load_from_memory(&d).ok());
            return done(self.store(image.as_ref(), Some(profile), name));
        }
        let scheme = url::Url::parse(url).map(|u| u.scheme().to_lowercase());
        if !matches!(scheme.as_deref(), Ok("https" | "http")) {
            return done(None);
        }
        let (this, profile_owned, name) =
            (self.clone(), profile.to_owned(), name.map(str::to_owned));
        self.download(
            url,
            profile,
            Box::new(move |body| {
                let image = body.and_then(|b| image::load_from_memory(&b).ok());
                done(match image {
                    Some(image) => this.store(Some(&image), Some(&profile_owned), name.as_deref()),
                    None => None,
                });
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255])))
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn names_and_data_urls() {
        assert_eq!(safe_name(Some("a_b-1")), Some("a_b-1"));
        assert_eq!(safe_name(Some("../x")), None);
        assert_eq!(safe_name(Some("")), None);
        assert_eq!(safe_name(Some(&"a".repeat(81))), None);
        let url = format!("data:image/png;base64,{}", STANDARD.encode(tiny_png(2, 2)));
        assert_eq!(data_url_bytes(&url), Some(tiny_png(2, 2)));
        let payload = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
        let escaped = payload.replace('=', "%3D");
        let bytes = data_url_bytes(&format!("data:image/png;base64,%20{escaped}")).unwrap();
        let image = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (1, 1));
        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(
            data_url_bytes("data:text/plain,a%20b"),
            Some(b"a b".to_vec())
        );
    }

    #[test]
    fn png_fits_and_never_upscales() {
        let wide =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(16, 8, image::Rgba([0, 0, 255, 255])));
        let (bytes, side) = png(&wide, 64).unwrap();
        assert_eq!(side, 16);
        let back = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!((back.width(), back.height()), (16, 16));
        assert_eq!(back.get_pixel(8, 8)[3], 255);
        assert_eq!(back.get_pixel(8, 0)[3], 0);
        let big =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(200, 100, image::Rgba([0, 0, 0, 255])));
        assert_eq!(png(&big, 64).unwrap().1, 64);
    }
}
