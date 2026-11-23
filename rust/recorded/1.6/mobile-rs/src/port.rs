// Ria original, no Ardia source.

use ria_core::identity::mobile::{Mobile, MobileSearch, MobileSuggestOptions};
use ria_core::omnibox::suggestion::{
    BookmarkRow, HistoryRow, SourceTab, Suggestion, SuggestionSource,
};
use ria_core::search::engines::CustomSearchEngine;
use ria_core::search::site_search::{ScopeKind, SearchScope};
use serde_json::{Value, json};

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

fn optional(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}

fn rows(v: &Value) -> impl Iterator<Item = &Value> {
    v.as_array().into_iter().flatten()
}

fn scope(v: &Value) -> Option<SearchScope> {
    Some(SearchScope {
        kind: match v["kind"].as_str()? {
            "engine" => ScopeKind::Engine,
            "site" => ScopeKind::Site,
            "history" => ScopeKind::History,
            "provider" => ScopeKind::Provider,
            _ => return None,
        },
        name: text(&v["name"]).into(),
        host: text(&v["host"]).into(),
        url: text(&v["url"]).into(),
        engine_id: optional(&v["engineId"]),
        provider_id: optional(&v["providerId"]),
    })
}

fn source(v: &Value) -> SuggestionSource {
    SuggestionSource {
        tabs: rows(&v["tabs"])
            .map(|r| SourceTab {
                id: text(&r["id"]).into(),
                url: text(&r["url"]).into(),
                title: text(&r["title"]).into(),
                custom_title: optional(&r["customTitle"]),
                favicon: optional(&r["favicon"]),
            })
            .collect(),
        history: rows(&v["history"])
            .map(|r| HistoryRow {
                url: text(&r["url"]).into(),
                title: text(&r["title"]).into(),
                favicon: optional(&r["favicon"]),
                visits: r["visits"].as_f64().unwrap_or_default(),
                last_visit: r["lastVisit"].as_f64().unwrap_or_default(),
            })
            .collect(),
        bookmarks: rows(&v["bookmarks"])
            .map(|r| BookmarkRow {
                url: text(&r["url"]).into(),
                title: text(&r["title"]).into(),
                favicon: optional(&r["favicon"]),
            })
            .collect(),
    }
}

fn suggestion(row: Suggestion) -> Option<Value> {
    Some(match row {
        Suggestion::Page {
            url,
            title,
            favicon,
            tab_id,
            bookmarked,
            visited,
        } => {
            let mut row = json!({"kind":"page", "url":url, "title":title, "favicon":favicon});
            if let Some(id) = tab_id {
                row["tabId"] = id.into();
            }
            if bookmarked {
                row["bookmarked"] = true.into();
            }
            if visited {
                row["visited"] = true.into();
            }
            row
        }
        Suggestion::Search {
            query,
            url,
            engine,
            suggested,
            bang,
        } => {
            let mut row = json!({"kind":"search", "query":query, "url":url, "engine":engine});
            if suggested {
                row["suggested"] = true.into();
            }
            if bang {
                row["bang"] = true.into();
            }
            row
        }
        Suggestion::Calc {
            expression,
            value,
            url,
        } => {
            json!({"kind":"calc", "expression":expression, "value":value, "url":url})
        }
        _ => return None,
    })
}

fn call(mobile: &mut Mobile, step: &Value) -> Result<Value, &'static str> {
    let arg = &step[1];
    let extra = &step[2];
    Ok(match text(&step[0]) {
        "mobileLoadBangDocument" => json!(mobile.load_bang_document(text(arg))),
        "mobileBangDocument" => json!(mobile.make_bang_document(text(arg), extra.as_str())),
        "mobileSetSources" => json!(mobile.set_sources(source(arg))?),
        "mobileSetSearch" => {
            let e = mobile.set_search(&MobileSearch {
                engine_id: optional(&arg["engineId"]),
                custom_url: optional(&arg["customUrl"]),
                custom_engines: rows(&arg["customEngines"])
                    .map(|r| CustomSearchEngine {
                        id: text(&r["id"]).into(),
                        name: text(&r["name"]).into(),
                        keyword: text(&r["keyword"]).into(),
                        url: text(&r["url"]).into(),
                    })
                    .collect(),
            });
            json!({"id":e.id, "name":e.name, "url":e.url, "suggests":e.suggests})
        }
        "mobileEngines" => json!(
            mobile
                .engines()
                .into_iter()
                .map(|e| json!({"id":e.id, "name":e.name, "url":e.url, "custom":e.custom}))
                .collect::<Vec<_>>()
        ),
        "mobileWarmBangs" => json!(mobile.warm_bangs()),
        "mobileSuggest" => {
            let result = mobile.suggest(
                text(arg),
                &MobileSuggestOptions {
                    current_url: optional(&extra["currentUrl"]),
                    current_tab_id: optional(&extra["currentTabId"]),
                    remote: rows(&extra["remote"])
                        .map(text)
                        .map(str::to_owned)
                        .collect(),
                    search_preferred: extra["preference"] == "search",
                    limit: extra["limit"].as_f64(),
                    prevent_inline: extra["preventInline"] == true,
                    scope: scope(&extra["scope"]),
                    now: extra["now"].as_f64(),
                },
            )?;
            let items = result
                .items
                .into_iter()
                .map(suggestion)
                .collect::<Option<Vec<_>>>()
                .ok_or("Unexpected mobile suggestion kind")?;
            let mut out = json!({"items": items, "completion":result.completion});
            if let Some(via) = result.via {
                out["via"] = json!(
                    via.into_iter()
                        .map(|(k, v)| (k, v.as_str()))
                        .collect::<std::collections::BTreeMap<_, _>>()
                );
            }
            out
        }
        "mobileResolve" => json!(mobile.resolve(text(arg), scope(extra).as_ref()).map(
            |r| json!({"kind":r.kind.as_str(), "url":r.url, "query":r.query, "engine":r.engine})
        )),
        "mobileSuggestRequest" => json!(mobile.suggest_request(text(arg), scope(extra).as_ref())),
        "mobileParseRemote" => json!(Mobile::parse_remote(text(arg))),
        "mobileFindScope" => json!(mobile.find_scope(text(arg)).map(|s| {
            let kind = match s.kind {
                ScopeKind::Engine => "engine",
                ScopeKind::Site => "site",
                ScopeKind::History => "history",
                ScopeKind::Provider => "provider",
            };
            let mut row = json!({"kind":kind, "name":s.name, "host":s.host, "url":s.url});
            if let Some(id) = s.engine_id {
                row["engineId"] = id.into();
            }
            if let Some(id) = s.provider_id {
                row["providerId"] = id.into();
            }
            row
        })),
        "mobileClean" => json!(Mobile::clean(text(arg))),
        "mobileDisplay" => {
            let d = Mobile::display(text(arg), text(extra))?;
            json!({"display":d.display, "host":d.host, "trail":d.trail})
        }
        _ => return Err("Unknown mobile export"),
    })
}

pub fn replay_mobile(input: String) -> Result<Vec<Value>, String> {
    let steps: Vec<Value> = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    let mut mobile = Mobile::default();
    Ok(steps
        .iter()
        .map(|step| call(&mut mobile, step).unwrap_or_else(|error| json!({"error":error})))
        .collect())
}
