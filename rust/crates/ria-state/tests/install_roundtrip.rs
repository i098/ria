// Ria original, no Ardia source.
use ria_state::extensions::State;
use serde_json::{Value, json};

/// Every real install state of the TypeScript store survives a trip through serde unchanged.
#[test]
fn every_install_state_round_trips() {
    for source in ["webStore", "unpacked"] {
        let base = json!({ "key": 3, "windowId": "w1", "profile": "p1", "source": source });
        let with = |extra: Value| {
            let mut v = base.clone();
            v.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            v
        };
        let installs = [
            with(json!({ "status": "error", "error": "boom" })),
            with(json!({ "status": "ready", "pkg": { "id": "x" } })),
            with(json!({ "status": "installing", "pkg": { "id": "x" } })),
            with(json!({ "status": "prompt", "prompt": { "name": "x" } })),
        ];
        for install in installs {
            let mut state_json = serde_json::to_value(State::default()).unwrap();
            state_json["install"] = install;
            let state: State = serde_json::from_value(state_json.clone()).unwrap();
            assert_eq!(
                serde_json::to_value(&state).unwrap(),
                state_json,
                "{state_json}"
            );
        }
    }
}

/// A required field of type `T | undefined` is absent from the JSON when it is undefined.
#[test]
fn undefined_field_is_absent() {
    let state = ria_state::commands::State::default();
    assert!(state.bar.is_none());
    let json = serde_json::to_value(&state).unwrap();
    assert!(json.get("bar").is_none());
    let back: ria_state::commands::State = serde_json::from_value(json).unwrap();
    assert_eq!(back, state);
}
