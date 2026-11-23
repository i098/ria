// Ported from ardia packages/ardiacore/ios/ArdiaWebView+SiteData.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use objc2::rc::Weak;
use objc2_foundation::{NSDictionary, NSString, NSURL};
use ria_engine::call::main_queue;
use serde_json::{Map, Value, json};

use crate::core::bindings::{Profile, Tab};
use crate::core::convert::{from_json, to_json};

use super::ardia_web_view_page_commands::awaits_first_document;

const MAX_STORAGE_BYTES: usize = 2_000_000;
const MAX_STORAGE_WRITES: usize = 4_000;
const MAX_COOKIE_WRITES: usize = 2_000;

type Reply = Finish<Option<Value>>;
type Finish<T> = Box<dyn FnOnce(T)>;

pub fn site_data(tab: Option<&Tab>, profile: Option<&Profile>, request: &Value, completion: Reply) {
    let Some(tab) = tab.filter(|tab| {
        !tab.closed()
            && !awaits_first_document(tab)
            && profile.is_some_and(|profile| !profile.off_the_record())
            && tab
                .profile()
                .is_some_and(|profile| !profile.off_the_record())
    }) else {
        return completion(None);
    };
    match request["op"].as_str() {
        Some("read") => read_site_data(tab, request, completion),
        Some("write") => write_site_data(tab, request, completion),
        _ => completion(None),
    }
}

fn call(
    tab: &Tab,
    method: &str,
    params: Option<&Value>,
    done: impl FnOnce(Option<Value>) + 'static,
) {
    if tab.closed() {
        return done(None);
    }
    let params = params.and_then(|params| from_json(params).downcast::<NSDictionary>().ok());
    let done = Cell::new(Some(done));
    tab.dev_tools_call(
        &NSString::from_str(method),
        params.as_deref(),
        1.0,
        move |result, _| {
            if let Some(done) = done.take() {
                let result = result.map(|value| to_json(&value));
                main_queue::post(Box::new(move || done(result)));
            }
        },
    );
}

struct Batch<T> {
    pending: Cell<usize>,
    result: RefCell<T>,
    finish: Cell<Option<Finish<T>>>,
}

impl<T: Default + 'static> Batch<T> {
    fn new(finish: impl FnOnce(T) + 'static) -> Rc<Self> {
        Rc::new(Self {
            pending: Cell::new(1),
            result: RefCell::default(),
            finish: Cell::new(Some(Box::new(finish))),
        })
    }

    fn add(
        self: &Rc<Self>,
        apply: impl FnOnce(&mut T, Option<Value>) + 'static,
    ) -> impl FnOnce(Option<Value>) + 'static {
        self.pending.set(self.pending.get() + 1);
        let batch = Rc::clone(self);
        move |value| {
            apply(&mut batch.result.borrow_mut(), value);
            batch.complete();
        }
    }

    fn complete(&self) {
        let pending = self.pending.get() - 1;
        self.pending.set(pending);
        if pending == 0
            && let Some(finish) = self.finish.take()
        {
            let result = self.result.take();
            main_queue::post(Box::new(move || finish(result)));
        }
    }
}

fn array(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn strings(value: &Value, limit: usize) -> impl Iterator<Item = String> + '_ {
    array(value)
        .iter()
        .filter_map(Value::as_str)
        .filter(|s| !s.is_empty())
        .take(limit)
        .map(str::to_lowercase)
}

fn web_origin(text: &str) -> Option<String> {
    let url = NSURL::URLWithString(&NSString::from_str(text))?;
    let scheme = url.scheme()?.lowercaseString().to_string();
    let host = url.host()?.lowercaseString().to_string();
    if host.is_empty() || !matches!(scheme.as_str(), "http" | "https") {
        return None;
    }
    Some(match url.port() {
        Some(port) => format!("{scheme}://{host}:{port}"),
        None => format!("{scheme}://{host}"),
    })
}

fn storage_id(origin: &str) -> Value {
    json!({"storageKey": format!("{origin}/"), "isLocalStorage": true})
}

fn covered(domain: &str, domains: &[String]) -> bool {
    let bare = domain.trim_matches('.').to_lowercase();
    !bare.is_empty()
        && domains.iter().any(|wanted| {
            bare.strip_suffix(wanted)
                .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with('.'))
        })
}

fn truthy_number(value: &Value) -> bool {
    value
        .as_bool()
        .unwrap_or_else(|| value.as_f64().is_some_and(|number| number != 0.0))
}

fn filter_cookies(result: Value, domains: &[String]) -> Option<Vec<Value>> {
    let Value::Object(mut result) = result else {
        return None;
    };
    let Value::Array(mut cookies) = result.remove("cookies")? else {
        return None;
    };
    cookies.retain(|cookie| {
        cookie.get("partitionKey").is_none()
            && !truthy_number(&cookie["partitionKeyOpaque"])
            && cookie["domain"]
                .as_str()
                .is_some_and(|domain| covered(domain, domains))
    });
    Some(cookies)
}

fn pair(value: &Value) -> Option<(&str, &str)> {
    let [key, value] = array(value) else {
        return None;
    };
    Some((key.as_str()?, value.as_str()?))
}

fn storage_items(result: Value) -> Option<Vec<Value>> {
    let Value::Object(mut result) = result else {
        return None;
    };
    let Value::Array(mut entries) = result.remove("entries")? else {
        return None;
    };
    let mut units = 0;
    entries.retain(|entry| {
        let Some((key, value)) = pair(entry) else {
            return false;
        };
        units += key.encode_utf16().count() + value.encode_utf16().count();
        true
    });
    (units <= MAX_STORAGE_BYTES).then_some(entries)
}

fn read_site_data(tab: &Tab, request: &Value, completion: Reply) {
    let batch = Batch::new(
        move |(cookies, storage): (Option<Vec<Value>>, Map<String, Value>)| {
            completion(cookies.map(|cookies| json!({"cookies": cookies, "storage": storage})));
        },
    );
    let domains: Vec<_> = strings(&request["domains"], 64).collect();
    let cookies_done = batch.add(move |state, value| {
        state.0 = value.and_then(|value| filter_cookies(value, &domains));
    });
    let weak_tab = Weak::from(tab);
    call(tab, "Network.getAllCookies", None, move |result| {
        if result.is_none()
            && let Some(tab) = weak_tab.load()
        {
            call(&tab, "Storage.getCookies", None, cookies_done);
        } else {
            cookies_done(result);
        }
    });
    let mut origins = HashSet::new();
    for origin in strings(&request["origins"], 16).filter_map(|origin| web_origin(&origin)) {
        if !origins.insert(origin.clone()) {
            continue;
        }
        let params = json!({"storageId": storage_id(&origin)});
        call(
            tab,
            "DOMStorage.getDOMStorageItems",
            Some(&params),
            batch.add(move |state, value| {
                state.1.insert(
                    origin,
                    value
                        .and_then(storage_items)
                        .map_or(Value::Null, Value::Array),
                );
            }),
        );
    }
    batch.complete();
}

fn cookie_write(cookie: &Value) -> Option<Value> {
    let mut params = Map::new();
    for key in ["name", "value", "url", "domain", "path", "sameSite"] {
        if cookie[key].is_string() {
            params.insert(key.into(), cookie[key].clone());
        }
    }
    for key in ["secure", "httpOnly"] {
        if cookie[key].is_boolean() || cookie[key].is_number() {
            params.insert(key.into(), cookie[key].clone());
        }
    }
    if cookie["expires"]
        .as_f64()
        .is_some_and(|number| number > 0.0)
    {
        params.insert("expires".into(), cookie["expires"].clone());
    }
    (!cookie["name"].as_str().unwrap_or_default().is_empty()
        && params.contains_key("value")
        && (params.contains_key("url") || params.contains_key("domain")))
    .then_some(Value::Object(params))
}

#[derive(Default)]
struct WriteCounts {
    cookies: usize,
    removed: usize,
    storage: usize,
}

fn write_site_data(tab: &Tab, request: &Value, completion: Reply) {
    let batch = Batch::new(move |count: WriteCounts| {
        completion(Some(
            json!({"cookiesWritten": count.cookies, "removed": count.removed, "storageWritten": count.storage}),
        ));
    });
    for params in array(&request["cookies"])
        .iter()
        .filter_map(cookie_write)
        .take(MAX_COOKIE_WRITES)
    {
        call(
            tab,
            "Network.setCookie",
            Some(&params),
            batch.add(|count, result| {
                count.cookies +=
                    usize::from(result.is_some_and(|value| truthy_number(&value["success"])));
            }),
        );
    }
    for raw in array(&request["remove"]) {
        let (Some(name), Some(domain)) = (raw["name"].as_str(), raw["domain"].as_str()) else {
            continue;
        };
        let mut params = json!({"name": name, "domain": domain});
        if let Some(path) = raw["path"].as_str() {
            params["path"] = Value::from(path);
        }
        call(
            tab,
            "Network.deleteCookies",
            Some(&params),
            batch.add(|count, result| {
                count.removed += usize::from(result.is_some());
            }),
        );
    }
    if let Some(storage) = request["storage"].as_object() {
        write_storage(tab, storage, &batch);
    }
    batch.complete();
}

fn write_storage(tab: &Tab, storage: &Map<String, Value>, batch: &Rc<Batch<WriteCounts>>) {
    let mut remaining = MAX_STORAGE_WRITES;
    for (origin, change) in storage {
        let Some(origin) = web_origin(origin).filter(|_| change.is_object()) else {
            continue;
        };
        let id = storage_id(&origin);
        let set = array(&change["set"])
            .iter()
            .filter_map(pair)
            .map(|(key, value)| (key, Some(value)));
        let remove = array(&change["remove"])
            .iter()
            .filter_map(Value::as_str)
            .map(|key| (key, None));
        for (key, value) in set.chain(remove).take(remaining) {
            remaining -= 1;
            let mut params = json!({"storageId": id, "key": key});
            let method = if let Some(value) = value {
                params["value"] = Value::from(value);
                "DOMStorage.setDOMStorageItem"
            } else {
                "DOMStorage.removeDOMStorageItem"
            };
            call(
                tab,
                method,
                Some(&params),
                batch.add(|count, result| {
                    count.storage += usize::from(result.is_some());
                }),
            );
        }
        if remaining == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_read_excludes_partitioned_and_lookalike_domains() {
        let cookies = json!({"cookies": [
            {"domain": ".a.EXAMPLE.org", "name": "keep"},
            {"domain": "notexample.org", "name": "lookalike"},
            {"domain": "example.org", "partitionKey": null},
            {"domain": "example.org", "partitionKeyOpaque": true}
        ]});
        assert_eq!(
            filter_cookies(cookies, &["example.org".into()]).unwrap(),
            [json!({"domain": ".a.EXAMPLE.org", "name": "keep"})]
        );
    }

    #[test]
    fn cookie_write_drops_non_cdp_fields_and_session_expiry() {
        let cookie = json!({"name":"a", "value":"", "domain":"example.org", "expires":-1, "partitionKey":"other", "secure":true});
        assert_eq!(
            cookie_write(&cookie),
            Some(json!({"name":"a", "value":"", "domain":"example.org", "secure":true}))
        );
        assert!(cookie_write(&json!({"name":"a", "domain":"example.org"})).is_none());
    }

    #[test]
    fn storage_read_rejects_the_whole_origin_above_the_limit() {
        let large = "😀".repeat(MAX_STORAGE_BYTES / 2);
        assert!(storage_items(json!({"entries":[["k", large]]})).is_none());
        assert_eq!(
            storage_items(json!({"entries":[["k", "v"], ["malformed"]]})),
            Some(vec![json!(["k", "v"])])
        );
    }
}
