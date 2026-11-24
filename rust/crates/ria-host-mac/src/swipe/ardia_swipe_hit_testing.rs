// Ported from ardia packages/ardiacore/ios/ArdiaSwipeHitTesting.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use objc2::Message;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSScrollView, NSView, NSWindow};
use objc2_foundation::{NSPoint, NSRect};
use serde_json::{Value, json};

use super::ardia_swipe::{hooks, rect_value, targets};
use super::ardia_swipe_internal::{
    LEGACY_HIT_ROUTING, WRONG_RENDERER_HIT, pointer, same, superview, tracing, window_number,
};
use super::swipe_area::SwipeArea;
use crate::core::bindings::PageView;

pub fn point_in_rect(point: NSPoint, rect: NSRect) -> bool {
    point.x >= rect.min().x
        && point.x < rect.max().x
        && point.y >= rect.min().y
        && point.y < rect.max().y
}

fn class_name(object: &AnyObject) -> String {
    object.class().name().to_string_lossy().into_owned()
}

fn in_window(view: &NSView) -> Value {
    rect_value(view.convertRect_toView(view.bounds(), None))
}

fn descends(view: &NSView, ancestor: Option<&NSView>) -> bool {
    ancestor.is_some_and(|ancestor| view.isDescendantOf(ancestor))
}

fn contains(view: &NSView, location: NSPoint) -> bool {
    point_in_rect(view.convertPoint_fromView(location, None), view.bounds())
}

pub fn hit_view(window: &NSWindow, location_in_window: NSPoint) -> Option<Retained<NSView>> {
    let content = window.contentView()?;
    superview(&content)
        .unwrap_or(content)
        .hitTest(location_in_window)
}

fn miss(
    target: &SwipeArea,
    window: &NSWindow,
    location: NSPoint,
    hit: &NSView,
) -> Option<&'static str> {
    if !same(target.window().as_deref(), Some(window)) {
        return Some("otherWindow");
    }
    let Some(container) = superview(target) else {
        return Some("noSuperview");
    };
    if target.isHiddenOrHasHiddenAncestor() {
        Some("hidden")
    } else if !hit.isDescendantOf(&container) {
        Some("hitOutsideContainer")
    } else if !contains(target, location) {
        Some("pointOutside")
    } else {
        None
    }
}

fn better(target: &SwipeArea, best: Option<&SwipeArea>) -> bool {
    best.is_none_or(|best| {
        superview(target).is_some_and(|container| descends(&container, superview(best).as_deref()))
    })
}

pub fn target_at(
    window: &NSWindow,
    location_in_window: NSPoint,
    hit: Option<&NSView>,
    mut why: Option<&mut Vec<Value>>,
) -> Option<Retained<SwipeArea>> {
    let hit = hit?;
    let mut best = None;
    for target in targets() {
        let miss = miss(&target, window, location_in_window, hit);
        if let Some(why) = why.as_deref_mut() {
            why.push(json!({
                "target": pointer(&*target),
                "window": window_number(target.window().as_deref()),
                "inWindow": in_window(&target),
                "pager": target.is_pager(),
                "miss": miss.unwrap_or(""),
            }));
        }
        if miss.is_none() && better(&target, best.as_deref()) {
            best = Some(target);
        }
    }
    best
}

pub fn hit_chain(hit: Option<&NSView>) -> Value {
    let mut chain = Vec::new();
    let mut view = hit.map(|hit| hit.retain());
    while let Some(v) = view {
        chain.push(json!({
            "class": class_name(&v),
            "frame": rect_value(v.frame()),
            "inWindow": in_window(&v),
            "hidden": v.isHidden(),
        }));
        view = superview(&v);
    }
    Value::Array(chain)
}

pub fn is_pager(target: &SwipeArea) -> bool {
    target.is_pager()
}

pub fn has_native_pager(target: &SwipeArea) -> bool {
    is_pager(target) && target.has_native_pager()
}

pub fn first_renderer(view: &NSView) -> Option<Retained<NSView>> {
    if PageView::is_page_view(view) {
        return Some(view.retain());
    }
    view.subviews().iter().find_map(|sub| first_renderer(&sub))
}

pub fn react_root(view: &NSView, window: &NSWindow) -> Retained<NSView> {
    if let Some(root) = hooks().and_then(|hooks| hooks.root_view(window))
        && view.isDescendantOf(&root)
    {
        return root;
    }
    let content = window.contentView();
    let mut view = view.retain();
    while let Some(parent) =
        superview(&view).filter(|parent| !same(Some(&**parent), content.as_deref()))
    {
        view = parent;
    }
    view
}

fn misrouted_candidate(
    target: &SwipeArea,
    window: &NSWindow,
    location: NSPoint,
    root: &NSView,
) -> bool {
    same(target.window().as_deref(), Some(window))
        && superview(target).is_some()
        && !target.isHiddenOrHasHiddenAncestor()
        && is_pager(target)
        && contains(target, location)
        && std::ptr::eq(&*react_root(target, window), root)
}

pub fn misrouted_renderer_target(
    window: &NSWindow,
    location: NSPoint,
    hit: Option<&NSView>,
) -> Option<Retained<SwipeArea>> {
    let hit = hit.filter(|hit| PageView::is_page_view(hit) && !contains(hit, location))?;
    let root = react_root(hit, window);
    let mut best = None;
    for target in targets() {
        if misrouted_candidate(&target, window, location, &root) && better(&target, best.as_deref())
        {
            best = Some(target);
        }
    }
    best
}

/// The routed hit, its target, and whether the target came from the misrouted-renderer fallback.
pub type Resolved = (Option<Retained<NSView>>, Option<Retained<SwipeArea>>, bool);

pub fn resolve_hit(window: Option<&NSWindow>, location: NSPoint) -> Resolved {
    let Some(window) = window else {
        return (None, None, false);
    };
    let mut hit = hit_view(window, location);
    if tracing() && WRONG_RENDERER_HIT.get() {
        hit = window
            .contentView()
            .and_then(|content| first_renderer(&content))
            .or(hit);
    }
    let target = target_at(window, location, hit.as_deref(), None);
    if target.is_some() || (tracing() && LEGACY_HIT_ROUTING.get()) {
        return (hit, target, false);
    }
    match fallback_hit(window, location, hit.as_deref()) {
        Some((inner, pager)) => (Some(inner), Some(pager), true),
        None => (hit, None, false),
    }
}

fn fallback_hit(
    window: &NSWindow,
    location: NSPoint,
    hit: Option<&NSView>,
) -> Option<(Retained<NSView>, Retained<SwipeArea>)> {
    let pager = misrouted_renderer_target(window, location, hit)?;
    let container = superview(&pager)?;
    let point = superview(&container).map_or(NSPoint::new(0.0, 0.0), |outer| {
        outer.convertPoint_fromView(location, None)
    });
    let inner = container.hitTest(point)?;
    if PageView::is_page_view(&inner)
        || !same(inner.window().as_deref(), Some(window))
        || !inner.isDescendantOf(&container)
        || !contains(&inner, location)
    {
        return None;
    }
    let routed = target_at(window, location, Some(&inner), None)?;
    std::ptr::eq(&*routed, &*pager).then_some((inner, pager))
}

fn scroll_views(hit: Option<&NSView>, container: Option<&NSView>) -> Vec<Retained<NSScrollView>> {
    let mut found = Vec::new();
    let mut view = hit.map(|hit| hit.retain());
    while let Some(v) = view.filter(|v| !same(Some(&**v), container)) {
        if let Some(scroll) = v.downcast_ref::<NSScrollView>() {
            found.push(scroll.retain());
        }
        view = superview(&v);
    }
    found
}

fn visible_and_width(scroll: &NSScrollView) -> (NSRect, f64) {
    let visible = scroll.contentView().bounds();
    let width = scroll
        .documentView()
        .map_or(0.0, |document| document.frame().size.width);
    (visible, width)
}

fn blocks(visible: NSRect, width: f64, direction: i32) -> bool {
    if direction > 0 {
        visible.min().x > 0.5
    } else {
        visible.max().x < width - 0.5
    }
}

pub fn native_content_scrolls(
    hit: Option<&NSView>,
    container: Option<&NSView>,
    direction: i32,
) -> bool {
    scroll_views(hit, container).iter().any(|scroll| {
        let (visible, width) = visible_and_width(scroll);
        width > visible.size.width + 0.5 && blocks(visible, width, direction)
    })
}

pub fn native_scroll_report(hit: Option<&NSView>, container: Option<&NSView>) -> Value {
    Value::Array(
        scroll_views(hit, container)
            .iter()
            .map(|scroll| scroll_entry(scroll))
            .collect(),
    )
}

fn scroll_entry(scroll: &NSScrollView) -> Value {
    let (visible, width) = visible_and_width(scroll);
    let fits = width <= visible.size.width + 0.5;
    let document = scroll.documentView();
    let wrapper = superview(scroll);
    let wrapper_frame = wrapper
        .as_ref()
        .map_or(NSRect::ZERO, |wrapper| wrapper.frame());
    json!({
        "class": class_name(scroll),
        "frame": rect_value(scroll.frame()),
        "inWindow": in_window(scroll),
        "clipBounds": rect_value(visible),
        "clipFrame": rect_value(scroll.contentView().frame()),
        "document": rect_value(document.as_ref().map_or(NSRect::ZERO, |document| document.frame())),
        "documentClass": document.as_ref().map_or(String::new(), |document| class_name(document)),
        "wrapper": wrapper.as_ref().map_or(String::new(), |wrapper| class_name(wrapper)),
        "wrapperFrame": rect_value(wrapper_frame),
        "wrapperHorizontal": width > wrapper_frame.size.width,
        "hasHorizontalScroller": scroll.hasHorizontalScroller(),
        "hasVerticalScroller": scroll.hasVerticalScroller(),
        "scrollerStyle": scroll.scrollerStyle().0,
        "horizontalElasticity": scroll.horizontalScrollElasticity().0,
        "verticalElasticity": scroll.verticalScrollElasticity().0,
        "fits": fits,
        "blocksBack": !fits && blocks(visible, width, 1),
        "blocksForward": !fits && blocks(visible, width, -1),
    })
}
