// Ported from ardia packages/ardiacore/ios/ArdiaWebView+ContextMenu.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSEventMask, NSEventType, NSView, NSWindow};
use objc2_foundation::{NSData, NSPoint, NSRange, NSString, NSURL, NSUUID};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use super::ardia_tab_registry::TabRegistry;
use crate::core::bindings::{Payload, Tab};
use crate::core::convert::{from_json, to_json};

const SEPARATOR: i64 = 3;
const PICK_POINT_LIFETIME: Duration = Duration::from_secs(60);
const SHARE_LIFETIME: Duration = Duration::from_secs(3_600);

fn selection_label(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let ns = NSString::from_str(&collapsed);
    if ns.length() <= 50 {
        return collapsed;
    }
    let end = collapsed
        .encode_utf16()
        .take(50)
        .enumerate()
        .filter_map(|(i, ch)| (ch == u16::from(b' ') && i > 25).then_some(i))
        .last()
        .unwrap_or(50);
    let range = ns.rangeOfComposedCharacterSequencesForRange(NSRange::new(0, end));
    let label = ns.substringToIndex(range.length);
    format!("{label}…")
}

fn menu_title(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
        } else if chars.next_if_eq(&'&').is_some() {
            out.push('&');
        }
    }
    out
}

fn items(menu: &Value) -> &[Value] {
    menu.as_array().map_or(&[], Vec::as_slice)
}

fn menu_item_type(item: &Value) -> i64 {
    item["type"].as_i64().unwrap_or_else(|| {
        if flag(&item["separator"]) {
            SEPARATOR
        } else {
            0
        }
    })
}

fn flag(value: &Value) -> bool {
    value
        .as_bool()
        .unwrap_or_else(|| value.as_f64().is_some_and(|number| number != 0.0))
}

pub fn describe_menu(menu: &[Value], captured: bool) -> Vec<Value> {
    menu.iter().filter(|item| item.is_object()).map(|item| {
        let kind = menu_item_type(item);
        let label = menu_title(item["label"].as_str().unwrap_or_default());
        let id = item["id"].as_i64().unwrap_or(0);
        let enabled = flag(&item["enabled"]);
        let visible = !matches!(item["visible"], Value::Bool(_) | Value::Number(_)) || flag(&item["visible"]);
        let mut row = if captured {
            json!({"id": id, "title": label, "enabled": enabled, "hidden": !visible, "separator": kind == SEPARATOR, "checked": flag(&item["checked"])})
        } else {
            let kind = usize::try_from(kind).ok().and_then(|i| [1, 2, 3, 4, 0, 5, 5].get(i)).copied().unwrap_or(0);
            json!({"id": id, "label": label, "type": kind, "enabled": enabled, "visible": visible})
        };
        if let Some(submenu) = item["submenu"].as_array() { row["submenu"] = Value::Array(describe_menu(submenu, captured)); }
        row
    }).collect()
}

pub fn find_pick(menu: &[Value], pick: &str) -> Option<Value> {
    let mut input = pick.trim().split('\t');
    let label = input.next().filter(|label| !label.is_empty())?;
    let flags = input
        .next()
        .and_then(|flags| flags.parse::<i32>().ok())
        .unwrap_or(0);
    let mut path = Vec::new();
    let command = menu_command(menu, label, &mut path)?;
    Some(json!({"command": command, "path": path, "flags": flags}))
}

fn menu_command(menu: &[Value], label: &str, path: &mut Vec<usize>) -> Option<i64> {
    for (index, item) in menu.iter().enumerate().filter(|(_, item)| item.is_object()) {
        path.push(index);
        if menu_title(item["label"].as_str().unwrap_or_default()) == label {
            let id = item["id"].as_i64();
            if id.is_none() {
                path.pop();
            }
            return id;
        }
        if let Some(id) = menu_command(items(&item["submenu"]), label, path) {
            return Some(id);
        }
        path.pop();
    }
    None
}

struct MenuPoint {
    tab: Weak<Tab>,
    at: Instant,
    point: NSPoint,
}

#[derive(Default)]
pub struct ElementPick {
    menu: Option<MenuPoint>,
    armed: Option<(Weak<Tab>, u64)>,
    sequence: u64,
    input: bool,
}

impl ElementPick {
    pub fn note_context_menu(&mut self, tab: &Tab, host: &NSView, zoom: f64) {
        let Some(window) = host.window() else { return };
        let point = host.convertPoint_fromView(window.mouseLocationOutsideOfEventStream(), None);
        let bounds = host.bounds();
        let x = point.x - bounds.origin.x;
        let y = point.y - bounds.origin.y;
        if x < 0.0 || y < 0.0 || x >= bounds.size.width || y >= bounds.size.height {
            return;
        }
        let y = if host.isFlipped() {
            y
        } else {
            bounds.size.height - y
        };
        let zoom = if zoom > 0.0 { zoom } else { 1.0 };
        self.note_menu_point(tab, Instant::now(), NSPoint::new(x / zoom, y / zoom));
    }

    pub fn note_menu_point(&mut self, tab: &Tab, now: Instant, point: NSPoint) {
        self.menu = Some(MenuPoint {
            tab: Weak::new(tab),
            at: now,
            point,
        });
    }

    pub fn start_for_frame_click(
        &mut self,
        tab: &Tab,
        tab_id: i32,
        url: &str,
        frame: i32,
        now: Instant,
    ) -> Option<Value> {
        if tab.closed() || tab.tab_id() != tab_id || tab.url().to_string() != url {
            return None;
        }
        let point = self
            .menu
            .take()
            .filter(|menu| {
                frame == 0
                    && same_tab(&menu.tab, tab)
                    && now.saturating_duration_since(menu.at) < PICK_POINT_LIFETIME
            })
            .map_or(NSPoint::new(-1.0, -1.0), |menu| menu.point);
        Some(json!({"command": "hideElement", "text": "", "point": [point.x, point.y]}))
    }

    pub fn arm(&mut self, tab: &Tab, document_count: u64) -> u64 {
        self.sequence += 1;
        self.armed = Some((Weak::new(tab), document_count));
        self.input = false;
        self.sequence
    }

    /// Call only for a real mouse-up or Return on this page, never for a page script message.
    pub fn note_input(&mut self, tab: &Tab) {
        if self
            .armed
            .as_ref()
            .is_some_and(|(armed, _)| same_tab(armed, tab))
        {
            self.input = true;
        }
    }

    pub fn take(&mut self, tab: &Tab, document_count: u64, sequence: u64) -> bool {
        let armed = self.armed.take();
        let input = std::mem::take(&mut self.input);
        let matched = sequence == self.sequence
            && input
            && armed.is_some_and(|(armed, count)| count == document_count && same_tab(&armed, tab));
        self.sequence += 1;
        matched
    }
}

fn same_tab(weak: &Weak<Tab>, tab: &Tab) -> bool {
    weak.load().is_some_and(|armed| std::ptr::eq(&*armed, tab))
}

pub struct ElementPickMonitor(Retained<AnyObject>);

impl ElementPickMonitor {
    pub fn install(mtm: MainThreadMarker, note_input: impl Fn(&NSView) + 'static) -> Option<Self> {
        let note_input = Rc::new(note_input);
        // SAFETY: The local monitor returns each event unmodified and AppKit owns the block.
        unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                NSEventMask::LeftMouseUp | NSEventMask::KeyDown,
                &RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| {
                    let input = event.as_ref();
                    let view = input.window(mtm).and_then(|window| match input.r#type() {
                        NSEventType::KeyDown if matches!(input.keyCode(), 36 | 76) => window
                            .firstResponder()
                            .and_then(|responder| responder.downcast::<NSView>().ok()),
                        NSEventType::LeftMouseUp => window.contentView().and_then(|content| {
                            let root = super::superview(&content).unwrap_or(content);
                            root.hitTest(root.convertPoint_fromView(input.locationInWindow(), None))
                        }),
                        _ => None,
                    });
                    if let Some(view) = view {
                        let note_input = note_input.clone();
                        main_queue::post(Box::new(move || note_input(&view)));
                    }
                    event.as_ptr()
                }),
            )
        }
        .map(Self)
    }
}

impl Drop for ElementPickMonitor {
    fn drop(&mut self) {
        // SAFETY: The stored token comes from AppKit's local monitor registration.
        unsafe { NSEvent::removeMonitor(&self.0) };
    }
}

const TAKE_FRAME_CLICK: &str = r#"new Promise((resolve) => {
  const until = Date.now() + 3000;
  const look = () => {
    const ublock = chrome.extension.getBackgroundPage()?.µBlock;
    const click = ublock?.ardiaHideElementFrameClick;
    if (click && Date.now() - click.at < 5000) {
      ublock.ardiaHideElementFrameClick = null;
      return resolve({tabId: click.tabId, pageUrl: click.pageUrl, frameId: click.frameId});
    }
    if (Date.now() > until) return resolve(null);
    setTimeout(look, 25);
  };
  look();
}).then((value) => post('result', JSON.stringify({value})),
        (e) => post('result', JSON.stringify({error: String(e && e.message || e)})))"#;

pub struct FrameClick {
    pub tab_id: i32,
    pub page_url: String,
    pub frame_id: i32,
}

/// Read the click recorded by the content blocker's trusted extension page.
/// Call only with that extension's hidden tab after a native menu selection.
pub fn take_frame_click(
    hidden_extension_tab: &Tab,
    finish: impl FnOnce(Option<FrameClick>) + 'static,
) {
    let finish = Rc::new(Cell::new(Some(finish)));
    let deadline = finish.clone();
    main_queue::after(
        Duration::from_secs(6),
        Box::new(move || {
            if let Some(finish) = deadline.take() {
                finish(None);
            }
        }),
    );
    hidden_extension_tab.evaluate(&NSString::from_str(TAKE_FRAME_CLICK), move |reply| {
        let finish = finish.clone();
        main_queue::post(Box::new(move || {
            let click = reply.and_then(|reply| {
                let mut data = serde_json::from_str::<Value>(&reply).ok()?;
                let click = data.get_mut("value")?.as_object_mut()?;
                let tab_id = i32::try_from(click.get("tabId")?.as_i64()?).ok()?;
                let page_url = match click.remove("pageUrl")? {
                    Value::String(url) if !url.is_empty() => url,
                    _ => return None,
                };
                let frame_id = click
                    .get("frameId")
                    .and_then(Value::as_i64)
                    .and_then(|id| i32::try_from(id).ok())
                    .unwrap_or(-1);
                Some(FrameClick {
                    tab_id,
                    page_url,
                    frame_id,
                })
            });
            if let Some(finish) = finish.take() {
                finish(click);
            }
        }));
    });
}

pub enum MenuEvent {
    Command(Value),
    Share(ShareData),
    Captured {
        tab: String,
        window: Retained<NSWindow>,
        link: String,
        items: Vec<Value>,
    },
}

pub struct MenuIvars {
    engine_name: RefCell<String>,
    pick: Cell<Option<String>>,
    background: RefCell<Option<Rc<RefCell<TabRegistry>>>>,
    emit: Rc<dyn Fn(MenuEvent)>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaContextMenuDelegate"]
    #[ivars = MenuIvars]
    pub struct ContextMenuDelegate;

    impl ContextMenuDelegate {
        // Section 6.4 exception: Chromium uses both menu replies before the callback returns.
        #[unsafe(method_id(tab:contextMenuItemsForSelection:))]
        fn for_selection(&self, _tab: &Tab, selection: &NSString) -> Retained<AnyObject> {
            let label = selection_label(&selection.to_string());
            let title = format!("Search {} for “{label}”", self.ivars().engine_name.borrow());
            from_json(&json!([{ "id": "search", "title": title, "replaces": "search" }]))
        }

        #[unsafe(method_id(tab:runContextMenu:))]
        fn run_menu(&self, tab: &Tab, menu: &Payload) -> Option<Retained<AnyObject>> {
            let rows = menu.objectForKey(&NSString::from_str("items"));
            if let Some(pick) = self.ivars().pick.take() {
                rows.as_deref().and_then(|rows| {
                    let rows = to_json(rows);
                    find_pick(items(&rows), &pick).as_ref().map(from_json)
                })
            } else {
                self.capture(tab, menu, rows.as_deref());
                None
            }
        }

        #[unsafe(method(tab:contextMenuCommand:text:modifiers:))]
        fn command(&self, _tab: &Tab, item: &NSString, text: Option<&NSString>, modifiers: Option<&Payload>) {
            if item.to_string() == "search" {
                self.queue(MenuEvent::Command(json!({"command": "search", "text": text.map(ToString::to_string).unwrap_or_default(), "modifiers": modifiers.map(|value| to_json(value)).unwrap_or_else(|| json!({}))})));
            }
        }

        #[unsafe(method(tab:shareFromContextMenu:))]
        fn share(&self, _tab: &Tab, item: &Payload) {
            self.queue(MenuEvent::Share(ShareData::copy(item)));
        }
    }
);

impl ContextMenuDelegate {
    pub fn new(mtm: MainThreadMarker, emit: impl Fn(MenuEvent) + 'static) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MenuIvars {
            engine_name: RefCell::new("Google".into()),
            pick: Cell::default(),
            background: RefCell::default(),
            emit: Rc::new(emit),
        });
        // SAFETY: The allocated class has NSObject as its superclass.
        unsafe { msg_send![super(this), init] }
    }

    pub fn set_search_engine(&self, name: String) {
        *self.ivars().engine_name.borrow_mut() = if name.is_empty() {
            "Google".into()
        } else {
            name
        };
    }

    pub fn set_test_pick(&self, pick: String) {
        self.ivars().pick.set(Some(pick));
    }

    pub fn enable_background_capture(&self, registry: Rc<RefCell<TabRegistry>>) {
        *self.ivars().background.borrow_mut() = Some(registry);
    }

    fn capture(&self, tab: &Tab, menu: &Payload, rows: Option<&AnyObject>) {
        let background = self.ivars().background.borrow();
        let Some(registry) = background.as_ref() else {
            return;
        };
        let registry = registry.borrow();
        let Some((tab, window)) = registry
            .app_tab_id(tab)
            .zip(registry.view_for_tab(tab).and_then(|view| view.window()))
        else {
            return;
        };
        let link = menu
            .objectForKey(&NSString::from_str("link"))
            .and_then(|link| link.downcast::<NSString>().ok())
            .map_or_else(String::new, |link| link.to_string());
        self.queue(MenuEvent::Captured {
            tab: tab.to_owned(),
            window,
            link,
            items: rows.map_or_else(Vec::new, |rows| describe_menu(items(&to_json(rows)), true)),
        });
    }

    fn queue(&self, event: MenuEvent) {
        let emit = self.ivars().emit.clone();
        main_queue::post(Box::new(move || emit(event)));
    }
}

pub struct ShareData {
    fields: serde_json::Map<String, Value>,
    image: Option<(Vec<u8>, String)>,
    point: Option<NSPoint>,
}

impl ShareData {
    fn copy(item: &Payload) -> Self {
        let field = |key| item.objectForKey(&NSString::from_str(key));
        let string = |key| {
            field(key)
                .and_then(|value| value.downcast::<NSString>().ok())
                .map(|value| value.to_string())
        };
        let fields = ["kind", "url", "title", "text"]
            .into_iter()
            .filter_map(|key| Some((key.into(), Value::String(string(key)?))))
            .collect();
        let image = field("image")
            .and_then(|value| value.downcast::<NSData>().ok())
            .map(|image| (image.to_vec(), string("mimeType").unwrap_or_default()));
        let point = field("point").and_then(|value| {
            let (x, y) = serde_json::from_value::<(f64, f64)>(to_json(&value)).ok()?;
            Some(NSPoint::new(x, y))
        });
        Self {
            fields,
            image,
            point,
        }
    }

    pub fn command(
        mut self,
        images: Option<&SharedImages>,
        root_point: impl FnOnce(NSPoint) -> Option<NSPoint>,
    ) -> Value {
        if let Some((image, mime)) = self.image
            && let Some(images) = images
            && let Ok(path) = images.share(
                &image,
                &mime,
                self.fields
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
            && let Some(path) = path.to_str()
            && let Some(url) = NSURL::fileURLWithPath(&NSString::from_str(path)).absoluteString()
        {
            self.fields.insert("fileUrl".into(), url.to_string().into());
        }
        let text = self
            .fields
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let mut payload = json!({"command": "share", "text": text, "share": self.fields});
        if let Some(point) = self.point.and_then(root_point) {
            payload["anchor"] = json!([point.x - 1.0, point.y - 1.0, 2, 2]);
        }
        payload
    }
}

pub struct SharedImages {
    own: PathBuf,
    _lock: File,
}

impl SharedImages {
    /// Remove shares whose owning process released its lock.
    pub fn remove_stale(base: &Path) {
        let root = base.join("Ria Shared Images");
        if !is_private_dir(&root) {
            return;
        }
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            if entry.file_name().to_str().is_some_and(|name| {
                name.split_once('-').is_some_and(|(pid, id)| {
                    !pid.is_empty() && pid.bytes().all(|c| c.is_ascii_digit()) && is_uuid(id)
                })
            }) {
                let _ = remove_inactive_owner(&entry.path());
            }
        }
    }

    pub fn new(base: &Path) -> io::Result<Self> {
        let root = base.join("Ria Shared Images");
        if let Err(error) = DirBuilder::new().mode(0o700).create(&root)
            && (error.kind() != io::ErrorKind::AlreadyExists || !is_private_dir(&root))
        {
            return Err(error);
        }
        Self::remove_stale(base);
        let own = root.join(format!("{}-{}", std::process::id(), unique_id()));
        DirBuilder::new().mode(0o700).create(&own)?;
        let lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_EXLOCK | libc::O_NONBLOCK)
            .open(own.join(".lock"))?;
        Ok(Self { own, _lock: lock })
    }

    pub fn share(&self, image: &[u8], mime: &str, source: &str) -> io::Result<PathBuf> {
        let mut folders: Vec<_> = fs::read_dir(&self.own)?
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if !valid_share_dir(&path) {
                    return None;
                }
                Some((entry.metadata().ok()?.created().ok()?, path))
            })
            .collect();
        folders.sort_unstable();
        for (_, path) in folders.iter().take(folders.len().saturating_sub(9)) {
            let _ = remove_share_folder(path);
        }
        let folder = self.own.join(unique_id());
        DirBuilder::new().mode(0o700).create(&folder)?;
        let ext = match mime.to_ascii_lowercase().as_str() {
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "jpg",
        };
        let base = NSURL::URLWithString(&NSString::from_str(source))
            .and_then(|url| url.lastPathComponent())
            .map_or_else(String::new, |name| {
                name.stringByDeletingPathExtension().to_string()
            });
        let base = if base == "/" { "" } else { base.trim() };
        let safe: String = base
            .chars()
            .filter(|c| !c.is_control())
            .map(|c| if matches!(c, ':' | '/') { '-' } else { c })
            .collect();
        let name = if safe.is_empty() { "Image" } else { &safe };
        let name = NSString::from_str(name);
        let name = if name.length() > 80 {
            name.substringToIndex(name.rangeOfComposedCharacterSequenceAtIndex(80).location)
        } else {
            name
        };
        let path = folder.join(format!("{name}.{ext}"));
        let written = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .and_then(|mut file| file.write_all(image));
        if let Err(error) = written {
            let _ = remove_share_folder(&folder);
            return Err(error);
        }
        main_queue::after(
            SHARE_LIFETIME,
            Box::new(move || {
                let _ = remove_share_folder(&folder);
            }),
        );
        Ok(path)
    }

    pub fn remove(&self, file: &Path) -> io::Result<()> {
        if let Some(folder) = file
            .parent()
            .filter(|folder| folder.parent() == Some(&self.own))
        {
            remove_share_folder(folder)?;
        }
        Ok(())
    }
}

fn unique_id() -> String {
    NSUUID::UUID().UUIDString().to_string()
}

fn is_private_dir(path: &Path) -> bool {
    // SAFETY: getuid has no preconditions.
    let uid = unsafe { libc::getuid() };
    fs::symlink_metadata(path)
        .is_ok_and(|info| info.is_dir() && info.uid() == uid && info.mode() & 0o077 == 0)
}

fn valid_share_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_uuid)
        && is_private_dir(path)
}

fn is_uuid(name: &str) -> bool {
    NSUUID::initWithUUIDString(objc2::AnyThread::alloc(), &NSString::from_str(name)).is_some()
}

fn remove_inactive_owner(path: &Path) -> io::Result<()> {
    if !is_private_dir(path) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let lock_path = path.join(".lock");
    let lock = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&lock_path)?;
    if !lock.metadata()?.is_file() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    lock.try_lock().map_err(io::Error::from)?;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_name() != ".lock" {
            remove_share_folder(&entry.path())?;
        }
    }
    fs::remove_file(lock_path)?;
    fs::remove_dir(path)
}

fn remove_share_folder(path: &Path) -> io::Result<()> {
    if !valid_share_dir(path) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let entries = fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
    if entries
        .iter()
        .any(|entry| !entry.file_type().is_ok_and(|kind| kind.is_file()))
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    for entry in entries {
        fs::remove_file(entry.path())?;
    }
    fs::remove_dir(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_nested_menu_by_visible_title() {
        let input = json!([{ "label": "Show", "submenu": [{ "label": "A&&B &C", "id": 42 }] }]);
        assert_eq!(
            find_pick(items(&input), " A&B C\t8 "),
            Some(json!({"command": 42, "path": [0, 0], "flags": 8}))
        );
    }

    #[test]
    fn menu_descriptions_use_engine_types_and_preserve_visibility() {
        let input = [
            json!({"id": 17, "label": "A&&&B", "type": 2, "visible": false, "enabled": true, "checked": true}),
            json!({"separator": true}),
        ];
        assert_eq!(
            describe_menu(&input, false),
            [
                json!({"id": 17, "label": "A&B", "type": 3, "visible": false, "enabled": true}),
                json!({"id": 0, "label": "", "type": 4, "visible": true, "enabled": false}),
            ],
        );
        assert_eq!(
            describe_menu(&input, true),
            [
                json!({"id": 17, "title": "A&B", "enabled": true, "hidden": true, "separator": false, "checked": true}),
                json!({"id": 0, "title": "", "enabled": false, "hidden": false, "separator": true, "checked": false}),
            ],
        );
    }

    #[test]
    fn cleanup_keeps_locked_shares_and_removes_abandoned_shares() {
        let base = std::env::temp_dir().join(unique_id());
        DirBuilder::new().mode(0o700).create(&base).unwrap();
        let live = SharedImages::new(&base).unwrap();
        let abandoned = SharedImages::new(&base).unwrap();
        let live_path = live.own.clone();
        let abandoned_path = abandoned.own.clone();
        let flat = live_path.parent().unwrap().join(unique_id());
        DirBuilder::new().mode(0o700).create(&flat).unwrap();
        let shared = live
            .share(b"cat", "image/png", "https://example.com/cat%20photo.png")
            .unwrap();
        let filename = shared.file_name().unwrap().to_owned();
        let prefix = "😀".repeat(39);
        let source = format!("https://example.com/{prefix}👩‍💻{}.png", "😀".repeat(30));
        let long = live.share(b"cat", "image/png", &source).unwrap();
        let long_filename = long.file_name().unwrap().to_owned();
        for owner in [&live_path, &abandoned_path] {
            let share = owner.join(unique_id());
            DirBuilder::new().mode(0o700).create(&share).unwrap();
            fs::write(share.join("image.png"), b"image").unwrap();
        }
        drop(abandoned);
        SharedImages::remove_stale(&base);
        let preserved_live = live_path.exists();
        let removed_abandoned = !abandoned_path.exists();
        let preserved_flat = flat.is_dir();
        drop(live);
        fs::remove_dir_all(&base).unwrap();
        assert!(preserved_live);
        assert!(removed_abandoned);
        assert!(preserved_flat);
        assert_eq!(filename, "cat photo.png");
        assert_eq!(
            long_filename.to_str(),
            Some(format!("{prefix}.png").as_str())
        );
    }
}
