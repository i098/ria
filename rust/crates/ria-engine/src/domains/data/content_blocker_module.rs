// Ported from ardia packages/ardiacore/ios/ContentBlockerModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `contentBlocker` call table.

use std::time::Duration;

use super::call_table::{BadCallArgs, CallArgs, CallReply, CallTable, Handler};
use super::content_blocker::ContentBlocker;

const CODE: &str = "ERR_CONTENT_BLOCKER";

type Outcome = Result<(), BadCallArgs>;

fn settled(reply: CallReply) -> Box<dyn FnOnce(Option<String>)> {
    Box::new(move |error| match error {
        Some(error) => reply.fail(CODE, error),
        None => reply.ok(None),
    })
}

fn state(b: &ContentBlocker, _: CallArgs, reply: CallReply) -> Outcome {
    b.state(Box::new(move |s| reply.ok(Some(s))));
    Ok(())
}

fn set_enabled(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.set_enabled(args.bool("enabled")?, settled(reply));
    Ok(())
}

fn set_list_enabled(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.set_list_enabled(&args.text("id")?, args.bool("enabled")?, settled(reply));
    Ok(())
}

fn add_list(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.add_list(
        &args.text("url")?,
        Box::new(move |result| match result {
            Ok(added) => reply.ok(Some(added)),
            Err(error) => reply.fail(CODE, error),
        }),
    );
    Ok(())
}

fn remove_list(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.remove_list(&args.text("id")?, settled(reply));
    Ok(())
}

fn is_allowed(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.is_allowed(
        &args.text("host")?,
        Box::new(move |on| reply.ok(Some(on.into()))),
    );
    Ok(())
}

fn set_allowed(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.set_allowed(args.bool("allowed")?, &args.text("host")?, settled(reply));
    Ok(())
}

fn hidden_elements(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.hidden_elements(
        &args.profile,
        Box::new(move |result| match result {
            Ok(filters) => reply.ok(Some(filters.into())),
            Err(error) => reply.fail(CODE, error),
        }),
    );
    Ok(())
}

fn hide_elements(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.hide_elements(&args.strings("filters")?, &args.profile, settled(reply));
    Ok(())
}

fn restore_elements(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    b.restore_elements(&args.strings("filters")?, &args.profile, settled(reply));
    Ok(())
}

fn dev_fail_next_message(b: &ContentBlocker, _: CallArgs, reply: CallReply) -> Outcome {
    b.dev_fail_next_message();
    reply.ok(None);
    Ok(())
}

fn dev_delay_next_load(b: &ContentBlocker, args: CallArgs, reply: CallReply) -> Outcome {
    let delay = Duration::try_from_secs_f64(args.double("ms")?.max(0.0) / 1000.0)
        .map_err(|_| BadCallArgs("ms is outside the supported delay range".into()))?;
    b.dev_delay_next_load(delay);
    reply.ok(None);
    Ok(())
}

static HANDLERS: &[(&str, Handler<ContentBlocker>)] = &[
    ("state", state),
    ("setEnabled", set_enabled),
    ("setListEnabled", set_list_enabled),
    ("addList", add_list),
    ("removeList", remove_list),
    ("isAllowed", is_allowed),
    ("setAllowed", set_allowed),
    ("hiddenElements", hidden_elements),
    ("hideElements", hide_elements),
    ("restoreElements", restore_elements),
    ("devFailNextMessage", dev_fail_next_message),
    ("devDelayNextLoad", dev_delay_next_load),
];

/// `ContentBlockerModule.calls`.
pub static CALLS: CallTable<ContentBlocker> = CallTable {
    domain: "contentBlocker",
    handlers: HANDLERS,
};

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::domains::data::call_table::{Rejection, Settled};
    use crate::domains::data::content_blocker::tests::setup;

    fn dispatch(
        b: &ContentBlocker,
        name: &str,
        profile: &str,
        args: &str,
    ) -> Rc<RefCell<Option<Settled>>> {
        let out = Rc::new(RefCell::new(None));
        let sink = out.clone();
        CALLS.run(
            b,
            name,
            profile,
            args,
            CallReply::new(move |s| *sink.borrow_mut() = Some(s)),
        );
        out
    }

    fn run(b: &ContentBlocker, name: &str, profile: &str, args: &str) -> Option<Settled> {
        dispatch(b, name, profile, args).take()
    }

    #[test]
    fn calls_map_to_the_blocker() {
        let (b, _, _, _resources) = setup();
        assert_eq!(CALLS.module_name(), "ArdiaContentBlocker");
        assert_eq!(
            run(&b, "devFailNextMessage", "", "{}"),
            Some(Ok("null".into()))
        );
        assert_eq!(
            run(&b, "devDelayNextLoad", "", r#"{"ms":5}"#),
            Some(Ok("null".into()))
        );
        assert!(
            matches!(run(&b, "devDelayNextLoad", "", r#"{"ms":1e30}"#), Some(Err(r)) if r.code == "ERR_BAD_ARGS")
        );
        assert_eq!(
            run(
                &b,
                "hideElements",
                "private-1",
                r#"{"filters":["a.com##.x"]}"#
            ),
            Some(Err(Rejection::new(
                CODE,
                "a private window keeps its hidden elements for the session"
            )))
        );
        assert_eq!(
            run(&b, "restoreElements", "", r#"{"filters":[]}"#),
            Some(Ok("null".into()))
        );
        assert!(
            matches!(run(&b, "setEnabled", "", "{}"), Some(Err(r)) if r.code == "ERR_BAD_ARGS")
        );
    }

    #[test]
    fn list_and_hidden_element_calls_return_payloads_or_errors() {
        let url = r#"{"url":"https://example.com/list.txt"}"#;
        for (call, args, script_reply, expected) in [
            (
                "addList", url,
                r#"{"value":{"id":"custom","custom":true,"existed":false,"filters":2,"failed":false}}"#,
                Ok(r#"{"custom":true,"existed":false,"failed":false,"filters":2,"id":"custom"}"#.to_owned()),
            ),
            (
                "addList", url, r#"{"value":null}"#,
                Err(Rejection::new(CODE, "no answer")),
            ),
            (
                "addList", url, r#"{"value":{}}"#,
                Err(Rejection::new(CODE, "no answer")),
            ),
            (
                "addList", url, r#"{"value":["custom",true,false,2,false]}"#,
                Err(Rejection::new(CODE, "no answer")),
            ),
            (
                "hiddenElements", "{}", r#"{"value":["example.com##.ad"]}"#,
                Ok(r#"["example.com##.ad"]"#.into()),
            ),
            (
                "hiddenElements", "{}", r#"{"error":"read failed"}"#,
                Err(Rejection::new(CODE, "read failed")),
            ),
        ] {
            let (blocker, host, scripts, _resources) = setup();
            let reply = dispatch(&blocker, call, "", args);
            let tab = scripts.tab(0);
            tab.load();
            tab.answer(script_reply);
            if call == "addList" {
                let changes = host.changes.borrow();
                if expected.is_ok() {
                    assert_eq!(changes.as_slice(), &[serde_json::json!({
                        "list": "custom",
                        "listEnabled": true,
                    })]);
                } else {
                    assert!(changes.is_empty(), "{script_reply}");
                }
            }
            let actual = reply.take().expect("the call settled");
            if call == "addList"
                && let (Ok(actual), Ok(expected)) = (&actual, &expected)
            {
                use crate::types::AddedFilterList;
                let actual: AddedFilterList = serde_json::from_str(actual).unwrap();
                let expected: AddedFilterList = serde_json::from_str(expected).unwrap();
                assert_eq!(actual, expected);
            } else {
                assert_eq!(actual, expected, "{call}: {script_reply}");
            }
        }
    }

    #[test]
    fn a_negative_delay_cancels_a_pending_delay() {
        let (b, _, _, _resources) = setup();
        assert_eq!(
            run(&b, "devDelayNextLoad", "", r#"{"ms":60000}"#),
            Some(Ok("null".into()))
        );
        assert_eq!(
            run(&b, "devDelayNextLoad", "", r#"{"ms":-1}"#),
            Some(Ok("null".into()))
        );
        b.load("");
        assert!(b.is_loaded(""));
    }
}
