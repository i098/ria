// Ria original, no Ardia source.

//! The `NSUserDefaults` store of the download mover (`ArdiaDownloadMover.mm` keeps its journal and its moved list there).

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::NSString;
use ria_engine::domains::data::download_mover::Defaults;
use serde_json::Value;

use super::convert::{from_json, to_json};

/// `NSUserDefaults.standardUserDefaults`.
pub struct StandardDefaults;

fn store() -> Retained<AnyObject> {
    unsafe { msg_send![class!(NSUserDefaults), standardUserDefaults] }
}

impl Defaults for StandardDefaults {
    fn get(&self, key: &str) -> Option<Value> {
        let object: Option<Retained<AnyObject>> =
            unsafe { msg_send![&*store(), objectForKey: &*NSString::from_str(key)] };
        object.map(|object| to_json(&object))
    }

    fn set(&self, key: &str, value: Value) {
        let _: () = unsafe {
            msg_send![&*store(), setObject: &*from_json(&value), forKey: &*NSString::from_str(key)]
        };
    }

    fn remove(&self, key: &str) {
        let _: () = unsafe { msg_send![&*store(), removeObjectForKey: &*NSString::from_str(key)] };
    }
}
