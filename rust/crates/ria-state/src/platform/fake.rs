// Ria original, no Ardia source.

//! The fake [`Platform`] of the replay drivers: it records each call and answers with the result that the
//! TypeScript run recorded.

use std::collections::VecDeque;

use serde_json::{Value, json};

use super::{Platform, PlatformCall};
use crate::kernel::AppAction;

/// One recorded answer: the `result` of the TypeScript call, and the result action that `ria-app` queues for it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Answer {
    /// The recorded result. `None` is `undefined`, which `JSON.stringify` drops from the record.
    pub result: Option<Value>,
    pub action: Option<AppAction>,
}

/// Records every call and answers in order with the queued [`Answer`]s; a call with no answer left gets `Answer::default()`.
#[derive(Debug, Default)]
pub struct FakePlatform {
    answers: VecDeque<Answer>,
    calls: Vec<Value>,
}

impl FakePlatform {
    pub fn new(answers: impl IntoIterator<Item = Answer>) -> Self {
        Self {
            answers: answers.into_iter().collect(),
            calls: Vec::new(),
        }
    }

    /// Takes the calls since the last take, as the `{name, args, result}` records of `nativeFake().takeCalls()`.
    pub fn take_calls(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.calls)
    }
}

impl Platform for FakePlatform {
    fn run(&mut self, call: PlatformCall) -> Option<AppAction> {
        let answer = self.answers.pop_front().unwrap_or_default();
        let mut record = json!({ "name": call.name(), "args": call.args() });
        if let Some(result) = answer.result {
            record["result"] = result;
        }
        self.calls.push(record);
        answer.action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::Number;

    #[test]
    fn records_calls_in_the_shape_of_the_typescript_fake() {
        let mut fake = FakePlatform::new([Answer {
            result: Some(json!(4)),
            action: None,
        }]);
        fake.run(PlatformCall::ShellCopyText {
            text: "héllo".into(),
        });
        fake.run(PlatformCall::TabsReportCommits { on: true });
        fake.run(PlatformCall::EngineInfo {});
        fake.run(PlatformCall::MediaStartCasting {
            id: Number(7.0),
            sink: "s".into(),
            mode: Number(1.5),
        });
        fake.run(PlatformCall::AutofillSetSettings {
            profile: "p".into(),
            settings: json!({"cards": true}),
        });
        assert_eq!(
            fake.take_calls(),
            vec![
                json!({"name": "ArdiaShell.copyText", "args": ["héllo"], "result": 4}),
                json!({"name": "tabs.reportCommits", "args": [{"on": true}]}),
                json!({"name": "engine.info", "args": []}),
                json!({"name": "media.startCasting", "args": [{"id": 7, "sink": "s", "mode": 1.5}]}),
                json!({"name": "autofill.setSettings", "args": [{"profile": "p", "cards": true}]}),
            ]
        );
        assert!(fake.take_calls().is_empty());
    }

    #[test]
    fn answers_come_in_order_and_run_out_to_none() {
        let action = Some(AppAction::Find(crate::find::Action::FindStep {
            tab_id: "t".into(),
            backwards: true,
        }));
        let mut fake = FakePlatform::new([Answer {
            result: None,
            action: action.clone(),
        }]);
        assert_eq!(fake.run(PlatformCall::ShellReadText {}), action);
        assert_eq!(fake.run(PlatformCall::ShellReadText {}), None);
    }
}
