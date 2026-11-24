// Ria original, no Ardia source.

//! The engine omits `status` from a `search.suggest` reply on stop and on net error
//! (`ac_search_suggest.cc`), so the typed reply must decode both shapes.

use ria_engine::api::search::SuggestReply;
use serde_json::json;

#[test]
fn decodes_stopped_reply_without_status() {
    let reply: SuggestReply = serde_json::from_value(json!({ "stopped": true })).unwrap();
    assert_eq!(reply.stopped, Some(true));
    assert!(reply.status.is_none());
    assert!(reply.body.is_none());
}

#[test]
fn decodes_net_error_reply_without_status() {
    let reply: SuggestReply = serde_json::from_value(json!({ "netError": -106 })).unwrap();
    assert_eq!(reply.net_error.map(|n| n.0), Some(-106.0));
    assert!(reply.status.is_none());
}
