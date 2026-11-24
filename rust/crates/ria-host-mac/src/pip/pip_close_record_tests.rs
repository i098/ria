// Ported from ardia packages/ardiacore/tests/pip-close-record.test.mjs at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The tests read the Ardia sources of the engine and the host. They look in `RIA_ARDIA_SOURCE`,
//! then in the `ardia/` submodule at the repository root; when neither holds the sources, they skip.

use std::path::PathBuf;

struct Sources {
    observer: String,
    window_host: String,
    tab_media: String,
    pip_native: String,
    painting: String,
    host_bridge: String,
    engine: String,
    header: String,
}

fn sources() -> Option<Sources> {
    let root = std::env::var_os("RIA_ARDIA_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../ardia"));
    let read = |path: &str| std::fs::read_to_string(root.join(path)).ok();
    let core = "engine/ardiacore/src/ardia/core/";
    let ios = "packages/ardiacore/ios/";
    let sources = Sources {
        observer: read(&format!("{core}ac_picture_in_picture.mm"))?,
        window_host: read(&format!("{core}ac_window_host.mm"))?,
        tab_media: read(&format!("{core}ardiacore_tab_media.mm"))?,
        pip_native: read(&format!("{ios}ArdiaWebView+PictureInPicture.mm"))?,
        painting: read(&format!("{ios}ArdiaWebView+Painting.mm"))?,
        host_bridge: read(&format!("{ios}ArdiaHost.mm"))?,
        engine: read(&format!("{core}ardiacore_engine.mm"))?,
        header: read(&format!("{core}public/ArdiaCore.h"))?,
    };
    Some(sources)
}

macro_rules! sources_or_skip {
    () => {
        match sources() {
            Some(sources) => sources,
            None => {
                eprintln!("skipped: the Ardia sources are not present (set RIA_ARDIA_SOURCE)");
                return;
            }
        }
    };
}

/// The text from `signature` to the brace that closes its first block.
fn body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("{signature} exists"));
    let mut depth = 0;
    let open = start + source[start..].find('{').expect("a block opens");
    for (offset, byte) in source[open..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start..=open + offset];
                }
            }
            _ => {}
        }
    }
    panic!("{signature} has no end");
}

fn has_in_order(text: &str, parts: &[&str]) -> bool {
    let mut from = 0;
    for part in parts {
        match text[from..].find(part) {
            Some(at) => from += at + part.len(),
            None => return false,
        }
    }
    true
}

/// `constexpr size_t <name> = <digits>;`
fn has_limit(text: &str, name: &str) -> bool {
    let head = format!("constexpr size_t {name} = ");
    text.find(&head).is_some_and(|at| {
        let rest = &text[at + head.len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && rest[digits..].starts_with(';')
    })
}

#[test]
fn the_engine_records_every_picture_in_picture_close_without_the_pip_trace() {
    let s = sources_or_skip!();
    let exit = body(&s.observer, "void OnExitPictureInPicture() override");
    assert!(exit.contains("TakeExitRequest()"));
    assert!(exit.contains("RecordClose(opener_.get(), kind_, entered_auto_, by_app)"));
    assert!(
        exit.contains(
            "Report(opener_, false, kind_, by_app ? @\"app\" : @\"chrome\", entered_auto_)"
        )
    );
    let record_at = exit.find("RecordClose(").expect("the record call");
    assert!(
        record_at
            > exit
                .rfind("PictureInPictureTracing()")
                .expect("the trace block"),
        "the record is written outside the trace block"
    );
    assert!(
        record_at < exit.find("if (!opener_)").expect("the opener check"),
        "the record is written even when the opener is gone"
    );

    let record = body(&s.observer, "void RecordClose(");
    assert!(!record.contains("PictureInPictureTracing") && !record.contains("Traced("));
    for field in [
        "by=%s",
        "kind=%s",
        "enteredAuto=%d",
        "chromeAuto=%d",
        "hostShown=%d",
        "activations=[%s]",
    ] {
        assert!(record.contains(field), "the close record has {field}");
    }
    assert!(record.contains("IsInAutoPictureInPicture()"));
    assert!(record.contains("ShownByHost(opener)"));
    assert!(record.contains("kCloseRecordLimit"));
    assert!(has_limit(&s.observer, "kCloseRecordLimit"));
    assert!(has_limit(&s.observer, "kActivationNoteLimit"));
}

#[test]
fn the_engine_keeps_the_last_window_activation_changes_with_the_window_and_its_age_bounded() {
    let s = sources_or_skip!();
    let note = body(&s.observer, "void NoteWindowActivation(");
    assert!(!note.contains("PictureInPictureTracing") && !note.contains("Traced("));
    assert!(note.contains("kActivationNoteLimit"));
    assert!(body(&s.observer, "std::string RecentActivations(").contains("window=%ld ageMs=%ld"));
    assert!(
        s.observer
            .contains("entered_auto_ = helper && helper->IsInAutoPictureInPicture()")
    );
}

#[test]
fn both_window_activation_paths_in_the_window_host_note_the_change() {
    let s = sources_or_skip!();
    assert!(body(&s.window_host, "void ActivateExclusively(").contains(
        "NoteWindowActivation(\"activate exclusively\", target->ns_window().windowNumber)"
    ));
    let widget = body(
        &s.window_host,
        "void WindowHost::OnWidgetActivationChanged(",
    );
    assert!(
        widget.contains("NoteWindowActivation(active ? \"widget active\" : \"widget inactive\"")
    );
    assert!(
        widget.find("NoteWindowActivation").expect("the note")
            < widget.find("ActivateExclusively(").expect("the call"),
        "the change is noted before the other windows are deactivated"
    );
    assert_eq!(
        s.window_host
            .matches("ActivateExclusively(this, browser)")
            .count(),
        2
    );
}

#[test]
fn a_close_the_app_asked_for_is_told_apart_from_one_chrome_started() {
    let s = sources_or_skip!();
    assert!(has_in_order(
        body(&s.tab_media, "exitPictureInPicture"),
        &[
            "NotePictureInPictureExitRequested()",
            "ExitPictureInPicture()"
        ]
    ));
    assert!(s.observer.contains("by_app ? @\"app\" : @\"chrome\""));
}

#[test]
fn the_native_exit_branch_and_the_sight_rule_record_their_reasons_without_the_pip_trace() {
    let s = sources_or_skip!();
    let note = body(&s.pip_native, "- (void)notePictureInPictureSight:");
    assert!(!note.contains("TracingPictureInPicture"));
    for field in ["seenByUser=%d", "visible=%d", "appActive=%d", "key=%d"] {
        assert!(note.contains(field), "the sight note has {field}");
    }
    assert!(note.contains("NSApp.isActive"));
    assert!(note.contains("isKeyWindow"));
    assert!(note.contains("ardia_host::NoteFocusChange("));

    let update = body(&s.pip_native, "- (void)updateAutoPictureInPicture");
    let branch = update
        .find("else if (_reports.seenByUser")
        .expect("the exit branch");
    let after_condition = &update[branch..];
    let open = after_condition.find('{').expect("the branch opens");
    assert!(
        after_condition[open + 1..]
            .trim_start()
            .starts_with("[self notePictureInPictureSight:@\"exit requested\"];")
    );
    assert!(has_in_order(
        &after_condition[open..],
        &[
            "_reports.autoPictureInPictureActive = NO;",
            "[self exitPictureInPicture];"
        ]
    ));

    let sight = body(&s.painting, "- (void)userSightChanged");
    let noted = sight
        .find("notePictureInPictureSight:seen ? @\"sight seen\" : @\"sight left\"")
        .expect("the sight note");
    assert!(
        noted
            < sight
                .find("updateAutoPictureInPicture")
                .expect("the update")
    );
    assert!(!sight[noted.saturating_sub(160)..noted].contains("TracingPictureInPicture"));
}

#[test]
fn the_close_reaches_the_page_as_closed_by_and_auto_and_the_record_reaches_copy_diagnostics() {
    let s = sources_or_skip!();
    let change = body(
        &s.pip_native,
        "- (void)tab:(ArdiaCoreTab *)tab didChangePictureInPicture:",
    );
    assert!(change.contains("payload[@\"closedBy\"] = state[@\"closedBy\"]"));
    assert!(change.contains("payload[@\"auto\"]"));
    assert!(
        change.find("payload[@\"closedBy\"]").expect("closedBy")
            < change.find("[self emit:").expect("emit"),
        "the close fields are on the payload before either kind emits it"
    );
    assert_eq!(
        change
            .matches("[self emit:@\"pictureInPicture\" payload:payload]")
            .count(),
        2,
        "video and document closes both emit the payload with the close fields"
    );
    assert!(
        s.host_bridge
            .contains("@\"pictureInPictureCloses\" : engine.pictureInPictureCloses ?: @\"\",")
    );
    assert!(has_in_order(
        &s.engine,
        &[
            "- (NSString*)pictureInPictureCloses {",
            "return NS(ardiacore::PictureInPictureCloseRecord());"
        ]
    ));
    assert!(
        s.header
            .contains("@property(readonly) NSString* pictureInPictureCloses;")
    );
}
