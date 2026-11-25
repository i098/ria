//! An abort that lands after `job` checked `aborted` but before it registered its abort listener
//! (review-1 finding) must still cancel the import. The fake native aborts the signal inside
//! `add_listener`, which `job` calls between those two points.

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ria_import::logic::*;

type E = Box<dyn Error + Send + Sync>;

#[derive(Default)]
struct Fake {
    signal: Mutex<Option<Arc<AbortSignal>>>,
    import_calls: Arc<AtomicUsize>,
    removed: Arc<AtomicUsize>,
}

struct Sub(Arc<AtomicUsize>);
impl Subscription for Sub {
    fn remove(self: Box<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl ImportNative for Fake {
    fn add_listener(&self, _l: EventListener) -> Box<dyn Subscription> {
        if let Some(s) = self.signal.lock().unwrap().as_ref() {
            s.abort();
        }
        Box::new(Sub(Arc::clone(&self.removed)))
    }
    fn import_data(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &[ImportKind],
        _: &NativeImportOptions,
    ) -> NativeText {
        self.import_calls.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }
    fn list_browsers(&self) -> NativeText {
        Ok(None)
    }
    fn cancel_import(&self, _: &str) {}
    fn unlock_browser(&self, _: &str, _: Option<&str>) -> NativeText {
        Ok(None)
    }
    fn is_browser_unlocked(&self, _: &str) -> bool {
        false
    }
    fn forget_unlocked_keys(&self) {}
    fn write_imported(&self, _: &str, _: &str, _: &[WriteKind]) -> NativeText {
        Ok(None)
    }
    fn discard_imported(&self, _: &str) {}
    fn list_extensions(&self, _: &str, _: &str) -> NativeText {
        Ok(None)
    }
    fn import_safari_export(&self, _: &str, _: &str) -> NativeText {
        Ok(None)
    }
    fn safari_has_full_disk_access(&self) -> bool {
        false
    }
    fn open_full_disk_access_settings(&self) -> NativeText {
        Ok(None)
    }
    fn import_safari_direct(&self, _: &str) -> NativeText {
        Ok(None)
    }
    fn import_bookmarks_html(&self, _: &str) -> NativeText {
        Ok(None)
    }
    fn import_passwords_csv(&self, _: &str) -> NativeText {
        Ok(None)
    }
    fn choose_import_file(&self, _: ImportFileKind) -> Result<Option<String>, E> {
        Ok(None)
    }
    fn read_dia_tabs(&self) -> NativeText {
        Ok(None)
    }
}

#[test]
fn abort_between_check_and_registration_cancels_without_importing() {
    let signal = Arc::new(AbortSignal::default());
    let fake = Fake::default();
    *fake.signal.lock().unwrap() = Some(Arc::clone(&signal));
    let removed = Arc::clone(&fake.removed);
    let calls = Arc::clone(&fake.import_calls);
    let client = ImportClient::new(fake);
    let options = ImportOptions {
        signal: Some(Arc::clone(&signal)),
        ..Default::default()
    };

    let err = client
        .import_data("chrome", "Default", &[ImportKind::History], options)
        .unwrap_err();

    assert_eq!(err.code, "cancelled");
    assert_eq!(
        removed.load(Ordering::SeqCst),
        1,
        "event subscription removed"
    );
    assert_eq!(signal.listener_count(), 0);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "native import must not start"
    );
}
