// Ported from ardia packages/sync/src/extensions.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The adapter that syncs the browser extensions: their on, incognito and pin state, removals,
//! and the permission approvals of each version.
//!
//! Changes from Ardia:
//! - A record and an approval are typed (`ExtensionState`, `ExtensionApproval`). A change, the
//!   published values and a kept approval carry only their own fields, in their own order; Ardia
//!   passes on the JSON object with any other fields and its key order.
//! - `captured` is a map, so an id is in it only when the extract put it there; Ardia's `in`
//!   also finds `Object.prototype` names (`constructor`, `__proto__`), which are not extension ids.
//! - `republish` reads `queued` when `extract` runs: the extraction must not borrow the scope.
//!   `scope.ts` calls `republish` in the same cycle, before the outbox changes.
//! - The host methods `read` and `change` return `Err` where the JS promise rejects.

use super::js::{cmp_utf16, is_js_space};
use super::scope_state::{
    Adapter, ApplyRecords, ExtractCtx, Extraction, LocalBoxFuture, SyncResult,
};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// An extension that the browser has.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalExtension {
    pub id: String,
    pub syncable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incognito: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

/// What the browser reports of its extensions.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LocalExtensions {
    pub ready: bool,
    pub extensions: Vec<LocalExtension>,
    pub pending: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settling: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued: Option<Vec<String>>,
}

/// The synced state of one extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionState {
    pub on: bool,
    pub inc: bool,
    pub pin: bool,
}

/// The permissions that a user approved for one version: the fingerprints of its permission sets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionApproval {
    pub v: String,
    pub fps: Vec<String>,
}

/// The kind of an `ExtensionIntent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntentKind {
    Installed,
    Uninstalled,
}

/// A user's install or uninstall that waits to be published, with its generation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtensionIntent {
    pub kind: IntentKind,
    pub g: f64,
}

/// A change that the adapter asks the browser to make.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ExtensionChange {
    Install {
        id: String,
        record: ExtensionState,
        approval: Option<ExtensionApproval>,
    },
    Reconcile {
        id: String,
        record: ExtensionState,
        approval: Option<ExtensionApproval>,
    },
    Cancel {
        id: String,
    },
}

impl ExtensionChange {
    pub fn id(&self) -> &str {
        match self {
            ExtensionChange::Install { id, .. }
            | ExtensionChange::Reconcile { id, .. }
            | ExtensionChange::Cancel { id } => id,
        }
    }
}

/// Port of the `ExtensionsHost` interface (`packages/sync/src/extensions.ts:27`): the browser.
pub trait ExtensionsHost {
    /// `None` when the browser has no extension calls.
    fn read(&self) -> LocalBoxFuture<'_, SyncResult<Option<LocalExtensions>>>;
    /// `Ok(false)` when the browser refused the change.
    fn change<'a>(&'a self, change: &'a ExtensionChange) -> LocalBoxFuture<'a, SyncResult<bool>>;
    fn intents(&self) -> IndexMap<String, ExtensionIntent>;
    fn settle_intents(&self, published: &IndexMap<String, f64>);
    fn keep_installed(&self, id: &str);
}

pub const EXTENSION_PREFIX: &str = "ext:";
pub const EXTENSION_APPROVAL_PREFIX: &str = "ext:v:";
pub const EXTENSION_RETRY_MS: f64 = 3_600_000.0;
const MAX_FINGERPRINTS: usize = 8;

/// Port of `extensionKey` (`packages/sync/src/extensions.ts:43`).
pub fn extension_key(id: &str) -> String {
    format!("{EXTENSION_PREFIX}{id}")
}

/// Port of `extensionApprovalKey` (`packages/sync/src/extensions.ts:44`).
pub fn extension_approval_key(id: &str) -> String {
    format!("{EXTENSION_APPROVAL_PREFIX}{id}")
}

/// Port of `extensionIdOf` (`packages/sync/src/extensions.ts:45`). `slice` counts UTF-16 units.
pub fn extension_id_of(key: &str) -> &str {
    if let Some(id) = key.strip_prefix(EXTENSION_APPROVAL_PREFIX) {
        return id;
    }
    let mut units = 0;
    for (i, c) in key.char_indices() {
        if units >= EXTENSION_PREFIX.len() {
            return &key[i..];
        }
        units += c.len_utf16();
    }
    ""
}

/// Port of `isStateKey` (`packages/sync/src/extensions.ts:47`).
fn is_state_key(key: &str) -> bool {
    key.starts_with(EXTENSION_PREFIX) && !key.starts_with(EXTENSION_APPROVAL_PREFIX)
}

/// `Number.parseInt(n, 10) || 0`.
fn version_part(n: &str) -> f64 {
    let s = n.trim_start_matches(is_js_space);
    let (neg, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let digits = &s[..s
        .bytes()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(s.len())];
    let value = digits.parse::<f64>().unwrap_or(0.0);
    if neg { -value } else { value }
}

/// Port of `compareVersions` (`packages/sync/src/extensions.ts:49`): -1, 0 or 1.
pub fn compare_versions(a: &str, b: &str) -> i32 {
    let pa: Vec<f64> = a.split('.').map(version_part).collect();
    let pb: Vec<f64> = b.split('.').map(version_part).collect();
    for i in 0..pa.len().max(pb.len()) {
        let d = pa.get(i).copied().unwrap_or(0.0) - pb.get(i).copied().unwrap_or(0.0);
        // `if (d)`: 0 and NaN (`Infinity - Infinity`) go on.
        if d != 0.0 && !d.is_nan() {
            return if d < 0.0 { -1 } else { 1 };
        }
    }
    0
}

/// Port of `isRemoval` (`packages/sync/src/extensions.ts:59`).
pub fn is_removal(v: &Value) -> bool {
    v.get("removed") == Some(&Value::Bool(true))
}

/// The state that `isExtensionState` (`packages/sync/src/extensions.ts:61`) accepts.
fn extension_state(v: &Value) -> Option<ExtensionState> {
    if !v.is_object() || is_removal(v) {
        return None;
    }
    Some(ExtensionState {
        on: v.get("on")?.as_bool()?,
        inc: v.get("inc")?.as_bool()?,
        pin: v.get("pin")?.as_bool()?,
    })
}

/// Port of `isExtensionState` (`packages/sync/src/extensions.ts:61`).
pub fn is_extension_state(v: &Value) -> bool {
    extension_state(v).is_some()
}

/// The approval that `isApproval` (`packages/sync/src/extensions.ts:71`) accepts.
fn approval(v: &Value) -> Option<ExtensionApproval> {
    let v = v.as_object()?;
    let fps = v
        .get("fps")?
        .as_array()?
        .iter()
        .map(|fp| fp.as_str().map(str::to_owned))
        .collect::<Option<_>>()?;
    Some(ExtensionApproval {
        v: v.get("v")?.as_str()?.to_owned(),
        fps,
    })
}

/// Port of `localState` (`packages/sync/src/extensions.ts:78`).
fn local_state(e: &LocalExtension) -> ExtensionState {
    ExtensionState {
        on: e.user_disabled != Some(true),
        inc: e.incognito == Some(true),
        pin: e.pinned == Some(true),
    }
}

/// Port of `grantable` (`packages/sync/src/extensions.ts:82`).
fn grantable(approval: Option<&ExtensionApproval>, e: &LocalExtension) -> bool {
    approval.is_some_and(|a| {
        e.approved != Some(true)
            && Some(&a.v) == e.version.as_ref()
            && e.fingerprint.as_ref().is_some_and(|fp| a.fps.contains(fp))
    })
}

/// Port of `approvalToPublish` (`packages/sync/src/extensions.ts:85`).
fn approval_to_publish(
    e: &LocalExtension,
    known: Option<ExtensionApproval>,
) -> Option<ExtensionApproval> {
    let (Some(true), Some(fp), Some(version)) = (
        e.approved,
        &e.fingerprint,
        e.version.as_deref().filter(|v| !v.is_empty()),
    ) else {
        return known;
    };
    let Some(known) = known.filter(|k| compare_versions(version, &k.v) <= 0) else {
        return Some(ExtensionApproval {
            v: version.to_owned(),
            fps: vec![fp.clone()],
        });
    };
    if compare_versions(version, &known.v) < 0 || known.fps.contains(fp) {
        return Some(known);
    }
    let mut fps = known.fps;
    fps.push(fp.clone());
    fps.sort_by(|a, b| cmp_utf16(a, b));
    let fps = fps.split_off(fps.len().saturating_sub(MAX_FINGERPRINTS));
    Some(ExtensionApproval { v: known.v, fps })
}

/// Port of `newerApproval` (`packages/sync/src/extensions.ts:92`).
fn newer_approval(approval: Option<&ExtensionApproval>, e: &LocalExtension) -> bool {
    approval.is_some_and(|a| compare_versions(&a.v, e.version.as_deref().unwrap_or("0")) > 0)
}

fn has_intent(intents: &IndexMap<String, ExtensionIntent>, id: &str, kind: IntentKind) -> bool {
    intents.get(id).is_some_and(|i| i.kind == kind)
}

/// Port of the adapter of `extensionsAdapter` (`packages/sync/src/extensions.ts:94`).
pub struct ExtensionsAdapter {
    host: Rc<dyn ExtensionsHost>,
    tried: RefCell<HashMap<String, f64>>,
    unsettled: RefCell<HashSet<String>>,
}

/// Port of `extensionsAdapter` (`packages/sync/src/extensions.ts:94`).
pub fn extensions_adapter(host: Rc<dyn ExtensionsHost>) -> ExtensionsAdapter {
    ExtensionsAdapter {
        host,
        tried: RefCell::default(),
        unsettled: RefCell::default(),
    }
}

impl ExtensionsAdapter {
    /// Port of `run` (`packages/sync/src/extensions.ts:97`).
    async fn run(&self, change: ExtensionChange) -> SyncResult<bool> {
        let ok = self.host.change(&change).await?;
        let id = change.id().to_owned();
        if ok {
            self.unsettled.borrow_mut().remove(&id);
        } else {
            self.unsettled.borrow_mut().insert(id);
        }
        Ok(ok)
    }

    /// Port of `reconcileOne` (`packages/sync/src/extensions.ts:104`).
    #[expect(clippy::too_many_arguments, reason = "the arguments of reconcileOne")]
    async fn reconcile_one(
        &self,
        id: &str,
        record: Option<&Value>,
        approval_value: Option<&Value>,
        local: &LocalExtensions,
        intents: &IndexMap<String, ExtensionIntent>,
        joined: bool,
        held: &dyn Fn() -> bool,
    ) -> SyncResult<bool> {
        if held() {
            return Ok(false);
        }
        let e = local.extensions.iter().find(|x| x.id == id);
        let pending = local.pending.iter().any(|p| p == id);
        let queued = local
            .queued
            .as_ref()
            .is_some_and(|q| q.iter().any(|p| p == id));
        let approval = approval_value.and_then(approval);
        if record.is_some_and(is_removal) {
            if has_intent(intents, id, IntentKind::Installed) {
                return Ok(true);
            }
            if !joined && e.is_some_and(|e| e.syncable) {
                self.host.keep_installed(id);
                return Ok(true);
            }
            return if e.is_none() && (pending || queued) {
                self.run(ExtensionChange::Cancel { id: id.to_owned() })
                    .await
            } else {
                Ok(true)
            };
        }
        let Some(record) = record.and_then(extension_state) else {
            return Ok(true);
        };
        if let Some(e) = e {
            if !e.syncable {
                return Ok(true);
            }
            if record != local_state(e)
                || grantable(approval.as_ref(), e)
                || newer_approval(approval.as_ref(), e)
            {
                return self
                    .run(ExtensionChange::Reconcile {
                        id: id.to_owned(),
                        record,
                        approval,
                    })
                    .await;
            }
            return Ok(true);
        }
        if has_intent(intents, id, IntentKind::Uninstalled) {
            return Ok(true);
        }
        self.run(ExtensionChange::Install {
            id: id.to_owned(),
            record,
            approval,
        })
        .await
    }

    /// Port of `reconcile` (`packages/sync/src/extensions.ts:186`): installs what the scope has
    /// and this browser lacks, at most once per `EXTENSION_RETRY_MS`, and grants approvals.
    pub async fn reconcile(
        &self,
        visible: &IndexMap<String, Value>,
        now: f64,
        held: Option<&dyn Fn() -> bool>,
    ) -> SyncResult<()> {
        let held = held.unwrap_or(&|| false);
        let Some(local) = self.host.read().await?.filter(|l| l.ready) else {
            return Ok(());
        };
        let intents = self.host.intents();
        for (key, record) in visible {
            if !is_state_key(key) || !is_extension_state(record) {
                continue;
            }
            let id = extension_id_of(key);
            let approval_value = visible.get(&extension_approval_key(id));
            if let Some(e) = local.extensions.iter().find(|x| x.id == id) {
                if e.syncable && grantable(approval_value.and_then(approval).as_ref(), e) {
                    self.reconcile_one(
                        id,
                        Some(record),
                        approval_value,
                        &local,
                        &intents,
                        true,
                        held,
                    )
                    .await?;
                }
                continue;
            }
            if held() {
                return Ok(());
            }
            let tried = self
                .tried
                .borrow()
                .get(id)
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
            if local.pending.iter().any(|p| p == id) || now - tried < EXTENSION_RETRY_MS {
                continue;
            }
            self.tried.borrow_mut().insert(id.to_owned(), now);
            self.reconcile_one(
                id,
                Some(record),
                approval_value,
                &local,
                &intents,
                true,
                held,
            )
            .await?;
        }
        Ok(())
    }
}

impl Adapter for ExtensionsAdapter {
    fn prefix(&self) -> &str {
        EXTENSION_PREFIX
    }

    fn adopt_remote_on_join(&self) -> bool {
        true
    }

    /// Port of `extract` (`packages/sync/src/extensions.ts:139`).
    fn extract<'a>(
        &'a self,
        ctx: &'a ExtractCtx<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Extraction>>> {
        Box::pin(async move {
            let Some(local) = self.host.read().await?.filter(|l| l.ready) else {
                return Ok(None);
            };
            let intents = self.host.intents();
            let mut values = IndexMap::new();
            let mut captured: IndexMap<String, f64> = IndexMap::new();
            let mut installed = HashSet::new();
            let unsettled = self.unsettled.borrow();
            let settling = |id: &String| {
                local.pending.contains(id)
                    || local.settling.as_ref().is_some_and(|s| s.contains(id))
                    || unsettled.contains(id)
            };
            for e in &local.extensions {
                if !e.syncable || e.version.as_deref().is_none_or(str::is_empty) {
                    continue;
                }
                installed.insert(e.id.as_str());
                if settling(&e.id) {
                    continue;
                }
                let key = extension_key(&e.id);
                let stored = ctx.base(&key);
                let explicit = intents
                    .get(&e.id)
                    .filter(|i| i.kind == IntentKind::Installed);
                if explicit.is_none()
                    && ((ctx.known(&key) && stored.is_none())
                        || stored.as_ref().is_some_and(is_removal))
                {
                    continue;
                }
                values.insert(key, json!(local_state(e)));
                if let Some(intent) = explicit {
                    captured.insert(e.id.clone(), intent.g);
                }
                let approval_key = extension_approval_key(&e.id);
                let stored_approval = ctx.base(&approval_key).as_ref().and_then(approval);
                if let Some(approval) = approval_to_publish(e, stored_approval) {
                    values.insert(approval_key, json!(approval));
                }
            }
            for (id, intent) in &intents {
                if intent.kind != IntentKind::Uninstalled || installed.contains(id.as_str()) {
                    continue;
                }
                values.insert(extension_key(id), json!({ "removed": true }));
                captured.insert(id.clone(), intent.g);
            }
            let keys: HashSet<String> = values.keys().cloned().collect();
            let republished: HashSet<String> = captured
                .keys()
                .map(|id| extension_key(id))
                .filter(|key| !ctx.queued(key))
                .collect();
            let host = Rc::clone(&self.host);
            Ok(Some(Extraction {
                values,
                ignore: Some(Box::new(move |key| !keys.contains(key))),
                republish: Some(Box::new(move |key| {
                    is_state_key(key) && republished.contains(key)
                })),
                published: Some(Box::new(move |keys: &IndexSet<String>| {
                    let settled: IndexMap<String, f64> = keys
                        .iter()
                        .filter(|key| is_state_key(key))
                        .map(|key| extension_id_of(key))
                        .filter_map(|id| captured.get(id).map(|g| (id.to_owned(), *g)))
                        .collect();
                    if !settled.is_empty() {
                        host.settle_intents(&settled);
                    }
                })),
                json: None,
            }))
        })
    }

    /// Port of `apply` (`packages/sync/src/extensions.ts:176`).
    fn apply<'a>(
        &'a self,
        visible: &'a IndexMap<String, Value>,
        changed: &'a IndexSet<String>,
        records: &'a ApplyRecords<'a>,
    ) -> LocalBoxFuture<'a, SyncResult<bool>> {
        Box::pin(async move {
            let Some(local) = self.host.read().await?.filter(|l| l.ready) else {
                return Ok(false);
            };
            let intents = self.host.intents();
            let mut ok = true;
            let ids: IndexSet<&str> = changed.iter().map(|key| extension_id_of(key)).collect();
            for id in ids {
                let record = visible.get(&extension_key(id));
                let approval_value = visible.get(&extension_approval_key(id));
                ok = self
                    .reconcile_one(
                        id,
                        record,
                        approval_value,
                        &local,
                        &intents,
                        records.joined,
                        records.held,
                    )
                    .await?
                    && ok;
            }
            Ok(ok && !(records.held)())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_parse_int() {
        assert_eq!(compare_versions("1.10", "1.9"), 1);
        assert_eq!(compare_versions(" 2x", "2"), 0);
        assert_eq!(compare_versions("1.0.0", "1"), 0);
        assert_eq!(compare_versions("-1", "0"), -1);
        assert_eq!(compare_versions("a.1", "0.1"), 0);
        let huge = "9".repeat(400);
        assert_eq!(
            compare_versions(&huge, &huge),
            0,
            "Infinity - Infinity is NaN"
        );
    }

    #[test]
    fn approval_keeps_the_last_fingerprints() {
        let e = LocalExtension {
            id: "x".into(),
            syncable: true,
            version: Some("1".into()),
            approved: Some(true),
            fingerprint: Some("z".into()),
            ..LocalExtension::default()
        };
        let known = ExtensionApproval {
            v: "1".into(),
            fps: (0..8).map(|i| format!("a{i}")).collect(),
        };
        let fps = approval_to_publish(&e, Some(known)).map(|a| a.fps);
        assert_eq!(
            fps.as_ref()
                .map(|f| (f.len(), f[7].as_str(), f[0].as_str())),
            Some((8, "z", "a1"))
        );
    }

    #[test]
    fn ids_slice_like_js() {
        assert_eq!(extension_id_of("ext:v:a"), "a");
        assert_eq!(extension_id_of("ext:a"), "a");
        assert_eq!(extension_id_of("ex"), "");
    }
}
