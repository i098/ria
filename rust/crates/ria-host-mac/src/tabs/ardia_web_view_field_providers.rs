// Ported from ardia packages/ardiacore/ios/ArdiaWebView+FieldProviders.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2_foundation::{NSString, NSURL, NSURLComponents, NSUUID};
use serde_json::{Value, json};

use super::ardia_web_view_page_messages::flag;

mod native;
pub use native::{
    SystemPasswordPanel, open_verification_code, system_panel_availability, verification_handler,
    when_system_panel_closes,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Field {
    Username,
    Password,
    Code,
}

impl Field {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "username" => Self::Username,
            "password" => Self::Password,
            "code" => Self::Code,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Provider {
    pub login: bool,
    pub one_time_code: bool,
    pub code_setup: bool,
    pub generation: f64,
}

impl Provider {
    pub fn allows(self, field: Field) -> bool {
        match field {
            Field::Username | Field::Password => self.login,
            Field::Code => self.one_time_code,
        }
    }

    pub fn payload(self) -> Value {
        json!({"login": self.login, "oneTimeCode": self.one_time_code, "codeSetup": self.code_setup, "generation": self.generation})
    }
}

pub struct FieldProviders {
    session: String,
    generation: f64,
    providers: HashMap<String, Provider>,
}

impl Default for FieldProviders {
    fn default() -> Self {
        Self {
            session: String::new(),
            generation: -1.0,
            providers: HashMap::new(),
        }
    }
}

impl FieldProviders {
    /// Returns `None` for a rejected update and `Some(true)` for a changed set.
    pub fn update<'a>(
        &mut self,
        profiles: impl IntoIterator<Item = &'a Value>,
        generation: f64,
        session: &str,
        original_profile: impl Fn(&str) -> String,
    ) -> Option<bool> {
        let new_session = self.session != session;
        if !generation.is_finite()
            || session.is_empty()
            || (!new_session && generation < self.generation)
        {
            return None;
        }
        let mut providers = HashMap::new();
        for entry in profiles {
            let Some(profile) = entry["profile"].as_str() else {
                continue;
            };
            let fields = entry["fields"].as_array().into_iter().flatten();
            let mut provider = Provider {
                login: false,
                one_time_code: false,
                code_setup: flag(entry, "codeSetup"),
                generation,
            };
            for field in fields {
                match field.as_str() {
                    Some("login") => provider.login = true,
                    Some("oneTimeCode") => provider.one_time_code = true,
                    _ => (),
                }
            }
            if provider.login || provider.one_time_code || provider.code_setup {
                providers.insert(original_profile(profile), provider);
            }
        }
        let changed = new_session || generation != self.generation || providers != self.providers;
        if changed {
            self.session = session.into();
            self.generation = generation;
            self.providers = providers;
        }
        Some(changed)
    }

    pub fn get(&self, original_profile: &str) -> Option<Provider> {
        self.providers.get(original_profile).copied()
    }
}

struct Offer {
    url: Retained<NSURL>,
    frame: String,
    navigation: u64,
    profile: String,
    generation: f64,
    handler: Retained<NSURL>,
}

#[derive(Default)]
pub struct ProviderPage {
    navigation: u64,
    sequence: f64,
    focus: Option<Field>,
    offers: HashMap<String, Offer>,
    query: Option<(String, Instant)>,
}

pub const FOCUS_QUERY_WAIT: Duration = Duration::from_millis(200);

impl ProviderPage {
    pub fn clear_provider_state(&mut self) {
        self.sequence = 0.0;
        self.focus = None;
        self.offers.clear();
    }

    pub fn reset(&mut self) -> bool {
        self.navigation = self.navigation.wrapping_add(1);
        self.clear_provider_state();
        self.query.take().is_some()
    }

    pub fn begin_query(&mut self, now: Instant) -> (&str, bool) {
        let superseded = self
            .query
            .replace((new_id(), now + FOCUS_QUERY_WAIT))
            .is_some();
        (self.query.as_ref().unwrap().0.as_str(), superseded)
    }

    pub fn expire_query(&mut self, query: &str) -> bool {
        if self.query.as_ref().is_some_and(|(id, _)| id == query) {
            self.query = None;
            return true;
        }
        false
    }

    pub fn note_focus(
        &mut self,
        field: Option<Field>,
        sequence: f64,
        generation: f64,
        current_generation: f64,
        query: &str,
        now: Instant,
    ) -> Option<bool> {
        if generation != current_generation
            || !sequence.is_finite()
            || sequence < 1.0
            || sequence <= self.sequence
        {
            return None;
        }
        self.sequence = sequence;
        self.focus = field;
        let (_, deadline) = self.query.as_ref().filter(|(id, _)| id == query)?;
        let on_time = now <= *deadline;
        self.query = None;
        Some(on_time)
    }

    pub fn panel_field(
        &self,
        provider: Option<Provider>,
        requested: Option<Field>,
        generation: f64,
    ) -> Result<Field, &'static str> {
        let provider = provider.ok_or("noProvider")?;
        if (requested.is_some() || generation >= 0.0) && generation != provider.generation {
            return Err("stale");
        }
        self.focus
            .filter(|focus| {
                provider.allows(*focus) && requested.is_none_or(|requested| requested == *focus)
            })
            .ok_or("notFocused")
    }

    pub fn forget_frame(&mut self, frame: Option<&str>) {
        self.offers
            .retain(|_, offer| frame.is_some_and(|frame| offer.frame != frame));
    }

    pub fn offer(
        &mut self,
        provider: Option<Provider>,
        generation: f64,
        original_profile: &str,
        frame: &str,
        text: &str,
        handler: impl FnOnce(&NSURL) -> Option<(Retained<NSURL>, String)>,
    ) -> Option<Value> {
        let provider =
            provider.filter(|provider| provider.code_setup && generation == provider.generation)?;
        if self.offers.len() >= 8 {
            return None;
        }
        let url = setup_url(text)?;
        if self.offers.values().any(|offer| *offer.url == *url) {
            return None;
        }
        let (handler, app) = handler(&url)?;
        if app.is_empty() {
            return None;
        }
        let token = new_id();
        let report = json!({"token": token, "app": app});
        self.offers.insert(
            token,
            Offer {
                url,
                frame: frame.into(),
                navigation: self.navigation,
                profile: original_profile.into(),
                generation: provider.generation,
                handler,
            },
        );
        Some(report)
    }

    pub fn redeem(
        &mut self,
        token: &str,
        providers: &FieldProviders,
        original_profile: &str,
        generation: f64,
        frame_alive: impl FnOnce(&str) -> bool,
        handler: impl FnOnce(&NSURL) -> Option<Retained<NSURL>>,
    ) -> Result<(Retained<NSURL>, Retained<NSURL>), &'static str> {
        let offer = self.offers.remove(token).ok_or("unknownToken")?;
        if (generation >= 0.0 && generation != providers.generation)
            || offer.generation != providers.generation
        {
            return Err("stale");
        }
        if offer.navigation != self.navigation {
            return Err("navigated");
        }
        if offer.profile != original_profile
            || !providers
                .get(original_profile)
                .is_some_and(|provider| provider.code_setup)
        {
            return Err("noProvider");
        }
        if !frame_alive(&offer.frame) {
            return Err("frameGone");
        }
        let handler = handler(&offer.url)
            .filter(|handler| handler.path() == offer.handler.path())
            .ok_or("handlerChanged")?;
        Ok((offer.url, handler))
    }
}

fn new_id() -> String {
    NSUUID::UUID().UUIDString().to_string()
}

fn setup_url(text: &str) -> Option<Retained<NSURL>> {
    let text = NSString::from_str(text);
    if text.is_empty() || text.len() > 2048 {
        return None;
    }
    let parts = NSURLComponents::componentsWithString(&text)?;
    let scheme = parts.scheme()?.lowercaseString();
    let host = parts.host()?.lowercaseString();
    let key = match (scheme.to_string().as_str(), host.to_string().as_str()) {
        ("otpauth", "totp" | "hotp") => "secret",
        ("otpauth-migration", "offline") => "data",
        _ => return None,
    };
    let items = parts.queryItems()?;
    if !items.iter().any(|item| {
        item.name().lowercaseString().to_string() == key
            && item.value().is_some_and(|value| !value.is_empty())
    }) {
        return None;
    }
    parts.URL()
}

pub fn panel_hint(label: &str, side: &str) -> Option<Value> {
    let alnum = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    let bytes = label.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 63
        || !alnum(bytes[0])
        || !alnum(bytes[bytes.len() - 1])
        || !bytes.iter().all(|byte| alnum(*byte) || *byte == b'-')
        || !matches!(side, "above" | "below")
    {
        return None;
    }
    Some(json!({"label": label, "side": side}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_generation_cannot_replace_the_current_providers() {
        let mut providers = FieldProviders::default();
        let login = json!({"profile": "Default", "fields": ["login"]});
        assert_eq!(
            providers.update([&login], 3.0, "session", |profile| profile.to_owned()),
            Some(true)
        );
        assert_eq!(
            providers.update([], 2.0, "session", |profile| profile.to_owned()),
            None
        );
        assert!(providers.get("Default").unwrap().login);
        assert_eq!(
            providers.update([], 2.0, "new session", |profile| profile.to_owned()),
            Some(true)
        );
        assert!(providers.get("Default").is_none());
    }

    #[test]
    fn a_late_focus_query_does_not_open_the_panel() {
        let mut page = ProviderPage::default();
        let now = Instant::now();
        let query = page.begin_query(now).0.to_owned();
        assert_eq!(
            page.note_focus(
                Some(Field::Password),
                1.0,
                2.0,
                2.0,
                &query,
                now + Duration::from_millis(201)
            ),
            Some(false)
        );
        assert_eq!(
            page.note_focus(Some(Field::Password), 2.0, 2.0, 2.0, &query, now),
            None
        );
    }
}
