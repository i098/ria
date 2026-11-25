// Ria original, no Ardia source.

//! The replay driver of `state/app`: reads `[action]` as JSON on stdin and writes `[{state, calls}]` on stdout.
//! `check.mjs` compares the output with the recording of the Ardia store (plan section 6.5, check S).

use std::error::Error;
use std::io;

use ria_state::kernel::{AppAction, AppState, Effects};
use ria_state::platform::{FakePlatform, Platform};
use ria_state::{app, settings};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn Error>> {
    let actions: Vec<app::Action> = serde_json::from_reader(io::stdin().lock())?;
    let mut state = AppState::default();
    // The recording starts with `cliControl` off, as the driver of Ardia sets it.
    state.get_mut::<settings::State>().settings.cli_control = false;
    let mut platform = FakePlatform::new([]);
    let mut steps = Vec::new();
    for action in actions {
        let mut fx = Effects::default();
        app::apply(&mut state, action, &mut fx);
        for call in fx.take() {
            let _: Option<AppAction> = platform.run(call);
        }
        let shown: Value = serde_json::to_value(state.get::<app::State>())?;
        steps.push(json!({ "state": shown, "calls": platform.take_calls() }));
    }
    serde_json::to_writer(io::stdout().lock(), &steps)?;
    Ok(())
}
