// Ria original, no Ardia source.

//! Copies between Foundation values and `serde_json::Value`. The delegate uses them at the edge, so that no
//! Objective-C object goes past it (the deferred entry rule of plan section 6.4: copy the data, then queue it).

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::{NSData, NSError};
use serde_json::Value;

/// Copies an Objective-C value (dictionary, array, string, number, null) into JSON.
/// Returns `Value::Null` for a value that JSON cannot hold (for example an `NSDate`).
pub fn to_json(object: &AnyObject) -> Value {
    // `+isValidJSONObject:` is false for a bare string or number, so wrap the value in an array.
    let wrapper: Retained<AnyObject> =
        unsafe { msg_send![class!(NSArray), arrayWithObject: object] };
    let valid: bool =
        unsafe { msg_send![class!(NSJSONSerialization), isValidJSONObject: &*wrapper] };
    if !valid {
        return Value::Null;
    }
    let data: Option<Retained<NSData>> = unsafe {
        msg_send![class!(NSJSONSerialization), dataWithJSONObject: &*wrapper, options: 0usize, error: std::ptr::null_mut::<*mut NSError>()]
    };
    let Some(data) = data else {
        return Value::Null;
    };
    match serde_json::from_slice::<Vec<Value>>(data.to_vec().as_slice()) {
        Ok(mut list) if list.len() == 1 => list.remove(0),
        _ => Value::Null,
    }
}

/// Builds an Objective-C value from JSON (`NSDictionary`, `NSArray`, `NSString`, `NSNumber` or `NSNull`).
pub fn from_json(value: &Value) -> Retained<AnyObject> {
    let text = Value::Array(vec![value.clone()]).to_string();
    let data = NSData::with_bytes(text.as_bytes());
    // `NSJSONReadingFragmentsAllowed` is not needed: the value is wrapped in an array.
    let list: Retained<AnyObject> = unsafe {
        msg_send![class!(NSJSONSerialization), JSONObjectWithData: &*data, options: 0usize, error: std::ptr::null_mut::<*mut NSError>()]
    };
    unsafe { msg_send![&*list, objectAtIndex: 0usize] }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn json_round_trips_through_foundation() {
        let value = json!({"a": [1, 2.5, "x\u{65e5}\u{672c}", null, true], "b": {"c": "d"}});
        assert_eq!(to_json(&from_json(&value)), value);
    }

    #[test]
    fn scalars_round_trip() {
        for value in [json!("text"), json!(7), json!(false)] {
            assert_eq!(to_json(&from_json(&value)), value);
        }
    }

    #[test]
    fn a_value_json_cannot_hold_becomes_null() {
        let date: Retained<AnyObject> = unsafe { msg_send![class!(NSDate), date] };
        assert_eq!(to_json(&date), Value::Null);
    }
}
