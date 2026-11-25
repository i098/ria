// Ported from ardia packages/sync/src/native.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The contract of the native sync module (folder, keychain, recovery kit, passwords) and the
//! transport that writes a scope's files through it. Slice 5.13 implements the module.
//!
//! Changes from Ardia:
//! - `SyncNative` is a trait, not the value of `requireOptionalNativeModule("ArdiaSync")`; the
//!   app passes an implementation, or none where Ardia's value is `null`.
//! - The string unions are enums (`PhraseResult`, `ChainStatus`, `RecoveryKitFormat`,
//!   `SealedSecret`), and a rejected promise is an `Err`.
//! - The optional `takeFileCounts` is a method whose default returns `None`.

use super::scope_state::{Listing, LocalBoxFuture, SyncResult, Transport};

/// What the native module knows of a sync folder.
#[derive(Clone, Debug, PartialEq)]
pub struct FolderInfo {
    pub path: String,
    pub default_path: String,
    pub i_cloud_available: bool,
    pub in_i_cloud: bool,
    pub exists: bool,
    pub writable: bool,
    pub has_sync_data: bool,
}

/// The result of `enterPhrase`.
#[derive(Clone, Debug, PartialEq)]
pub enum PhraseResult {
    Ok,
    WordCount { count: f64 },
    UnknownWord { word: String },
    Checksum,
    NoData,
    Mismatch,
    Keychain,
}

/// The secret of a `SealedLogin`: a sealed password, or none for a passkey-only login.
#[derive(Clone, Debug, PartialEq)]
pub enum SealedSecret {
    Sealed(String),
    Passwordless,
}

/// A saved login with its password sealed by the native module.
#[derive(Clone, Debug, PartialEq)]
pub struct SealedLogin {
    pub origin: String,
    pub username: String,
    pub created: f64,
    pub secret: SealedSecret,
}

/// The result of `chainStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainStatus {
    Ok,
    Missing,
    Unavailable,
}

/// The `format` of `saveRecoveryKit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryKitFormat {
    Pdf,
    Text,
}

/// The result of `takeFileCounts`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FileCounts {
    pub placeholders: f64,
    pub busy: f64,
    pub hydrated: f64,
}

/// Port of the `SyncNative` type (`packages/sync/src/native.ts:25`).
pub trait SyncNative {
    fn folder_info<'a>(
        &'a self,
        path: Option<&'a str>,
    ) -> LocalBoxFuture<'a, SyncResult<FolderInfo>>;
    fn choose_folder<'a>(
        &'a self,
        current: Option<&'a str>,
    ) -> LocalBoxFuture<'a, SyncResult<Option<String>>>;
    fn new_device_id(&self) -> String;
    fn device_name(&self) -> String;
    fn create_phrase<'a>(&'a self, account: &'a str) -> LocalBoxFuture<'a, SyncResult<bool>>;
    fn enter_phrase<'a>(
        &'a self,
        account: &'a str,
        text: &'a str,
        folder: &'a str,
    ) -> LocalBoxFuture<'a, SyncResult<PhraseResult>>;
    fn unlock<'a>(&'a self, account: &'a str) -> LocalBoxFuture<'a, SyncResult<bool>>;
    fn forget<'a>(&'a self, account: &'a str) -> LocalBoxFuture<'a, SyncResult<()>>;
    fn chain_status<'a>(&'a self, folder: &'a str) -> LocalBoxFuture<'a, SyncResult<ChainStatus>>;
    fn create_chain<'a>(&'a self, folder: &'a str) -> LocalBoxFuture<'a, SyncResult<()>>;
    fn write<'a>(
        &'a self,
        folder: &'a str,
        scope: &'a str,
        payload: &'a str,
    ) -> LocalBoxFuture<'a, SyncResult<String>>;
    fn read<'a>(
        &'a self,
        folder: &'a str,
        scope: &'a str,
        known: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<Listing>>;
    fn remove<'a>(
        &'a self,
        folder: &'a str,
        scope: &'a str,
        ids: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<()>>;
    fn delete_chain<'a>(&'a self, folder: &'a str) -> LocalBoxFuture<'a, SyncResult<()>>;
    fn seal_local<'a>(&'a self, text: &'a str) -> LocalBoxFuture<'a, SyncResult<String>>;
    fn open_local<'a>(&'a self, sealed: &'a str) -> LocalBoxFuture<'a, SyncResult<Option<String>>>;
    fn recovery_words(&self) -> LocalBoxFuture<'_, SyncResult<Option<Vec<String>>>>;
    fn qr_code(&self) -> LocalBoxFuture<'_, SyncResult<Option<String>>>;
    fn recovery_kit_preview(&self, width: f64) -> LocalBoxFuture<'_, SyncResult<Option<String>>>;
    fn save_recovery_kit(
        &self,
        format: RecoveryKitFormat,
    ) -> LocalBoxFuture<'_, SyncResult<Option<String>>>;
    fn copy_recovery_kit(&self) -> LocalBoxFuture<'_, SyncResult<Option<bool>>>;
    fn share_recovery_kit(&self) -> LocalBoxFuture<'_, SyncResult<Option<bool>>>;
    fn copy_recovery_phrase(&self) -> LocalBoxFuture<'_, SyncResult<Option<bool>>>;
    fn reveal_folder<'a>(&'a self, path: &'a str) -> LocalBoxFuture<'a, SyncResult<()>>;
    fn read_passwords<'a>(
        &'a self,
        engine_profile: &'a str,
    ) -> LocalBoxFuture<'a, SyncResult<Option<Vec<SealedLogin>>>>;
    fn save_password<'a>(
        &'a self,
        engine_profile: &'a str,
        origin: &'a str,
        username: &'a str,
        sealed: &'a str,
        replacing: bool,
    ) -> LocalBoxFuture<'a, SyncResult<bool>>;
    fn take_file_counts(&self) -> Option<FileCounts> {
        None
    }
}

/// The transport of `folderTransport`: the scope files of one folder.
pub struct FolderTransport<'n> {
    native: &'n dyn SyncNative,
    folder: String,
}

/// Port of `folderTransport` (`packages/sync/src/native.ts:57`).
pub fn folder_transport<'n>(native: &'n dyn SyncNative, folder: &str) -> FolderTransport<'n> {
    FolderTransport {
        native,
        folder: folder.to_owned(),
    }
}

impl Transport for FolderTransport<'_> {
    fn write<'a>(
        &'a self,
        scope: &'a str,
        payload: &'a str,
    ) -> LocalBoxFuture<'a, SyncResult<String>> {
        self.native.write(&self.folder, scope, payload)
    }

    fn read<'a>(
        &'a self,
        scope: &'a str,
        known: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<Listing>> {
        self.native.read(&self.folder, scope, known)
    }

    fn remove<'a>(
        &'a self,
        scope: &'a str,
        ids: &'a [String],
    ) -> LocalBoxFuture<'a, SyncResult<()>> {
        self.native.remove(&self.folder, scope, ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A native module that logs the folder calls and fails every other call.
    #[derive(Default)]
    struct Logged(RefCell<Vec<String>>);

    fn refused<'a, T: 'a>() -> LocalBoxFuture<'a, SyncResult<T>> {
        Box::pin(async { Err("not in this test".to_owned()) })
    }

    impl SyncNative for Logged {
        fn folder_info<'a>(
            &'a self,
            _: Option<&'a str>,
        ) -> LocalBoxFuture<'a, SyncResult<FolderInfo>> {
            refused()
        }
        fn choose_folder<'a>(
            &'a self,
            _: Option<&'a str>,
        ) -> LocalBoxFuture<'a, SyncResult<Option<String>>> {
            refused()
        }
        fn new_device_id(&self) -> String {
            String::new()
        }
        fn device_name(&self) -> String {
            String::new()
        }
        fn create_phrase<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<bool>> {
            refused()
        }
        fn enter_phrase<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
            _: &'a str,
        ) -> LocalBoxFuture<'a, SyncResult<PhraseResult>> {
            refused()
        }
        fn unlock<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<bool>> {
            refused()
        }
        fn forget<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<()>> {
            refused()
        }
        fn chain_status<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<ChainStatus>> {
            refused()
        }
        fn create_chain<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<()>> {
            refused()
        }
        fn write<'a>(
            &'a self,
            folder: &'a str,
            scope: &'a str,
            payload: &'a str,
        ) -> LocalBoxFuture<'a, SyncResult<String>> {
            self.0
                .borrow_mut()
                .push(format!("write {folder} {scope} {payload}"));
            Box::pin(async { Ok("f1".to_owned()) })
        }
        fn read<'a>(
            &'a self,
            folder: &'a str,
            scope: &'a str,
            known: &'a [String],
        ) -> LocalBoxFuture<'a, SyncResult<Listing>> {
            self.0
                .borrow_mut()
                .push(format!("read {folder} {scope} {}", known.join(",")));
            Box::pin(async {
                Ok(Listing {
                    present: vec!["f1".to_owned()],
                    ..Listing::default()
                })
            })
        }
        fn remove<'a>(
            &'a self,
            folder: &'a str,
            scope: &'a str,
            ids: &'a [String],
        ) -> LocalBoxFuture<'a, SyncResult<()>> {
            self.0
                .borrow_mut()
                .push(format!("remove {folder} {scope} {}", ids.join(",")));
            Box::pin(async { Err("gone".to_owned()) })
        }
        fn delete_chain<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<()>> {
            refused()
        }
        fn seal_local<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<String>> {
            refused()
        }
        fn open_local<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<Option<String>>> {
            refused()
        }
        fn recovery_words(&self) -> LocalBoxFuture<'_, SyncResult<Option<Vec<String>>>> {
            refused()
        }
        fn qr_code(&self) -> LocalBoxFuture<'_, SyncResult<Option<String>>> {
            refused()
        }
        fn recovery_kit_preview(&self, _: f64) -> LocalBoxFuture<'_, SyncResult<Option<String>>> {
            refused()
        }
        fn save_recovery_kit(
            &self,
            _: RecoveryKitFormat,
        ) -> LocalBoxFuture<'_, SyncResult<Option<String>>> {
            refused()
        }
        fn copy_recovery_kit(&self) -> LocalBoxFuture<'_, SyncResult<Option<bool>>> {
            refused()
        }
        fn share_recovery_kit(&self) -> LocalBoxFuture<'_, SyncResult<Option<bool>>> {
            refused()
        }
        fn copy_recovery_phrase(&self) -> LocalBoxFuture<'_, SyncResult<Option<bool>>> {
            refused()
        }
        fn reveal_folder<'a>(&'a self, _: &'a str) -> LocalBoxFuture<'a, SyncResult<()>> {
            refused()
        }
        fn read_passwords<'a>(
            &'a self,
            _: &'a str,
        ) -> LocalBoxFuture<'a, SyncResult<Option<Vec<SealedLogin>>>> {
            refused()
        }
        fn save_password<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
            _: &'a str,
            _: &'a str,
            _: bool,
        ) -> LocalBoxFuture<'a, SyncResult<bool>> {
            refused()
        }
    }

    #[test]
    fn folder_transport_forwards_to_the_folder() {
        let native = Logged::default();
        let transport = folder_transport(&native, "/sync");
        let known = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(
            pollster::block_on(transport.write("p", "{}")),
            Ok("f1".to_owned())
        );
        let listing = pollster::block_on(transport.read("p", &known));
        assert_eq!(listing.map(|l| l.present), Ok(vec!["f1".to_owned()]));
        assert_eq!(
            pollster::block_on(transport.remove("p", &known[..1])),
            Err("gone".to_owned())
        );
        assert_eq!(
            *native.0.borrow(),
            ["write /sync p {}", "read /sync p a,b", "remove /sync p a"]
        );
        assert_eq!(native.take_file_counts(), None);
    }
}
