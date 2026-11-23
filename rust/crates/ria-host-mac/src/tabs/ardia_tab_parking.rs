// Ported from ardia packages/ardiacore/ios/ArdiaTabParking.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2_app_kit::NSView;
use objc2_foundation::NSRect;
use ria_engine::types::{LinkRoute, Number};

use crate::core::bindings::{Adoption, Profile, Tab, Window};

const HOLD: Duration = Duration::from_secs(30);
const LINK_BACKSTOP: Duration = Duration::from_secs(300);

pub trait ParkedPageHolder {
    fn delegate(&self) -> &AnyObject;
    fn closed_by_chrome(&self) -> bool;
}

struct Launch {
    key: String,
    url: String,
    profile: String,
    tab: Retained<Tab>,
    expires: Instant,
}

struct Link {
    url: Option<String>,
    tab: Retained<Tab>,
}

pub struct TabParking {
    parking_view: Retained<NSView>,
    parked: HashMap<String, Retained<Tab>>,
    reloads: HashMap<String, (Instant, Retained<Tab>)>,
    holders: HashMap<*const Tab, (Weak<Tab>, Box<dyn ParkedPageHolder>)>,
    closed_launch: HashSet<String>,
    launch: Option<Launch>,
    launch_claimed: bool,
    links: BTreeMap<usize, Link>,
    link_profile: String,
    link_deadline: Option<Instant>,
    links_started: bool,
    dismissed_links: Vec<String>,
    dismissed_marks: HashSet<usize>,
}

impl TabParking {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            parking_view: NSView::initWithFrame(mtm.alloc(), NSRect::ZERO),
            parked: HashMap::new(),
            reloads: HashMap::new(),
            holders: HashMap::new(),
            closed_launch: HashSet::new(),
            launch: None,
            launch_claimed: false,
            links: BTreeMap::new(),
            link_profile: String::new(),
            link_deadline: None,
            links_started: false,
            dismissed_links: Vec::new(),
            dismissed_marks: HashSet::new(),
        }
    }

    pub fn park(&mut self, key: String, tab: Retained<Tab>) {
        self.parking_view.addSubview(&tab.view());
        let current = std::ptr::from_ref(&*tab);
        if let Some(previous) = self.parked.insert(key, tab)
            && std::ptr::from_ref(&*previous) != current
        {
            close_open_tab(&previous);
            self.holders.remove(&std::ptr::from_ref(&*previous));
        }
    }

    pub fn parked_tab(&self, key: &str) -> Option<&Tab> {
        self.parked.get(key).map(|tab| &**tab)
    }

    pub fn unpark(&mut self, key: &str) -> Option<Retained<Tab>> {
        self.parked.remove(key)
    }

    pub fn took_transfer(&mut self, key: &str) {
        self.reloads.remove(key);
    }

    pub fn release_transfer(&mut self, key: &str) {
        self.closed_launch.remove(key);
        if let Some(tab) = self.unpark(key) {
            close_open_tab(&tab);
            self.holders.remove(&std::ptr::from_ref(&*tab));
        }
    }

    pub fn launch_was_closed(&mut self, key: &str) -> bool {
        if let Some(tab) = self.parked.get(key) {
            let pointer = std::ptr::from_ref(&**tab);
            if self.holders.get(&pointer).is_some_and(|(tab_ref, holder)| {
                tab_ref.load().is_some() && (tab.closed() || holder.closed_by_chrome())
            }) {
                self.holders.remove(&pointer);
                self.parked.remove(key);
                self.closed_launch.insert(key.into());
            }
        }
        self.closed_launch.contains(key)
    }

    pub fn rehome_from_window(
        &self,
        closing: &Window,
        holding: impl Fn(&Tab) -> Option<Retained<Window>>,
        keeper_for_profile: impl Fn(&Profile) -> Retained<Window>,
        adopt: impl Fn(&Window, &Window, &Tab) -> Adoption,
    ) -> bool {
        for tab in self.parked.values() {
            if tab.closed() || !holding(tab).is_some_and(|owner| std::ptr::eq(&*owner, closing)) {
                continue;
            }
            if let Some(profile) = tab.profile() {
                let keeper = keeper_for_profile(&profile);
                let adoption = adopt(&keeper, closing, tab);
                if adoption != Adoption::Adopted {
                    keeper.close();
                }
                if adoption == Adoption::Busy {
                    return false;
                }
            }
        }
        true
    }

    /// # Safety
    ///
    /// The holder must implement `ArdiaCoreTabDelegate`.
    pub unsafe fn keep_offscreen(&mut self, tab: &Tab, holder: Box<dyn ParkedPageHolder>) {
        self.parking_view.addSubview(&tab.view());
        // SAFETY: The caller provides a delegate implementing the engine protocol.
        unsafe { tab.set_delegate(Some(holder.delegate())) };
        self.holders.retain(|_, (tab, _)| tab.load().is_some());
        self.holders
            .insert(std::ptr::from_ref(tab), (Weak::new(tab), holder));
    }

    pub fn take_holder(&mut self, tab: &Tab) -> Option<Box<dyn ParkedPageHolder>> {
        let (tab, holder) = self.holders.remove(&std::ptr::from_ref(tab))?;
        tab.load().map(|_| holder)
    }

    pub fn hold_across_reload(
        &mut self,
        tabs: impl IntoIterator<Item = (String, Retained<Tab>)>,
        now: Instant,
    ) {
        let until = now + HOLD;
        self.reloads
            .extend(tabs.into_iter().map(|(key, tab)| (key, (until, tab))));
    }

    pub fn expire_reloads(&mut self, now: Instant, mut expired: impl FnMut(&str, &Tab)) {
        for (key, (_, tab)) in self.reloads.extract_if(|_, (until, _)| *until <= now) {
            expired(&key, &tab);
            if self
                .parked
                .get(&key)
                .is_some_and(|parked| std::ptr::eq(&**parked, &*tab))
            {
                self.parked.remove(&key);
                close_open_tab(&tab);
                self.holders.remove(&std::ptr::from_ref(&*tab));
            }
        }
    }

    pub fn launch_tab(&self) -> Option<&Tab> {
        self.launch.as_ref().map(|launch| &*launch.tab)
    }

    pub fn hold_launch_tab(
        &mut self,
        key: &str,
        url: &str,
        profile: &str,
        now: Instant,
        open: impl FnOnce() -> Option<Retained<Tab>>,
    ) {
        if self.launch.is_none()
            && !self.launch_claimed
            && !key.is_empty()
            && let Some(tab) = open()
        {
            self.launch = Some(Launch {
                key: key.into(),
                url: url.into(),
                profile: profile.into(),
                tab,
                expires: now + HOLD,
            });
        }
    }

    pub fn claim_launch_tab(
        &mut self,
        key: &str,
        url: &str,
        profile: &str,
        drop_page: impl FnOnce(&Tab),
    ) {
        self.launch_claimed = true;
        let Some(launch) = self.launch.take() else {
            return;
        };
        if launch.key == key
            && launch.url == url
            && launch.profile == profile
            && !self.parked.contains_key(key)
        {
            self.parked.insert(launch.key, launch.tab);
        } else {
            drop_page(&launch.tab);
            close_open_tab(&launch.tab);
        }
    }

    pub fn expire_launch_tab(
        &mut self,
        now: Instant,
        is_held: impl FnOnce(&Tab) -> bool,
        drop_page: impl FnOnce(&Tab),
    ) {
        if let Some(launch) = &mut self.launch
            && launch.expires <= now
        {
            if is_held(&launch.tab) {
                launch.expires = now + HOLD;
            } else {
                self.claim_launch_tab("", "", "", drop_page);
            }
        }
    }

    pub fn first_link_tab(&self) -> Option<&Tab> {
        self.links.get(&0).map(|link| &*link.tab)
    }

    pub fn hold_link_tabs(
        &mut self,
        urls: &[String],
        profile: &str,
        now: Instant,
        mut open: impl FnMut(&str) -> Option<Retained<Tab>>,
    ) {
        if self.links_started {
            return;
        }
        self.links_started = true;
        self.link_profile = profile.into();
        for (index, url) in urls.iter().take(4).enumerate() {
            if let Some(tab) = open(url) {
                self.links.insert(
                    index,
                    Link {
                        url: Some(url.clone()),
                        tab,
                    },
                );
            }
        }
        if !self.links.is_empty() {
            self.link_deadline = Some(now + LINK_BACKSTOP);
        }
    }

    pub fn expire_link_tabs(&mut self, now: Instant, mut drop_page: impl FnMut(&Tab)) {
        if self.link_deadline.is_some_and(|until| until <= now) {
            self.link_deadline = None;
            for link in std::mem::take(&mut self.links).into_values() {
                drop_page(&link.tab);
                close_open_tab(&link.tab);
            }
        }
    }

    pub fn link_mark(&self, tab: &Tab) -> Option<String> {
        self.links.iter().find_map(|(index, link)| {
            (link.url.is_some() && std::ptr::eq(&*link.tab, tab)).then(|| format!("link:{index}"))
        })
    }

    pub fn dismiss_link_tab(&mut self, tab: &Tab, drop_page: impl FnOnce(&Tab)) {
        if let Some((index, link)) = self
            .links
            .extract_if(.., |_, link| std::ptr::eq(&*link.tab, tab))
            .next()
        {
            if let Some(url) = link.url {
                self.dismissed_links.push(url);
            }
            self.dismissed_marks.insert(index);
            drop_page(tab);
            close_open_tab(tab);
        }
    }

    pub fn take_dismissed_link_mark(&mut self, mark: &str) -> bool {
        mark_index(mark).is_some_and(|index| self.dismissed_marks.remove(&index))
    }

    pub fn route_link_tabs(&mut self, urls: &[String]) -> LinkRoute {
        let mut dismissed = Vec::new();
        let mut any = false;
        let mut adopt_ids = Vec::with_capacity(urls.len());
        for (index, url) in urls.iter().enumerate() {
            if let Some(at) = self.dismissed_links.iter().position(|item| item == url) {
                self.dismissed_links.remove(at);
                dismissed.push(Number(index as f64));
                adopt_ids.push(None);
            } else if let Some((mark, link)) = self
                .links
                .iter_mut()
                .find(|(_, link)| link.url.as_ref() == Some(url))
            {
                link.url = None;
                adopt_ids.push(Some(format!("link:{mark}")));
                any = true;
            } else {
                adopt_ids.push(None);
            }
        }
        LinkRoute {
            profile: any.then(|| self.link_profile.clone()),
            adopt_ids: any.then_some(adopt_ids),
            dismissed: (any || !dismissed.is_empty()).then_some(dismissed),
        }
    }

    pub fn park_link_tab(&mut self, mark: &str, key: &str) -> Option<Retained<Tab>> {
        if key.is_empty() || self.parked.contains_key(key) {
            return None;
        }
        let link = self.links.remove(&mark_index(mark)?)?;
        self.parked.insert(key.into(), link.tab.clone());
        Some(link.tab)
    }

    pub fn discard_tab(&mut self, tab: &Tab, key: &str, drop_page: impl FnOnce(&Tab)) {
        if self
            .parked
            .get(key)
            .is_some_and(|parked| std::ptr::eq(&**parked, tab))
        {
            self.parked.remove(key);
        }
        drop_page(tab);
        close_open_tab(tab);
        self.holders.remove(&std::ptr::from_ref(tab));
    }
}

fn mark_index(mark: &str) -> Option<usize> {
    let digits = mark.strip_prefix("link:")?;
    if (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|digit| digit.is_ascii_digit())
    {
        return None;
    }
    digits.parse().ok()
}

fn close_open_tab(tab: &Tab) {
    if !tab.closed() {
        tab.close_now();
    }
}
