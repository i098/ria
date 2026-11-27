// Ria original, no Ardia source.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use accesskit_macos::SubclassingAdapter;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSAppearanceCustomization, NSAutoresizingMaskOptions, NSColor, NSColorSpace,
    NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectView,
    NSWindowOrderingMode, NSWorkspace,
};
use objc2_foundation::{
    NSArray, NSPoint, NSRect, NSRunLoop, NSRunLoopCommonModes, NSSize, ns_string,
};
use objc2_quartz_core::{CADisplayLink, CAMetalLayer, CATransaction};
use ria_engine::call::{ActionQueue, main_queue};
use rune::accesskit::{ActionHandler, ActionRequest, ActivationHandler, TreeUpdate};
use rune::embed::LayerHost;
use rune::{AccessibilityLink, AppSpec, Color, Runtime, SystemAccent};

use crate::input::{Input, TextInputSnapshot};
use crate::view::{RuneView, ViewHost, Viewport, boundary};

type TextSnapshot<S> = fn(&Runtime<S>) -> Option<Rc<TextInputSnapshot>>;

/// Rune views for one window. `S` is the Rune state of each layer: the chrome and the overlay each get a clone, so
/// a clone that shares data (an `Rc<RefCell<..>>`) is shared state, and a plain field, such as a `TextEdit` that a
/// text field binds with `on_edit`, belongs to its layer. Reach the chrome's own copy with `WindowHost::edit_chrome`.
pub struct WindowSpec<S: 'static> {
    pub chrome: AppSpec<S>,
    pub overlay: AppSpec<S>,
    /// Key of the chrome element that defines the native page's rectangle.
    pub page_slot: &'static str,
    /// Returns the text AppKit can query for reconversion, if a field has focus.
    pub text_snapshot: TextSnapshot<S>,
}

/// Owns the native root, the two Rune hosts and a single display link.
/// Drop it before the engine shuts down. Use the application's main-thread action queue.
pub struct WindowHost<S: 'static> {
    link: Retained<CADisplayLink>,
    core: Rc<WindowCore<S>>,
}

impl<S: 'static> WindowHost<S> {
    /// Installs the surface in a window's content view. The view must already belong to the window.
    pub fn new(
        container: &NSView,
        state: S,
        spec: WindowSpec<S>,
        queue: Rc<ActionQueue>,
    ) -> Result<Self, String>
    where
        S: Clone,
    {
        let window = container
            .window()
            .ok_or("The surface container has no window")?;
        let mtm = container.mtm();
        let bounds = container.bounds();
        let size = local_rect(bounds.size);
        let root = NSView::initWithFrame(mtm.alloc(), bounds);
        root.setAutoresizingMask(resize_mask());
        let glass = NSVisualEffectView::initWithFrame(mtm.alloc(), size);
        glass.setAutoresizingMask(resize_mask());
        glass.setMaterial(NSVisualEffectMaterial::Sidebar);
        glass.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        root.addSubview(&glass);

        let settings = Settings::read(container);
        let mount = Mount {
            mtm,
            viewport: Viewport {
                bounds: size,
                scale: window.backingScaleFactor(),
            },
            settings,
        };
        let chrome = Layer::new(
            mount,
            false,
            state.clone(),
            &spec.chrome,
            spec.text_snapshot,
            queue.clone(),
        )?;
        let overlay = Layer::new(
            mount,
            true,
            state,
            &spec.overlay,
            spec.text_snapshot,
            queue.clone(),
        )?;
        overlay.view.setHidden(true);
        root.addSubview(&chrome.view);
        root.addSubview(&overlay.view);
        let core = Rc::new(WindowCore {
            root,
            chrome,
            overlay,
            queue,
            page_slot: spec.page_slot,
            page: RefCell::new(None),
            mounted: RefCell::new(Vec::new()),
            tick_pending: Cell::new(false),
            presentation_time: Cell::new(None),
            settings: Cell::new(settings),
        });
        core.update_ax_order();
        container.addSubview(&core.root);
        core.chrome.view.capture_viewport();
        core.overlay.view.capture_viewport();

        let weak = Rc::downgrade(&core);
        let target = DisplayTarget::new(mtm, move |time| {
            if let Some(core) = weak.upgrade() {
                core.enqueue_tick(time);
            }
        });
        // SAFETY: macOS 14+ supplies this NSView selector. DisplayTarget implements its callback.
        let link: Retained<CADisplayLink> = unsafe {
            msg_send![&*core.root, displayLinkWithTarget: &*target, selector: sel!(tick:)]
        };
        // SAFETY: The link belongs to the view's main thread and uses the common run-loop modes.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        Ok(Self { link, core })
    }

    /// Shows the given engine page view. Pass `None` when no page is selected.
    ///
    /// Changes from Ardia: Ardia mounts every tab's page in a sibling pane. Here the page that was shown stays
    /// mounted in the same slot, unhidden, until `retain_pages` drops it, so the next switch finds its page mounted.
    pub fn set_page(&self, view: Option<&NSView>) {
        let same = self.core.page.borrow().as_deref().map(std::ptr::from_ref)
            == view.map(std::ptr::from_ref);
        if same && view.is_none_or(|view| self.core.mounted(view)) {
            return;
        }
        self.core.page.take();
        if let Some(view) = view {
            self.core.mount(view);
            *self.core.page.borrow_mut() = Some(view.retain());
        }
        self.core
            .place_page(self.core.chrome.slot(self.core.page_slot));
        self.core.update_ax_order();
    }

    /// Mounts `view` in the page slot beside the shown page. A mounted view stays where it is.
    pub fn mount_page(&self, view: &NSView) {
        if !self.core.mounted(view) {
            self.core.mount(view);
            self.core
                .place_page(self.core.chrome.slot(self.core.page_slot));
        }
    }

    /// Unmounts every mounted page that is not in `keep`. The shown page is kept too.
    pub fn retain_pages(&self, keep: &[&NSView]) {
        let shown = self.core.page.borrow().clone();
        self.core.mounted.borrow_mut().retain(|view| {
            let kept = keep.iter().any(|k| std::ptr::eq(*k, &**view))
                || shown.as_deref().is_some_and(|s| std::ptr::eq(s, &**view));
            if !kept {
                view.removeFromSuperview();
            }
            kept
        });
    }

    /// The number of pages mounted in the page slot, shown or hidden.
    pub fn mounted_pages(&self) -> usize {
        self.core.mounted.borrow().len()
    }

    /// Hides and stops drawing the overlay when there is no overlay content.
    pub fn set_overlay_visible(&self, visible: bool) {
        self.core.overlay.view.setHidden(!visible);
        if visible {
            self.core.overlay.invalidate();
        }
        self.core.update_ax_order();
    }

    /// Marks each runtime whose input state changed during the application's drain.
    pub fn mark_dirty(&self, chrome: bool, overlay: bool) {
        if chrome {
            self.core.chrome.invalidate();
        }
        if overlay {
            self.core.overlay.invalidate();
        }
    }

    /// Runs `edit` on the chrome layer's own copy of the state and redraws the chrome. Returns None, and does not
    /// run `edit`, while the layer handles input or a frame.
    pub fn edit_chrome<R>(&self, edit: impl FnOnce(&mut S) -> R) -> Option<R> {
        let result = {
            let mut rune = self.core.chrome.entry.rune.try_borrow_mut().ok()?;
            edit(&mut rune.as_mut()?.runtime_mut().state)
        };
        self.core.chrome.invalidate();
        Some(result)
    }

    /// The latest display link target timestamp processed by the queue.
    pub fn presentation_time(&self) -> Option<f64> {
        self.core.presentation_time.get()
    }

    /// The number of synchronous AppKit queries that needed a cached answer.
    pub fn reentry_miss(&self) -> u64 {
        self.core.chrome.entry.reentry_miss.get() + self.core.overlay.entry.reentry_miss.get()
    }
}

impl<S: 'static> Drop for WindowHost<S> {
    fn drop(&mut self) {
        self.link.invalidate();
        self.core.root.removeFromSuperview();
    }
}

struct WindowCore<S: 'static> {
    root: Retained<NSView>,
    chrome: Layer<S>,
    overlay: Layer<S>,
    queue: Rc<ActionQueue>,
    page: RefCell<Option<Retained<NSView>>>,
    mounted: RefCell<Vec<Retained<NSView>>>,
    page_slot: &'static str,
    tick_pending: Cell<bool>,
    presentation_time: Cell<Option<f64>>,
    settings: Cell<Settings>,
}

impl<S: 'static> WindowCore<S> {
    fn enqueue_tick(self: &Rc<Self>, time: f64) {
        if self.tick_pending.replace(true) {
            return;
        }
        let core = Rc::downgrade(self);
        self.queue.push(Box::new(move || {
            if let Some(core) = core.upgrade() {
                core.tick_pending.set(false);
                core.tick(time);
            }
        }));
    }

    fn tick(&self, time: f64) {
        self.presentation_time.set(Some(time));
        let settings = Settings::read(&self.root);
        if self.settings.replace(settings) != settings {
            self.chrome.apply_settings(settings);
            self.overlay.apply_settings(settings);
        }
        self.chrome.resize();
        self.overlay.resize();
        if self.chrome.frame("Browser chrome") {
            self.place_page(self.chrome.slot(self.page_slot));
        }
        if !self.overlay.view.isHidden() {
            self.overlay.frame("Browser overlay");
        }
    }

    fn mounted(&self, view: &NSView) -> bool {
        self.mounted
            .borrow()
            .iter()
            .any(|v| std::ptr::eq(&**v, view))
    }

    fn mount(&self, view: &NSView) {
        if self.mounted(view) {
            return;
        }
        self.root.addSubview_positioned_relativeTo(
            view,
            NSWindowOrderingMode::Above,
            Some(&self.chrome.view),
        );
        self.mounted.borrow_mut().push(view.retain());
    }

    fn place_page(&self, slot: Option<NSRect>) {
        let frame = slot.map_or(NSRect::ZERO, |slot| {
            self.chrome.view.convertRect_toView(slot, Some(&self.root))
        });
        for page in self.mounted.borrow().iter() {
            if page.frame() != frame {
                page.setFrame(frame);
            }
        }
    }

    fn update_ax_order(&self) {
        let page = self.page.borrow().clone();
        let mut children: [&AnyObject; 3] =
            [&self.chrome.view, &self.chrome.view, &self.chrome.view];
        let mut count = 1;
        if let Some(page) = page.as_ref() {
            children[count] = page;
            count += 1;
        }
        if !self.overlay.view.isHidden() {
            children[count] = &self.overlay.view;
            count += 1;
        }
        let children = NSArray::from_slice(&children[..count]);
        // SAFETY: NSAccessibility accepts the accessible NSView objects in this array.
        unsafe { self.root.setAccessibilityChildren(Some(&children)) };
    }
}

struct Layer<S: 'static> {
    entry: Rc<LayerEntry<S>>,
    view: Retained<RuneView>,
}

#[derive(Clone, Copy)]
struct Mount {
    mtm: MainThreadMarker,
    viewport: Viewport,
    settings: Settings,
}

impl<S: 'static> Layer<S> {
    fn new(
        mount: Mount,
        overlay: bool,
        state: S,
        spec: &AppSpec<S>,
        text_snapshot: TextSnapshot<S>,
        queue: Rc<ActionQueue>,
    ) -> Result<Self, String> {
        let Mount {
            mtm,
            viewport,
            settings,
        } = mount;
        let Viewport {
            bounds: frame,
            scale,
        } = viewport;
        let entry = Rc::new(LayerEntry {
            rune: RefCell::new(None),
            adapter: RefCell::new(None),
            a11y: RefCell::new(AccessibilityLink::new()),
            surface: RefCell::new(None),
            caret: Cell::new(None),
            snapshot: RefCell::new(None),
            text_snapshot,
            reentry_miss: Cell::new(0),
            dirty: Cell::new(true),
            wake: Arc::new(AtomicBool::new(false)),
            deadline: Cell::new(None),
        });
        let weak: Weak<LayerEntry<S>> = Rc::downgrade(&entry);
        let view = RuneView::new(mtm, frame, overlay, weak, queue.clone());
        view.setAutoresizingMask(resize_mask());
        let metal = view.metal_layer();
        match_layer(&metal, viewport);

        let wake = entry.wake.clone();
        let waker = Arc::new(move || {
            let wake = wake.clone();
            main_queue::post(Box::new(move || wake.store(true, Ordering::Relaxed)));
        });
        // SAFETY: `metal` is the view's CAMetalLayer. LayerEntry retains it until after Rune is dropped.
        let mut rune = unsafe {
            LayerHost::from_core_animation_layer(
                Retained::as_ptr(&metal).cast_mut().cast::<c_void>(),
                spec,
                state,
                (frame.size.width as f32, frame.size.height as f32),
                scale as f32,
                settings.dark,
                waker,
            )?
        };
        rune.runtime_mut().transparent = true;
        settings.apply(rune.runtime_mut());
        // SAFETY: Set the native compositing mode after Rune configures the swap chain.
        metal.setOpaque(false);
        entry.rune.replace(Some(rune));
        entry.surface.replace(Some(metal));
        // SAFETY: `view` is unreleased and this adapter attaches before the view is shown.
        let adapter = unsafe {
            SubclassingAdapter::new(
                Retained::as_ptr(&view).cast_mut().cast(),
                AxActivate {
                    layer: Rc::downgrade(&entry),
                    queue: queue.clone(),
                },
                AxAction {
                    layer: Rc::downgrade(&entry),
                    queue,
                },
            )
        };
        entry.adapter.replace(Some(adapter));
        Ok(Self { entry, view })
    }

    fn invalidate(&self) {
        if let Some(rune) = self.entry.rune.borrow_mut().as_mut() {
            rune.runtime_mut().invalidate();
            self.entry.dirty.set(true);
        }
    }

    fn apply_settings(&self, settings: Settings) {
        if let Some(rune) = self.entry.rune.borrow_mut().as_mut() {
            settings.apply(rune.runtime_mut());
            self.entry.dirty.set(true);
        }
    }

    fn slot(&self, key: &str) -> Option<NSRect> {
        let guard = self.entry.rune.borrow();
        let ([x, y, width, height], _) = guard.as_ref()?.runtime().presented(key)?;
        Some(NSRect::new(
            NSPoint::new(x.into(), y.into()),
            NSSize::new(width.into(), height.into()),
        ))
    }

    fn resize(&self) {
        let Some(viewport) = self.view.take_viewport() else {
            return;
        };
        let surface = self.entry.surface.borrow().clone();
        let Some(surface) = surface else { return };
        match_layer(&surface, viewport);
        if let Some(rune) = self.entry.rune.borrow_mut().as_mut() {
            rune.resize(
                (
                    viewport.bounds.size.width as f32,
                    viewport.bounds.size.height as f32,
                ),
                viewport.scale as f32,
            );
        }
        // SAFETY: Rune can reset opacity when it reconfigures the swap chain.
        surface.setOpaque(false);
        self.entry.dirty.set(true);
    }

    fn frame(&self, name: &str) -> bool {
        let entry = &self.entry;
        let should_draw = entry.dirty.replace(false) | entry.wake.swap(false, Ordering::Relaxed);
        if !should_draw
            && !entry
                .deadline
                .get()
                .is_some_and(|time| time <= Instant::now())
        {
            return false;
        }
        let (frame, tree) = {
            let mut guard = entry.rune.borrow_mut();
            let Some(rune) = guard.as_mut() else {
                return false;
            };
            let frame = rune.frame();
            let runtime = rune.runtime_mut();
            entry.caret.set(runtime.ime_area);
            let snapshot = if runtime.has_secure_focus() {
                None
            } else {
                (entry.text_snapshot)(runtime)
            };
            entry.snapshot.replace(snapshot);
            let tree = entry.a11y.borrow_mut().service(runtime, name);
            (frame, tree)
        };
        // AppKit can give us a display tick before Metal has a drawable.
        entry.dirty.set(frame.animating || !frame.drew);
        entry.deadline.set(
            frame
                .wake_after_secs
                .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
                .and_then(|delay| Instant::now().checked_add(delay)),
        );
        if let Some(tree) = tree {
            let events = entry
                .adapter
                .borrow_mut()
                .as_mut()
                .and_then(|adapter| adapter.update_if_active(|| tree));
            if let Some(events) = events {
                events.raise();
            }
        }
        true
    }
}

struct LayerEntry<S: 'static> {
    rune: RefCell<Option<LayerHost<S>>>,
    adapter: RefCell<Option<SubclassingAdapter>>,
    a11y: RefCell<AccessibilityLink>,
    surface: RefCell<Option<Retained<CAMetalLayer>>>,
    caret: Cell<Option<[f32; 4]>>,
    snapshot: RefCell<Option<Rc<TextInputSnapshot>>>,
    text_snapshot: TextSnapshot<S>,
    reentry_miss: Cell<u64>,
    dirty: Cell<bool>,
    wake: Arc<AtomicBool>,
    deadline: Cell<Option<Instant>>,
}

impl<S: 'static> LayerEntry<S> {
    fn runtime<R>(&self, read: impl FnOnce(&Runtime<S>) -> R) -> Option<R> {
        match self.rune.try_borrow() {
            Ok(host) => host.as_ref().map(|host| read(host.runtime())),
            Err(_) => {
                self.reentry_miss
                    .set(self.reentry_miss.get().saturating_add(1));
                None
            }
        }
    }
}

impl<S: 'static> ViewHost for LayerEntry<S> {
    fn hit(&self, x: f32, y: f32) -> bool {
        self.runtime(|rt| rt.element_at(x, y).is_some())
            .unwrap_or(false)
    }

    fn caret(&self) -> Option<[f32; 4]> {
        self.runtime(|rt| rt.ime_area)
            .unwrap_or_else(|| self.caret.get())
    }

    fn snapshot(&self) -> Option<Rc<TextInputSnapshot>> {
        self.runtime(|rt| {
            if rt.has_secure_focus() {
                None
            } else {
                self.snapshot.borrow().clone()
            }
        })
        .unwrap_or_else(|| self.snapshot.borrow().clone())
    }

    fn handle(&self, input: Input) {
        let focus = match &input {
            Input::Focus(focused) => Some(*focused),
            _ => None,
        };
        if let Some(rune) = self.rune.borrow_mut().as_mut() {
            input.send(rune.runtime_mut());
            self.dirty.set(true);
        }
        if let Some(focused) = focus {
            let events = self
                .adapter
                .borrow_mut()
                .as_mut()
                .and_then(|adapter| adapter.update_view_focus_state(focused));
            if let Some(events) = events {
                events.raise();
            }
        }
    }
}

struct AxActivate<S: 'static> {
    layer: Weak<LayerEntry<S>>,
    queue: Rc<ActionQueue>,
}

impl<S: 'static> ActivationHandler for AxActivate<S> {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        boundary(|| {
            let layer = self.layer.clone();
            self.queue.push(Box::new(move || {
                if let Some(layer) = layer.upgrade() {
                    layer.a11y.borrow().activate();
                    layer.dirty.set(true);
                }
            }));
            None
        })
    }
}

struct AxAction<S: 'static> {
    layer: Weak<LayerEntry<S>>,
    queue: Rc<ActionQueue>,
}

impl<S: 'static> ActionHandler for AxAction<S> {
    fn do_action(&mut self, request: ActionRequest) {
        boundary(|| {
            let layer = self.layer.clone();
            self.queue.push(Box::new(move || {
                if let Some(layer) = layer.upgrade()
                    && let Some(host) = layer.rune.borrow_mut().as_mut()
                {
                    let rt = host.runtime_mut();
                    // Pinned Rune packs its element id in the upper bits of each AccessKit id;
                    // `accessibility_action` finds the node by the element's key (its author id),
                    // not by its path.
                    if let Some((_, Some(key))) =
                        rt.element_path(rune::NodeId(request.target_node.0 >> 16))
                    {
                        let key = key.to_owned();
                        if rt.accessibility_action(&key, request.action, request.data) {
                            layer.dirty.set(true);
                        }
                    }
                }
            }));
        });
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Settings {
    dark: bool,
    reduced: bool,
    accent: Option<[f32; 3]>,
}

impl Settings {
    fn read(view: &NSView) -> Self {
        let appearance = view.effectiveAppearance();
        let name = appearance.name();
        let dark = *name == *ns_string!("NSAppearanceNameDarkAqua")
            || *name == *ns_string!("NSAppearanceNameAccessibilityHighContrastDarkAqua");
        let accent = NSColor::controlAccentColor()
            .colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())
            .map(|color| {
                [
                    color.redComponent() as f32,
                    color.greenComponent() as f32,
                    color.blueComponent() as f32,
                ]
            });
        let reduced = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
        Self {
            dark,
            reduced,
            accent,
        }
    }

    fn apply<S: 'static>(self, runtime: &mut Runtime<S>) {
        runtime.set_system_dark(self.dark);
        runtime.set_system_reduced_motion(self.reduced);
        if let Some([r, g, b]) = self.accent {
            let decode = |x: f32| {
                let x = x.clamp(0.0, 1.0);
                if x <= 0.04045 {
                    x / 12.92
                } else {
                    ((x + 0.055) / 1.055).powf(2.4)
                }
            };
            let color = Color {
                r: decode(r),
                g: decode(g),
                b: decode(b),
                a: 1.0,
            };
            runtime.set_system_accent(SystemAccent::new(color, color));
        }
    }
}

fn match_layer(layer: &CAMetalLayer, viewport: Viewport) {
    // Suppress implicit animation during a live resize.
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    layer.setFrame(viewport.bounds);
    layer.setContentsScale(viewport.scale);
    CATransaction::commit();
}

fn local_rect(size: NSSize) -> NSRect {
    NSRect::new(NSPoint::ZERO, size)
}

fn resize_mask() -> NSAutoresizingMaskOptions {
    NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable
}

pub(crate) struct DisplayIvars(Box<dyn Fn(f64)>);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaSurfaceDisplayTarget"]
    #[ivars = DisplayIvars]
    pub(crate) struct DisplayTarget;

    impl DisplayTarget {
        #[unsafe(method(tick:))]
        fn tick(&self, link: &CADisplayLink) {
            boundary(|| {
                let time = link.targetTimestamp();
                (self.ivars().0)(time);
            });
        }
    }

    unsafe impl NSObjectProtocol for DisplayTarget {}
);

impl DisplayTarget {
    fn new(mtm: MainThreadMarker, tick: impl Fn(f64) + 'static) -> Retained<Self> {
        let target = mtm.alloc::<Self>().set_ivars(DisplayIvars(Box::new(tick)));
        // SAFETY: All ivars are initialized and the superclass initializer takes no arguments.
        unsafe { msg_send![super(target), init] }
    }
}
