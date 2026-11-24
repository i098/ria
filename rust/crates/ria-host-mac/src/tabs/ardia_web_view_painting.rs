// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Painting.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObjectProtocol};
use objc2_app_kit::NSView;
use objc2_foundation::{NSArray, NSRect};

use crate::core::bindings::Tab;

mod native;
pub use native::{ContentDrift, take_content_drift};

pub fn render_widget_in(page: &NSView) -> Option<Retained<NSView>> {
    let class = AnyClass::get(c"RenderWidgetHostViewCocoa")?;
    if page.isKindOfClass(class) {
        return Some(objc2::Message::retain(page));
    }
    let mut queue: VecDeque<_> = page.subviews().to_vec().into();
    while let Some(view) = queue.pop_front() {
        if view.isKindOfClass(class) {
            return Some(view);
        }
        queue.extend(view.subviews().to_vec());
    }
    None
}

pub struct FrameMark(Vec<(Retained<AnyObject>, Option<Retained<AnyObject>>)>);

impl PartialEq for FrameMark {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().zip(&other.0).all(
                |((layer, content), (other_layer, other_content))| {
                    Retained::as_ptr(layer) == Retained::as_ptr(other_layer)
                        && content.as_ref().map(Retained::as_ptr)
                            == other_content.as_ref().map(Retained::as_ptr)
                },
            )
    }
}

pub fn frame_mark(widget: &NSView) -> FrameMark {
    let mut mark = Vec::new();
    if let Some(layer) = widget.layer() {
        // SAFETY: The layer and its descendants implement CALayer's sublayers and contents selectors.
        unsafe {
            let trees: Option<Retained<NSArray<AnyObject>>> = msg_send![&*layer, sublayers];
            if let Some(trees) = trees {
                for tree in &*trees {
                    let layers: Option<Retained<NSArray<AnyObject>>> = msg_send![&*tree, sublayers];
                    if let Some(layers) = layers {
                        for layer in &*layers {
                            let contents = msg_send![&*layer, contents];
                            mark.push((layer, contents));
                        }
                    }
                }
            }
        }
    }
    FrameMark(mark)
}

pub fn layout_page(
    host: &NSView,
    tab: Option<&Tab>,
    fronted: Option<&NSView>,
    stand_in: Option<&NSView>,
    dev_tools: Option<&NSView>,
    paints: bool,
) {
    let bounds = host.bounds();
    let attached =
        |view: &NSView| super::superview(view).is_some_and(|parent| std::ptr::eq(&*parent, host));
    for view in [fronted, stand_in]
        .into_iter()
        .flatten()
        .filter(|view| attached(view))
    {
        view.setFrame(bounds);
    }
    let Some(tab) = tab else { return };
    let Some(page) = tab.view() else { return };
    let empty = |rect: NSRect| rect.size.width <= 0.0 || rect.size.height <= 0.0;
    if !attached(&page) || (empty(bounds) && !empty(page.frame()) && !paints) {
        return;
    }
    let mut page_frame = bounds;
    if let Some(dev_tools) = dev_tools {
        let mut dev_tools_frame = NSRect::ZERO;
        // SAFETY: Both frame pointers refer to live, distinct stack values.
        unsafe {
            tab.dev_tools_layout_for_size(bounds.size, &mut dev_tools_frame, &mut page_frame)
        };
        dev_tools.setFrame(dev_tools_frame);
    }
    page.setFrame(page_frame);
    native::fit_render_widgets(&page);
}

#[derive(Default)]
pub struct NeighbourWake {
    committed: Option<Instant>,
    speculative: Option<Instant>,
}

impl NeighbourWake {
    pub fn until(&self) -> Option<Instant> {
        self.committed.max(self.speculative)
    }

    pub fn wake(&mut self, now: Instant, duration: Duration, committed: bool) -> bool {
        let asleep = self.until().is_none_or(|until| until <= now);
        let slot = if committed {
            &mut self.committed
        } else {
            &mut self.speculative
        };
        *slot = (*slot).max(Some(now + duration));
        asleep
    }

    pub fn release(&mut self, now: Instant) -> bool {
        if self.speculative.is_none_or(|until| until <= now) {
            return false;
        }
        self.speculative = None;
        self.committed.is_none_or(|until| until <= now)
    }
}

pub const LEAVE_SCREEN_DELAY: Duration = Duration::from_millis(100);
pub const FRAME_HOLD_TIMEOUT: Duration = Duration::from_millis(250);
pub const HOLDER_BACKSTOP: Duration = Duration::from_millis(300);
pub const SWAP_REPORT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
pub struct FrameHold {
    awaiting: Option<(u64, FrameMark)>,
    generation: u64,
}

impl FrameHold {
    pub fn begin(&mut self, widget: &NSView) -> Option<u64> {
        if self.awaiting.is_some() || !widget.isHidden() {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.awaiting = Some((self.generation, frame_mark(widget)));
        Some(self.generation)
    }

    pub fn awaiting(&self) -> bool {
        self.awaiting.is_some()
    }

    pub fn frame_changed(&self, tab: &Tab) -> bool {
        self.awaiting.as_ref().is_some_and(|(_, mark)| {
            tab.view()
                .and_then(|view| render_widget_in(&view))
                .is_none_or(|widget| frame_mark(&widget) != *mark)
        })
    }

    pub fn finish(&mut self, generation: u64) -> bool {
        if self
            .awaiting
            .as_ref()
            .is_none_or(|(current, _)| *current != generation)
        {
            return false;
        }
        self.awaiting = None;
        true
    }
}

#[derive(Default)]
pub struct PaintingVisibility {
    pub visible: bool,
    pub warm: bool,
    pub neighbour_asleep: bool,
    pub leaving: bool,
    pub revealing: bool,
    pub holding: bool,
    pub awaiting_frame: bool,
    pub showing_stand_in: bool,
}

impl PaintingVisibility {
    pub fn paints(&self) -> bool {
        self.visible
            || (self.warm && !self.neighbour_asleep)
            || self.leaving
            || self.revealing
            || self.holding
    }

    pub fn alpha(&self) -> f64 {
        if self.awaiting_frame && !self.showing_stand_in {
            return 0.0;
        }
        if self.visible || self.revealing || self.holding || !(self.warm || self.leaving) {
            1.0
        } else {
            0.0
        }
    }

    pub fn apply(&self, host: &NSView) {
        host.setAlphaValue(self.alpha());
        let hidden = !self.paints();
        for child in host.subviews().iter() {
            child.setHidden(hidden);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releasing_speculative_wake_preserves_committed_deadline() {
        let now = Instant::now();
        let mut wake = NeighbourWake::default();
        assert!(wake.wake(now, Duration::from_secs(1), true));
        assert!(!wake.wake(now, Duration::from_secs(2), false));
        assert!(!wake.release(now));
        assert_eq!(wake.until(), Some(now + Duration::from_secs(1)));
    }
}
