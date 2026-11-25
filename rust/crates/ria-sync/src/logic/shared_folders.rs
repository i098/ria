// Ported from ardia packages/sync/src/sharedFolders.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The contract of the native shared-folders module: list a shared folder, apply file operations
//! to it, and watch it. Slice 5.13 implements the module.
//!
//! Changes from Ardia:
//! - `SharedFoldersNative` is a trait, not the value of
//!   `requireOptionalNativeModule("ArdiaSharedFolders")`.
//! - `addListener("onChange", listener)` is `on_change(listener)`: `onChange` is its only event.
//!   The `{ remove() }` that it returns is a `Subscription`.
//! - The string unions are enums, and a rejected promise is an `Err`.

use super::scope_state::{LocalBoxFuture, SyncResult};
use serde::{Deserialize, Serialize};

/// Why a shared file is not readable yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingReason {
    Download,
    Unreadable,
}

/// One entry of a listed folder level.
#[derive(Clone, Debug, PartialEq)]
pub enum SharedListingEntry {
    Link {
        file: String,
        text: String,
    },
    Pending {
        file: String,
        reason: PendingReason,
    },
    Dir {
        file: String,
        level: SharedListingLevel,
    },
}

/// One level of a listed shared folder.
#[derive(Clone, Debug, PartialEq)]
pub struct SharedListingLevel {
    pub entries: Vec<SharedListingEntry>,
    pub manifest: Option<String>,
    pub manifest_pending: bool,
    pub error: Option<String>,
}

/// What `read` finds at a shared folder path.
#[derive(Clone, Debug, PartialEq)]
pub enum SharedListing {
    Missing { error: Option<String> },
    Exists { root: SharedListingLevel },
}

/// One file operation that `apply` runs; `apply` takes a JSON array of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum SharedFolderOp {
    Write {
        path: String,
        text: String,
        create: bool,
        expect: Option<String>,
    },
    Move {
        path: String,
        to: String,
    },
    Delete {
        path: String,
        expect: String,
    },
    Mkdir {
        path: String,
    },
    Rmdir {
        path: String,
    },
}

/// Where shared folders live by default.
#[derive(Clone, Debug, PartialEq)]
pub struct SharedLocation {
    pub path: String,
    pub i_cloud_available: bool,
    pub i_cloud_path: String,
}

/// The device and inode of a folder: it stays the same when the folder moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SharedFolderIdentity {
    pub device: f64,
    pub inode: f64,
}

/// A folder with its identity.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedSharedFolder {
    pub identity: SharedFolderIdentity,
    pub path: String,
}

/// The result of `apply`: the paths whose `expect` did not match.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApplyResult {
    pub conflicts: Vec<String>,
}

/// The error code of a refused shared-folder operation.
pub const SHARED_FOLDER_REFUSED: &str = "ERR_SHARED_FOLDER_REFUSED";

/// The listener of `onChange`: it gets the paths that changed.
pub type ChangeListener = Box<dyn Fn(&[String])>;

/// A listener registration; `remove` stops it.
pub trait Subscription {
    fn remove(&self);
}

/// Port of the `SharedFoldersNative` type (`packages/sync/src/sharedFolders.ts:27`).
pub trait SharedFoldersNative {
    fn location(&self) -> SharedLocation;
    fn read<'a>(
        &'a self,
        path: &'a str,
        identity: Option<&'a SharedFolderIdentity>,
    ) -> LocalBoxFuture<'a, SyncResult<SharedListing>>;
    fn apply<'a>(
        &'a self,
        path: &'a str,
        ops: &'a str,
        identity: Option<&'a SharedFolderIdentity>,
    ) -> LocalBoxFuture<'a, SyncResult<ApplyResult>>;
    fn create<'a>(
        &'a self,
        parent: &'a str,
        parent_identity: Option<&'a SharedFolderIdentity>,
        name: &'a str,
    ) -> LocalBoxFuture<'a, SyncResult<ResolvedSharedFolder>>;
    fn resolve<'a>(&'a self, path: &'a str)
    -> LocalBoxFuture<'a, SyncResult<ResolvedSharedFolder>>;
    fn discard<'a>(
        &'a self,
        path: &'a str,
        identity: &'a SharedFolderIdentity,
    ) -> LocalBoxFuture<'a, SyncResult<bool>>;
    fn watch(&self, paths: &[String]);
    /// `addListener("onChange", listener)`.
    fn on_change(&self, listener: ChangeListener) -> Box<dyn Subscription>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_serialize_as_the_native_module_reads_them() {
        let ops = [
            SharedFolderOp::Write {
                path: "a".into(),
                text: "t".into(),
                create: true,
                expect: None,
            },
            SharedFolderOp::Move {
                path: "a".into(),
                to: "b".into(),
            },
        ];
        assert_eq!(
            serde_json::to_string(&ops).ok().as_deref(),
            Some(
                r#"[{"op":"write","path":"a","text":"t","create":true,"expect":null},{"op":"move","path":"a","to":"b"}]"#
            )
        );
    }
}
