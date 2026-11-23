// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Navigation.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use objc2::AnyThread;
use objc2::rc::Weak;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSEvent, NSImage, NSView};
use objc2_foundation::{NSDataBase64EncodingOptions, NSDictionary, NSString, NSURL};
use serde_json::{Value, json};

use crate::core::bindings::Tab;
use crate::core::convert::to_json;

use super::ardia_tab_lifecycle::TabLifecycle;

pub enum LoadAction {
    Ignore,
    Load,
    Create,
    Defer,
}

pub fn route_load(
    life: &mut TabLifecycle,
    url: &str,
    user_initiated: bool,
    has_tab: bool,
    has_window: bool,
    now: Instant,
) -> LoadAction {
    if url.is_empty() {
        return LoadAction::Ignore;
    }
    let transferred = life.transferred_url.take();
    let creating = life.creating_url.take();
    let same_page = transferred
        .as_deref()
        .is_some_and(|old| old.split('#').next() == url.split('#').next());
    let recent_creation = (has_tab || (!life.can_create() && !life.closed()))
        && life
            .creating_at
            .is_some_and(|at| now.saturating_duration_since(at) < Duration::from_secs(2))
        && creating.as_deref() == Some(url);
    if (has_tab && same_page) || recent_creation {
        return LoadAction::Ignore;
    }
    if has_tab {
        return LoadAction::Load;
    }
    life.pending_url = Some(url.into());
    life.pending_user_initiated = user_initiated;
    if has_window {
        LoadAction::Create
    } else {
        LoadAction::Defer
    }
}

#[derive(Default)]
pub struct NavigationRequest {
    loaded_once: bool,
    requested: Option<String>,
}

impl NavigationRequest {
    pub fn load(
        &mut self,
        life: &mut TabLifecycle,
        url: &str,
        user_initiated: bool,
        was_download: impl FnOnce(&str) -> bool,
    ) -> Result<(), Value> {
        let first = !std::mem::replace(&mut self.loaded_once, true);
        life.pending_user_initiated = false;
        if first && !user_initiated && was_download(url) {
            return Err(json!({"url": url, "committedUrl": "", "skipped": true}));
        }
        self.requested = Some(url.into());
        Ok(())
    }

    pub fn became_download(
        &mut self,
        url: &str,
        committed_url: &str,
        mut note: impl FnMut(&str),
    ) -> Value {
        note(url);
        if let Some(requested) = self.requested.take().filter(|requested| requested != url) {
            note(&requested);
        }
        json!({"url": url, "committedUrl": committed_url, "skipped": false})
    }
}

#[derive(Default)]
pub struct NavigationReports {
    queued: bool,
    sent: Option<Value>,
    favicon: Option<String>,
    commits: Commits,
    progress: Progress,
    blocked: Blocked,
}

#[derive(Default)]
struct Commits {
    tab: Option<Weak<Tab>>,
    url: String,
    documents: i32,
    at: f64,
    http_status: i32,
}

#[derive(Default)]
struct Progress {
    queued: bool,
    at: Option<Instant>,
    sent: f64,
}

#[derive(Default)]
struct Blocked {
    generation: u64,
    queued: bool,
    count: i64,
    last: Option<String>,
}

pub enum ProgressAction {
    Send,
    Wait(Duration),
    Ignore,
}

impl NavigationReports {
    pub fn queue(&mut self) -> bool {
        !std::mem::replace(&mut self.queued, true)
    }

    fn follow_commits(&mut self, tab: &Tab) {
        if !self
            .commits
            .tab
            .as_ref()
            .and_then(Weak::load)
            .is_some_and(|old| std::ptr::eq(&*old, tab))
        {
            self.commits = Commits {
                tab: Some(Weak::new(tab)),
                documents: tab.committed_document_count(),
                ..Commits::default()
            };
        }
    }

    pub fn note_document_commit(&mut self, tab: &Tab, now: SystemTime) {
        self.follow_commits(tab);
        let documents = tab.committed_document_count();
        if documents != self.commits.documents {
            self.commits.documents = documents;
            self.commits.at = now
                .duration_since(UNIX_EPOCH)
                .map_or(0.0, |time| time.as_secs_f64() * 1_000.0);
            self.commits.http_status = tab.committed_http_status();
        }
    }

    pub fn committed_url(&mut self, tab: &Tab) -> &str {
        self.follow_commits(tab);
        let visible = tab.url().to_string();
        if visible != self.commits.url
            && tab.navigation_entries().iter().find_map(|entry| {
                let entry = to_json(&entry);
                (entry["committed"].as_bool() == Some(true))
                    .then(|| entry["url"].as_str() == Some(&visible))
            }) == Some(true)
        {
            self.commits.url = visible;
        }
        &self.commits.url
    }

    pub fn emit(
        &mut self,
        tab: &Tab,
        page_theme: Option<(&str, Option<&str>)>,
        reports_commits: bool,
        crashed: &mut bool,
        now: SystemTime,
        mut send: impl FnMut(&Value),
    ) {
        self.queued = false;
        self.note_document_commit(tab, now);
        let mut url = tab.url().to_string();
        if renderer_debug_url(&url) {
            url = self
                .sent
                .as_ref()
                .and_then(|last| last["url"].as_str())
                .unwrap_or_default()
                .into();
        }
        let loading = tab.loading();
        let title = tab.title().to_string();
        let engine_theme = page_theme
            .is_none()
            .then(|| tab.theme_color())
            .flatten()
            .map(|value| value.to_string());
        let theme =
            page_theme.or_else(|| engine_theme.as_deref().map(|value| (value, Some("meta"))));
        let mut payload = json!({
            "url": url,
            "title": reported_title(&title, &url, loading),
            "canGoBack": tab.can_go_back(), "canGoForward": tab.can_go_forward(), "isLoading": loading,
            "themeColor": theme.map(|(color, _)| color), "themeColorSource": theme.and_then(|(_, source)| source),
            "initialDocumentAccessed": tab.initial_document_accessed(), "committedInitial": tab.committed_initial(),
            "committedAt": self.commits.at,
        });
        if reports_commits {
            payload["committedUrl"] = json!(self.committed_url(tab));
            if self.commits.http_status > 0 {
                payload["httpStatus"] = json!(self.commits.http_status);
            }
        }
        if *crashed && loading {
            *crashed = false;
            let mut gone = payload.clone();
            gone["url"] = json!("");
            send(&gone);
            self.sent = Some(gone);
        }
        if self.sent.as_ref() != Some(&payload) {
            send(&payload);
            self.sent = Some(payload);
        }
    }

    pub fn progress_changed(&mut self, value: f64, now: Instant) -> ProgressAction {
        let wait = self.progress.at.map_or(Duration::ZERO, |last| {
            Duration::from_millis(100).saturating_sub(now.saturating_duration_since(last))
        });
        if value <= 0.1 || value >= 1.0 || wait.is_zero() {
            ProgressAction::Send
        } else if std::mem::replace(&mut self.progress.queued, true) {
            ProgressAction::Ignore
        } else {
            ProgressAction::Wait(wait)
        }
    }

    pub fn send_progress(&mut self, value: f64, now: Instant, queued_only: bool) -> Option<Value> {
        let was_queued = std::mem::take(&mut self.progress.queued);
        if (queued_only && !was_queued) || value == self.progress.sent {
            return None;
        }
        self.progress.sent = value;
        self.progress.at = Some(now);
        Some(json!({"progress": value}))
    }

    pub fn reset_blocked(&mut self, report: bool) -> Option<Value> {
        let generation = self.blocked.generation.wrapping_add(1);
        let had = self.blocked.count != 0 || self.blocked.last.is_some();
        self.blocked = Blocked {
            generation,
            ..Blocked::default()
        };
        (report && had).then(|| json!({"count": 0, "url": ""}))
    }

    pub fn note_blocked(&mut self, count: i32, url: Option<String>) -> Option<u64> {
        self.blocked.count += i64::from(count);
        self.blocked.last = url;
        (!std::mem::replace(&mut self.blocked.queued, true)).then_some(self.blocked.generation)
    }

    pub fn emit_blocked(&mut self, generation: u64) -> Option<Value> {
        if self.blocked.generation != generation || !std::mem::take(&mut self.blocked.queued) {
            return None;
        }
        Some(
            json!({"count": self.blocked.count, "url": self.blocked.last.as_deref().unwrap_or_default()}),
        )
    }

    pub fn favicon_changed(
        &mut self,
        tab: &Tab,
        note: impl FnOnce(&NSString, &NSImage),
    ) -> Option<Value> {
        let image = tab.favicon()?;
        let url = if let Some(url) = tab.favicon_url().filter(|url| !url.is_empty()) {
            note(&url, &image);
            url.to_string()
        } else {
            favicon_png_url(&image)?
        };
        if self.favicon.as_deref() == Some(url.as_str()) {
            return None;
        }
        let event = json!({"url": url, "urls": [url]});
        self.favicon = Some(url);
        Some(event)
    }

    pub fn clear_favicon(&mut self) {
        self.favicon = None;
    }
}

fn favicon_png_url(image: &NSImage) -> Option<String> {
    // SAFETY: No destination rectangle or hints are needed for the current image.
    let cg_image =
        unsafe { image.CGImageForProposedRect_context_hints(std::ptr::null_mut(), None, None) }?;
    let bitmap = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg_image);
    // SAFETY: The PNG representation needs no encoder settings.
    let png = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }?;
    (!png.is_empty()).then(|| {
        format!(
            "data:image/png;base64,{}",
            png.base64EncodedStringWithOptions(NSDataBase64EncodingOptions::empty())
        )
    })
}

pub fn status_text(view: &NSView, text: &str, root_points_per_local_point: f64) -> Value {
    let mut report = json!({"text": text});
    if !text.is_empty()
        && let Some(window) = view.window()
    {
        let local = view.convertPoint_fromView(
            window.convertPointFromScreen(NSEvent::mouseLocation()),
            None,
        );
        let bounds = view.bounds();
        let x = local.x - bounds.origin.x;
        let y = local.y - bounds.origin.y;
        if x >= 0.0 && x < bounds.size.width && y >= 0.0 && y < bounds.size.height {
            let bottom = if view.isFlipped() {
                bounds.size.height - y
            } else {
                y
            };
            report["fromBottom"] = json!(bottom * root_points_per_local_point);
        }
    }
    report
}

fn renderer_debug_url(url: &str) -> bool {
    let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else {
        return false;
    };
    let scheme = url
        .scheme()
        .map(|scheme| scheme.lowercaseString().to_string());
    matches!(scheme.as_deref(), Some("chrome" | "ardia"))
        && url.host().is_some_and(|host| {
            matches!(
                host.lowercaseString().to_string().as_str(),
                "badcastcrash"
                    | "crash"
                    | "crashdump"
                    | "kill"
                    | "hang"
                    | "shorthang"
                    | "memory-exhaust"
                    | "memory-pressure-critical"
                    | "memory-pressure-moderate"
                    | "gpuclean"
                    | "gpucrash"
                    | "gpuhang"
                    | "inducebrowsercrashforrealz"
                    | "inducebrowserdcheckforrealz"
                    | "cfi-crash"
                    | "heap-corruption-crash"
            )
        })
}

fn reported_title<'a>(title: &'a str, url: &str, loading: bool) -> &'a str {
    if !loading || title.is_empty() {
        return title;
    }
    let Some((_, mut shown)) = url.split_once("://").filter(|(scheme, _)| {
        scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
    }) else {
        return title;
    };
    if shown
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("www."))
    {
        shown = &shown[4..];
    }
    if shown
        .split_once('/')
        .is_some_and(|(_, after)| after.is_empty())
        && !shown.contains(['?', '#'])
    {
        shown = &shown[..shown.len() - 1];
    }
    let decoded;
    if shown.contains("%20") {
        decoded = shown.replace("%20", " ");
        shown = &decoded;
    }
    if title == shown { "" } else { title }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transferred_url_skips_only_its_first_matching_load() {
        let now = Instant::now();
        let mut life = TabLifecycle::default();
        life.transferred_url = Some("https://example.test/#old".into());
        assert!(matches!(
            route_load(
                &mut life,
                "https://example.test/#new",
                false,
                true,
                true,
                now
            ),
            LoadAction::Ignore
        ));
        assert!(matches!(
            route_load(
                &mut life,
                "https://example.test/#new",
                false,
                true,
                true,
                now
            ),
            LoadAction::Load
        ));
    }

    #[test]
    fn loading_title_hides_only_the_displayed_url() {
        assert_eq!(
            reported_title("example.test", "HTTPS://WWW.example.test/", true),
            ""
        );
        assert_eq!(
            reported_title("example.test", "https://www.example.test/", false),
            "example.test"
        );
        assert_eq!(
            reported_title("example.test/a b", "https://example.test/a%20b", true),
            ""
        );
        assert_eq!(
            reported_title("example.test/a%20b", "https://example.test/a%20b", true),
            "example.test/a%20b"
        );
        assert_eq!(
            reported_title("example.test", "https://example.test/?", true),
            "example.test"
        );
    }

    #[test]
    fn resetting_blocked_counts_cancels_the_prior_timer() {
        let mut reports = NavigationReports::default();
        let old = reports.note_blocked(5, None).unwrap();
        assert_eq!(
            reports.reset_blocked(true),
            Some(json!({"count": 0, "url": ""}))
        );
        let new = reports
            .note_blocked(1, Some("https://example.test/".into()))
            .unwrap();
        assert_eq!(reports.emit_blocked(old), None);
        assert_eq!(
            reports.emit_blocked(new),
            Some(json!({"count": 1, "url": "https://example.test/"}))
        );
    }
}
