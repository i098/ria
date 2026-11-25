// Ria original, no Ardia source.

//! The Rust side of check S for `state/splits`: reads the actions of one recorded case from stdin, runs each one on
//! `ria-state`, and writes the steps (state, calls and result) to stdout. See `record-splits.mjs` for the Ardia side.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io;

use ria_state::browser::{SplitView, Tab, TabPin};
use ria_state::kernel::{AppState, Effects};
use ria_state::splits::{self, lanes, splits as lib};
use ria_state::{groups, tabs, windows};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

fn field<T: DeserializeOwned>(a: &Value, key: &str) -> T {
    serde_json::from_value(a[key].clone()).unwrap_or_else(|e| panic!("bad `{key}` in {a}: {e}"))
}

fn seed(app: &mut AppState, a: &Value) {
    app.get_mut::<tabs::State>().tabs = field(a, "tabs");
    app.get_mut::<windows::State>().windows = field(a, "windows");
    app.get_mut::<groups::State>().groups = field(a, "groups");
    app.get_mut::<splits::State>().splits = field(a, "splits");
}

fn project(app: &AppState) -> Value {
    let windows: Map<String, Value> = app
        .get::<windows::State>()
        .windows
        .iter()
        .map(|(id, w)| {
            let w = serde_json::to_value(w).expect("window");
            (
                id.clone(),
                json!({ "tabIds": w["tabIds"], "activeTabIds": w["activeTabIds"] }),
            )
        })
        .collect();
    let groups: HashMap<&String, &Vec<String>> = app
        .get::<groups::State>()
        .groups
        .iter()
        .map(|(id, g)| (id, &g.tab_ids))
        .collect();
    json!({ "splits": app.get::<splits::State>().splits, "windows": windows, "groups": groups })
}

fn pin(a: &Value, key: &str) -> Option<TabPin> {
    field(a, key)
}

fn call(a: &Value) -> Option<Value> {
    let fn_name = a["fn"].as_str().expect("fn");
    let tabs: HashMap<String, Tab> = if a.get("tabs").is_some() {
        field(a, "tabs")
    } else {
        HashMap::new()
    };
    let splits: HashMap<String, SplitView> = if a.get("splits").is_some() {
        field(a, "splits")
    } else {
        HashMap::new()
    };
    let ids = || -> Vec<String> { field(a, "ids") };
    let one = |key: &str| -> SplitView { field(a, key) };
    let v = |x: &dyn erased::Ser| x.to_value();
    Some(match fn_name {
        "removeFromSplits" => {
            let gone: HashSet<String> = field(a, "gone");
            v(&lib::remove_from_splits(&splits, &gone))
        }
        "sanitizeSplits" => v(&lib::sanitize_splits(&splits, &tabs)),
        "splitRowPanes" => {
            let s: Option<SplitView> = field(a, "split");
            v(&lib::split_row_panes(&tabs, s.as_ref()))
        }
        "withSplitRows" => v(&lib::with_split_rows(&tabs, &splits, &ids())),
        "splitRowHead" => v(&lib::split_row_head(
            &tabs,
            &splits,
            &field::<Vec<String>>(a, "order"),
            a["id"].as_str().unwrap(),
        )),
        "paneChunks" => v(&lib::pane_chunks(&ids())),
        "stackAt" => v(&lib::stack_at(&one("split"))),
        "splitTabIds" => v(&lib::split_tab_ids(&one("split"))),
        "isStacked" => v(&lib::is_stacked(
            &one("split"),
            a["tabId"].as_str().unwrap(),
        )),
        "evenSlots" => v(&lib::even_slots(&ids())),
        "withPaneIds" => v(&lib::with_pane_ids(&one("split"), &ids())),
        "splitOf" => v(&lib::split_of(&splits, a["tabId"].as_str())),
        "splitsWhereUnavailable" => {
            match lib::splits_where_unavailable(&splits, a["available"].as_bool().unwrap()) {
                Some(s) => json!({ "splits": s }),
                None => Value::Null,
            }
        }
        "splitLane" => {
            json!(format!("{:?}", lanes::split_lane(pin(a, "pin").as_ref())).to_lowercase())
        }
        "crossesLane" => json!(lanes::crosses_lane(
            pin(a, "anchor").as_ref(),
            pin(a, "tab").as_ref()
        )),
        "splitsOnRow" => json!(lanes::splits_on_row(
            &tabs,
            a["tabId"].as_str().unwrap(),
            a["rowTabId"].as_str().unwrap()
        )),
        other => panic!("unknown call {other}"),
    })
}

/// Lets `call` turn any serializable value into JSON, with `null` for `None` as Ardia's `?? null`.
mod erased {
    pub trait Ser {
        fn to_value(&self) -> serde_json::Value;
    }
    impl<T: serde::Serialize> Ser for T {
        fn to_value(&self) -> serde_json::Value {
            serde_json::to_value(self).expect("serialize")
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let actions: Vec<Value> = serde_json::from_reader(io::stdin().lock())?;
    let mut app = AppState::default();
    let mut steps = vec![];
    for a in actions {
        let mut fx = Effects::default();
        let result = match a["type"].as_str() {
            Some("seed") => {
                seed(&mut app, &a);
                None
            }
            Some("call") => call(&a),
            _ => {
                splits::apply(&mut app, serde_json::from_value(a.clone())?, &mut fx);
                None
            }
        };
        let calls: Vec<Value> = fx
            .calls()
            .iter()
            .map(|c| json!({ "name": c.name(), "args": c.args() }))
            .collect();
        let mut step = json!({ "state": project(&app), "calls": calls });
        if let Some(r) = result {
            step["result"] = r;
        }
        steps.push(step);
    }
    serde_json::to_writer(io::stdout().lock(), &steps)?;
    Ok(())
}
