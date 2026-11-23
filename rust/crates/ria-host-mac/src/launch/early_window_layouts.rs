// Ported from ardia packages/ardiacore/ios/ArdiaEarlyWindowLayouts.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::path::Path;

use serde_json::{Map, Value};

use super::{launch_config::LaunchConfig, saved_document, saved_session::SavedSession};

pub(super) struct Layout {
    pub frame: [f64; 4],
    pub page: [f64; 4],
    pub card: [f64; 4],
    pub radius: f64,
    pub page_color: Option<[f64; 4]>,
    pub picture: Option<Vec<u8>>,
    pub picture_size: Option<[f64; 2]>,
}

pub(super) fn read(config: &LaunchConfig, session: &SavedSession, dark: bool) -> Option<Layout> {
    if !config.switch_on("launchTab") {
        return None;
    }
    if let Some(document) = saved_document::read(config, Path::new("onboarding.json"), 0..=1)
        && document
            .fields
            .get("startedAt")
            .is_some_and(Value::is_number)
        && !document
            .fields
            .get("completedAt")
            .and_then(Value::as_f64)
            .is_some_and(|time| time > 0.0)
    {
        return None;
    }
    let tab = session.launch_tab.as_ref()?;
    let id = session.focused_window_id.as_deref()?;
    let window = session.window_with_id(id)?;
    if window.get("incognito").and_then(Value::as_bool) == Some(true)
        || window.contains_key("kind")
        || !window
            .get("tabIds")?
            .as_array()?
            .iter()
            .any(|id| id.as_str() == Some(&tab.id))
        || session
            .splits
            .iter()
            .any(|split| session.tab_ids_of_split(split).any(|id| id == tab.id))
        || session.launch_backdrop.as_ref()?.get("dark")?.as_bool()? != dark
    {
        return None;
    }
    let document = saved_document::read(config, Path::new("launch-layout.json"), 5..=5)?;
    let layout = document.fields.get("windows")?.get(id)?;
    if serde_json::from_str::<Map<String, Value>>(layout.get("state")?.as_str()?).ok()?
        != state_of(&session.settings, window)
    {
        return None;
    }
    let frame = numbers(window.get("frame")?)?;
    if frame[2] < 300.0 || frame[3] < 200.0 {
        return None;
    }
    Some(Layout {
        frame,
        page: insets(layout.get("page")?)?,
        card: insets(layout.get("card")?)?,
        radius: layout
            .get("radius")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .max(0.0),
        page_color: layout.get("pageColor").and_then(numbers),
        picture: super::launch_picture::read(config, session, window, id, layout),
        picture_size: layout.get("size").and_then(|size| {
            let [width, height] = size.as_array()?.as_slice() else {
                return None;
            };
            Some([width.as_f64()?, height.as_f64()?])
        }),
    })
}

fn state_of(settings: &Map<String, Value>, window: &Map<String, Value>) -> Map<String, Value> {
    let mut state = Map::new();
    for key in [
        "tabLayout",
        "sidebarSide",
        "addressBar",
        "sidebarWidth",
        "bookmarksBar",
        "appearance",
        "hideToolbarWhileScrolling",
        "showFullUrl",
    ] {
        if let Some(value) = settings.get(key) {
            state.insert(format!("s.{key}"), value.clone());
        }
    }
    for key in ["compactToolbar", "borderlessMode", "minimalismMode"] {
        if settings.get(key).and_then(Value::as_bool) == Some(true) {
            state.insert(format!("s.{key}"), Value::Bool(true));
        }
    }
    if let Some(value) = settings
        .get("pageFrame")
        .filter(|value| value.as_f64().is_some_and(|number| number != 9.0))
    {
        state.insert("s.pageFrame".into(), value.clone());
    }
    for key in ["tabLayout", "profileId", "incognito"] {
        if let Some(value) = window.get(key) {
            state.insert(format!("w.{key}"), value.clone());
        }
    }
    if let Some(hide) = window
        .get("autoHideTabs")
        .and_then(Value::as_bool)
        .or_else(|| settings.get("autoHideTabs").and_then(Value::as_bool))
    {
        state.insert("w.sidebarOpen".into(), Value::Bool(!hide));
    } else if let Some(open) = window.get("sidebarOpen") {
        state.insert("w.sidebarOpen".into(), open.clone());
    }
    state
}

fn numbers(value: &Value) -> Option<[f64; 4]> {
    let [a, b, c, d] = value.as_array()?.as_slice() else {
        return None;
    };
    Some([a.as_f64()?, b.as_f64()?, c.as_f64()?, d.as_f64()?])
}

fn insets(value: &Value) -> Option<[f64; 4]> {
    numbers(value).filter(|values| values.iter().all(|n| *n >= 0.0))
}

pub(super) fn page_rect(
    width: f64,
    height: f64,
    [left, top, right, bottom]: [f64; 4],
) -> Option<[f64; 4]> {
    let width = width - left - right;
    let height = height - top - bottom;
    (width >= 100.0 && height >= 100.0).then_some([left, bottom, width, height])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn saved_window_override_changes_the_layout_key() {
        let settings = json!({"autoHideTabs":true,"pageFrame":9,"compactToolbar":false});
        let window = json!({"autoHideTabs":false,"sidebarOpen":false,"profileId":"Default"});
        assert_eq!(
            state_of(settings.as_object().unwrap(), window.as_object().unwrap()),
            *json!({"w.sidebarOpen":true,"w.profileId":"Default"})
                .as_object()
                .unwrap()
        );
    }

    #[test]
    fn asymmetric_insets_use_the_bottom_in_appkit_coordinates() {
        assert_eq!(
            page_rect(200.0, 180.0, [10.0, 30.0, 20.0, 40.0]),
            Some([10.0, 40.0, 170.0, 110.0])
        );
        assert_eq!(page_rect(200.0, 150.0, [10.0, 30.0, 20.0, 40.0]), None);
    }
}
