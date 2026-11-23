// Ported from ardia packages/ardiacore/tests/download-mover.test.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The original runs its crash and lock checks on a second thread; here the move runs on one thread, so a crash
//! is a panic from the step hook, and the long-hash check proves events can be noted while a move is running.

use std::cell::Cell;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::download_mover::{Defaults, DownloadMover, MemoryDefaults};

struct Env {
    home: String,
    source: String,
    next_id: Cell<u32>,
    defaults: Arc<MemoryDefaults>,
}

impl Env {
    fn at(&self, relative: &str) -> String {
        format!("{}/{relative}", self.home)
    }
    fn make(&self, relative: &str, text: &str) -> String {
        let path = self.at(relative);
        fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }
    fn mover(&self) -> DownloadMover {
        DownloadMover::new(&self.home, self.defaults.clone())
    }
    fn finished_event(&self, path: &str, downloaded: &str) -> Value {
        self.next_id.set(self.next_id.get() + 1);
        json!({"id": format!("d{}", self.next_id.get()), "path": path, "state": "finished", "dangerous": false,
               "offTheRecord": false, "filename": path.rsplit('/').next().unwrap(), "sha256": sha256(downloaded.as_bytes()),
               "received": downloaded.len()})
    }
    fn finished(&self, mover: &DownloadMover, name: &str, text: &str, extra: Value) -> String {
        let path = self.make(&format!("src/{name}"), text);
        let mut event = self.finished_event(&path, text);
        for (key, value) in extra.as_object().unwrap() {
            event[key] = value.clone();
        }
        mover.note_download_event(&event);
        path
    }
    fn last_id(&self) -> String {
        format!("d{}", self.next_id.get())
    }
    fn staging_dirs(&self) -> usize {
        fs::read_dir(&self.source)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".ardia-moving-")
            })
            .count()
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn exists(path: &str) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn text(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn reason(result: &Value) -> String {
    if result["ok"] == true {
        "ok".into()
    } else {
        result["reason"].as_str().unwrap_or_default().into()
    }
}

fn hook(mover: &DownloadMover, f: impl Fn(&str, &str) + Send + Sync + 'static) {
    *mover.step_hook.lock().unwrap() = Some(Arc::new(f));
}

fn unhook(mover: &DownloadMover) {
    *mover.step_hook.lock().unwrap() = None;
}

#[test]
fn download_mover() {
    let base = std::env::temp_dir().join(format!("ria-download-mover-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let home = fs::canonicalize(&base)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let env = Env {
        source: format!("{home}/src"),
        home,
        next_id: Cell::new(0),
        defaults: Arc::new(MemoryDefaults::default()),
    };
    fs::create_dir_all(env.at("Downloads")).unwrap();
    let mut mover = env.mover();
    let none = json!({});

    {
        let stranger = env.make("src/stranger.txt", "not ours");
        assert_eq!(
            reason(&mover.move_download(&stranger, "~/Downloads")),
            "notADownload"
        );
        assert!(exists(&stranger));
    }
    {
        let path = env.make("src/partial.bin", "partial");
        mover.note_download_event(&json!({"id": "partial", "path": path, "state": "downloading"}));
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/Sorted")),
            "notFinished"
        );
        let risky = env.finished(&mover, "risky.dmg", "x", json!({"dangerous": true}));
        assert_eq!(
            reason(&mover.move_download(&risky, "~/Downloads/Sorted")),
            "dangerous"
        );
        let hidden = env.finished(&mover, "private.txt", "x", json!({"offTheRecord": true}));
        assert_eq!(
            reason(&mover.move_download(&hidden, "~/Downloads/Sorted")),
            "private"
        );
        assert!(exists(&path) && exists(&risky) && exists(&hidden));
    }
    {
        let path = env.finished(&mover, "rules.txt", "rules", none.clone());
        for folder in [
            "/tmp",
            "Downloads",
            "~/",
            "~",
            "~/../Downloads",
            "~/Downloads/../Documents",
            "~/Downloads/./x",
            "~/Downloads//x",
            "~/Documents",
            "~root/Downloads",
        ] {
            assert_eq!(
                reason(&mover.move_download(&path, folder)),
                "folderNotAllowed",
                "{folder}"
            );
        }
        assert!(exists(&path));
    }
    {
        let outside = env.at("outside");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, env.at("Downloads/link")).unwrap();
        let path = env.finished(&mover, "linked.txt", "linked", none.clone());
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/link/sub")),
            "symlink"
        );
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/link")),
            "symlink"
        );
        assert!(exists(&path));
        assert!(!exists(&format!("{outside}/sub")));
        assert!(!exists(&format!("{outside}/linked.txt")));
        env.make("Downloads/plain", "a file");
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/plain/sub")),
            "notAFolder"
        );
    }
    {
        let path = env.finished(&mover, "report.pdf", "first", none.clone());
        #[cfg(target_os = "linux")]
        let attribute = c"user.ardia.quarantine";
        #[cfg(target_os = "macos")]
        let attribute = c"com.apple.quarantine";
        const QUARANTINE: &[u8] = b"0083;00000000;Ardia;";
        let cpath = std::ffi::CString::new(path.clone()).unwrap();
        // SAFETY: valid C strings and a value buffer of the specified length.
        let set = unsafe {
            #[cfg(target_os = "linux")]
            {
                libc::setxattr(
                    cpath.as_ptr(),
                    attribute.as_ptr(),
                    QUARANTINE.as_ptr().cast(),
                    QUARANTINE.len(),
                    0,
                )
            }
            #[cfg(target_os = "macos")]
            {
                libc::setxattr(
                    cpath.as_ptr(),
                    attribute.as_ptr(),
                    QUARANTINE.as_ptr().cast(),
                    QUARANTINE.len(),
                    0,
                    0,
                )
            }
        };
        assert_eq!(set, 0, "{}", std::io::Error::last_os_error());
        let result = mover.move_download(&path, "~/Downloads/Sorted/2026");
        assert_eq!(reason(&result), "ok");
        let moved = env.at("Downloads/Sorted/2026/report.pdf");
        assert_eq!(result["path"], json!(moved));
        assert_eq!(text(&moved), "first");
        assert!(!exists(&path));
        let cmoved = std::ffi::CString::new(moved.clone()).unwrap();
        let mut copied = [0u8; QUARANTINE.len()];
        // SAFETY: valid C strings and a writable buffer of the specified length.
        let size = unsafe {
            #[cfg(target_os = "linux")]
            {
                libc::getxattr(
                    cmoved.as_ptr(),
                    attribute.as_ptr(),
                    copied.as_mut_ptr().cast(),
                    copied.len(),
                )
            }
            #[cfg(target_os = "macos")]
            {
                libc::getxattr(
                    cmoved.as_ptr(),
                    attribute.as_ptr(),
                    copied.as_mut_ptr().cast(),
                    copied.len(),
                    0,
                    0,
                )
            }
        };
        assert_eq!(size, QUARANTINE.len() as isize);
        assert_eq!(&copied, QUARANTINE);
        assert_eq!(result["event"]["path"], json!(moved));

        let second = env.finished(&mover, "report.pdf", "second", none.clone());
        let again = mover.move_download(&second, "~/Downloads/Sorted/2026/");
        assert_eq!(
            again["path"],
            json!(env.at("Downloads/Sorted/2026/report (1).pdf"))
        );
        assert_eq!(text(&moved), "first");
        assert_eq!(text(again["path"].as_str().unwrap()), "second");

        let tarball = env.finished(&mover, "site.tar.gz", "one", none.clone());
        mover.move_download(&tarball, "~/Downloads/Sorted");
        let tarball2 = env.finished(&mover, "site.tar.gz", "two", none.clone());
        assert!(
            mover.move_download(&tarball2, "~/Downloads/Sorted")["path"]
                .as_str()
                .unwrap()
                .ends_with("/site (1).tar.gz")
        );

        let stale = mover.note_download_event(&json!({"id": "unknown", "path": "/nowhere"}));
        assert_eq!(stale["path"], "/nowhere");
        let moved_id = format!("d{}", env.next_id.get() - 2);
        let replayed = mover
            .note_download_event(&json!({"id": moved_id, "path": second, "state": "finished"}));
        assert_eq!(replayed["path"], again["path"]);
        let relaunched = env.mover();
        let after_relaunch = relaunched
            .note_download_event(&json!({"id": moved_id, "path": second, "state": "finished"}));
        assert_eq!(after_relaunch["path"], again["path"]);
    }
    {
        let path = env.finished(&mover, "swapped.txt", "original", none.clone());
        fs::remove_file(&path).unwrap();
        env.make("src/swapped.txt", "impostor");
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads")),
            "changed"
        );
        assert_eq!(text(&path), "impostor");
        let gone = env.finished(&mover, "gone.txt", "x", none.clone());
        fs::remove_file(&gone).unwrap();
        assert_eq!(
            reason(&mover.move_download(&gone, "~/Downloads")),
            "missing"
        );
    }
    {
        let path = env.finished(&mover, "finance.csv", "numbers", none.clone());
        assert_eq!(
            reason(&mover.move_download(&path, "~/Docs/Finance")),
            "folderNotAllowed"
        );
        mover.set_approved_folders(&[
            "~/Docs/Finance".into(),
            "/etc".into(),
            "~/../escape".into(),
            "Docs".into(),
        ]);
        assert_eq!(
            reason(&mover.move_download(&path, "~/Docs/Finance")),
            "folderMissing"
        );
        assert!(!exists(&env.at("Docs")));
        fs::create_dir_all(env.at("Docs")).unwrap();
        assert_eq!(
            reason(&mover.move_download(&path, "~/Docs/Finance/2026")),
            "ok"
        );
        assert_eq!(text(&env.at("Docs/Finance/2026/finance.csv")), "numbers");
        let other = env.finished(&mover, "other.csv", "x", none.clone());
        assert_eq!(
            reason(&mover.move_download(&other, "~/Docs")),
            "folderNotAllowed"
        );
        assert_eq!(
            reason(&mover.move_download(&other, "~/etc")),
            "folderNotAllowed"
        );
        mover.set_approved_folders(&[]);
        assert_eq!(
            reason(&mover.move_download(&other, "~/Docs/Finance")),
            "folderNotAllowed"
        );
    }
    {
        let path = env.make("src/no-hash.txt", "webkit");
        let mut event = env.finished_event(&path, "webkit");
        event.as_object_mut().unwrap().remove("sha256");
        mover.note_download_event(&event);
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads")),
            "noIdentity"
        );
        assert!(exists(&path));
    }
    {
        let path = env.make("src/invoice.pdf", "an impostor another process put there");
        mover.note_download_event(&env.finished_event(&path, "the bytes the browser downloaded"));
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/Bills")),
            "changed"
        );
        assert_eq!(text(&path), "an impostor another process put there");
        assert!(!exists(&env.at("Downloads/Bills/invoice.pdf")));
        assert_eq!(env.staging_dirs(), 0);
    }
    {
        let path = env.finished(&mover, "before-claim.txt", "genuine", none.clone());
        fs::remove_file(&path).unwrap();
        env.make("src/before-claim.txt", "swapped in before the claim");
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/Bills")),
            "changed"
        );
        assert_eq!(text(&path), "swapped in before the claim");
        assert!(!exists(&env.at("Downloads/Bills/before-claim.txt")));
        assert_eq!(env.staging_dirs(), 0);
    }
    for blocked in [false, true] {
        let path = env.finished(&mover, "removed-destination.txt", "genuine", none.clone());
        let id = env.last_id();
        let destination = env.at("Downloads/Removed");
        fs::create_dir(&destination).unwrap();
        let source = path.clone();
        hook(&mover, move |step, _| {
            if step == "verified" {
                fs::remove_dir(&destination).unwrap();
                if blocked {
                    fs::write(&source, "replacement").unwrap();
                }
            }
        });
        let result = mover.move_download(&path, "~/Downloads/Removed");
        unhook(&mover);
        assert_eq!(reason(&result), "failed");
        if blocked {
            assert_eq!(text(&path), "replacement");
            let staged = result["stagedPath"].as_str().unwrap();
            assert_eq!(text(staged), "genuine");
            let replay = mover.note_download_event(&json!({"id": id, "path": path}));
            assert_eq!(replay["path"], staged);
            fs::remove_dir_all(std::path::Path::new(staged).parent().unwrap()).unwrap();
        } else {
            assert_eq!(text(&path), "genuine");
            assert_eq!(env.staging_dirs(), 0);
        }
    }
    {
        let path = env.finished(&mover, "after-check.txt", "genuine bytes", none.clone());
        hook(&mover, |step, staging| {
            if step != "verified" {
                return;
            }
            let staged = format!("{staging}/after-check.txt");
            fs::rename(&staged, format!("{staging}/hidden-away")).unwrap();
            fs::write(&staged, "swapped in after the check").unwrap();
        });
        let result = mover.move_download(&path, "~/Downloads/Bills");
        unhook(&mover);
        assert_eq!(reason(&result), "changed");
        assert!(!exists(&env.at("Downloads/Bills/after-check.txt")));
        let staging = result["stagedPath"].as_str().unwrap_or_default();
        assert!(!staging.is_empty() && exists(staging));
        assert_eq!(text(&format!("{staging}/hidden-away")), "genuine bytes");
        assert_eq!(
            text(&format!("{staging}/not-after-check.txt")),
            "swapped in after the check"
        );
        fs::remove_dir_all(staging).unwrap();
    }
    {
        let path = env.finished(&mover, "blocked.txt", "genuine", none.clone());
        let source = env.source.clone();
        hook(&mover, move |step, staging| {
            if step != "claimed" {
                return;
            }
            fs::OpenOptions::new()
                .append(true)
                .open(format!("{staging}/blocked.txt"))
                .unwrap()
                .write_all(b" and tampered")
                .unwrap();
            fs::write(
                format!("{source}/blocked.txt"),
                "something new at the old name",
            )
            .unwrap();
        });
        let result = mover.move_download(&path, "~/Downloads/Bills");
        unhook(&mover);
        assert_eq!(reason(&result), "changed");
        assert_eq!(text(&path), "something new at the old name");
        let staged = result["stagedPath"].as_str().unwrap();
        assert!(staged.ends_with("/blocked.txt"));
        assert_eq!(text(staged), "genuine and tampered");
        assert!(!exists(&env.at("Downloads/Bills/blocked.txt")));
        let staging_dir = staged.trim_end_matches("/blocked.txt");
        assert_eq!(
            fs::metadata(staging_dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        fs::remove_dir_all(staging_dir).unwrap();
    }
    {
        let path = env.finished(&mover, "clean.txt", "clean", none.clone());
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/Bills")),
            "ok"
        );
        assert_eq!(env.staging_dirs(), 0);
    }
    {
        let path = env.finished(&mover, "linked-before.txt", "two names", none.clone());
        let other = env.at("elsewhere-link.txt");
        fs::hard_link(&path, &other).unwrap();
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/Linked")),
            "linked"
        );
        assert_eq!(text(&path), "two names");
        assert_eq!(env.staging_dirs(), 0);
        fs::remove_file(&other).unwrap();
    }
    {
        let path = env.finished(&mover, "linked-after.txt", "linked later", none.clone());
        let other = env.at("later-link.txt");
        let link = other.clone();
        hook(&mover, move |step, staging| {
            if step == "verified" {
                fs::hard_link(format!("{staging}/linked-after.txt"), &link).unwrap();
            }
        });
        let result = mover.move_download(&path, "~/Downloads/Linked");
        unhook(&mover);
        assert_eq!(reason(&result), "linked");
        assert_eq!(text(&path), "linked later");
        assert!(!exists(&env.at("Downloads/Linked/linked-after.txt")));
        assert_eq!(env.staging_dirs(), 0);
        fs::remove_file(&other).unwrap();
    }
    {
        let path = env.finished(&mover, "stray.txt", "genuine stray", none.clone());
        hook(&mover, |step, staging| {
            if step != "verified" {
                return;
            }
            let staged = format!("{staging}/stray.txt");
            fs::rename(&staged, format!("{staging}/kept")).unwrap();
            fs::write(&staged, "impostor").unwrap();
            fs::write(format!("{staging}/not-stray.txt"), "blocker").unwrap();
        });
        let result = mover.move_download(&path, "~/Downloads/Strays");
        unhook(&mover);
        assert_eq!(reason(&result), "changed");
        let stray_path = env.at("Downloads/Strays/stray.txt");
        assert_eq!(result["strayPath"], json!(stray_path));
        assert_eq!(text(&stray_path), "impostor");
        let staged = result["stagedPath"].as_str().unwrap();
        assert_eq!(text(&format!("{staged}/kept")), "genuine stray");
        fs::remove_dir_all(staged).unwrap();
    }
    {
        let path = env.finished(&mover, "renamed.txt", "genuine", none.clone());
        let renamed = format!("{}/renamed-staging", env.source);
        let (source, target) = (env.source.clone(), renamed.clone());
        hook(&mover, move |step, staging| {
            if step != "claimed" {
                return;
            }
            fs::OpenOptions::new()
                .append(true)
                .open(format!("{staging}/renamed.txt"))
                .unwrap()
                .write_all(b"!")
                .unwrap();
            fs::write(format!("{source}/renamed.txt"), "blocker").unwrap();
            fs::rename(staging, &target).unwrap();
        });
        let result = mover.move_download(&path, "~/Downloads/Bills");
        unhook(&mover);
        assert_eq!(
            result["stagedPath"],
            json!(format!("{renamed}/renamed.txt"))
        );
        assert_eq!(text(result["stagedPath"].as_str().unwrap()), "genuine!");
        let rewritten = mover.note_download_event(&json!({"id": env.last_id(), "path": path}));
        assert_eq!(rewritten["path"], result["stagedPath"]);
        fs::remove_dir_all(&renamed).unwrap();
    }
    {
        // A crash is a panic from the step hook: nothing after it runs, as when the process dies there.
        let crash_at = |mover: &DownloadMover, name: &str, content: &str, at: &'static str| {
            let path = env.finished(mover, name, content, json!({}));
            hook(mover, move |step, _| {
                assert_ne!(step, at, "simulated crash")
            });
            assert!(
                catch_unwind(AssertUnwindSafe(
                    || mover.move_download(&path, "~/Downloads/Crash")
                ))
                .is_err()
            );
            path
        };
        let freed = crash_at(&mover, "crash-free.txt", "put back", "claimed");
        let mut restarted = env.mover();
        assert_eq!(text(&freed), "put back");
        assert_eq!(env.staging_dirs(), 0);
        assert!(env.defaults.get("ArdiaDownloadMoveJournal").is_none());

        mover = restarted;
        let taken = crash_at(&mover, "crash-taken.txt", "still mine", "claimed");
        let taken_id = env.last_id();
        env.make("src/crash-taken.txt", "someone else");
        restarted = env.mover();
        let reported =
            restarted.note_download_event(&json!({"id": taken_id, "path": taken}))["path"]
                .as_str()
                .unwrap()
                .to_owned();
        assert!(reported.starts_with(&format!("{}/.ardia-moving-", env.source)));
        assert_eq!(text(&reported), "still mine");
        assert_eq!(text(&taken), "someone else");
        fs::remove_dir_all(std::path::Path::new(&reported).parent().unwrap()).unwrap();

        mover = restarted;
        let placed = crash_at(&mover, "crash-placed.txt", "landed", "placed");
        let placed_id = env.last_id();
        restarted = env.mover();
        let landed = env.at("Downloads/Crash/crash-placed.txt");
        let committed = restarted.note_download_event(&json!({"id": placed_id, "path": placed}));
        assert_eq!(committed["path"], json!(landed));
        assert_eq!(text(&landed), "landed");
        assert_eq!(env.staging_dirs(), 0);
        mover = restarted;
    }
    {
        let mover = Arc::new(mover);
        let size = 8u64 << 20;
        let path = format!("{}/large.bin", env.source);
        fs::File::create(&path).unwrap().set_len(size).unwrap();
        mover.note_download_event(&json!({"id": "large", "path": path, "state": "finished", "dangerous": false, "offTheRecord": false,
                                          "sha256": sha256(&vec![0u8; size as usize]), "received": size}));
        let (hashing, reached_hash) = mpsc::channel();
        let (resume, paused) = mpsc::channel();
        let paused = std::sync::Mutex::new(paused);
        hook(&mover, move |step, _| {
            if step == "hashing" {
                hashing.send(std::thread::current().id()).unwrap();
                paused
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
            }
        });
        let folder = "~/Downloads/Large";
        let work = mover.start_move(path.clone(), folder.into()).unwrap();
        let worker = reached_hash.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_ne!(worker, std::thread::current().id());
        let journal_key = "ArdiaDownloadMoveJournal";
        let journal = env.defaults.get(journal_key).expect("active move is journaled");
        assert_eq!(journal["large"]["source"], path);
        let duplicate = mover
            .start_move(path, folder.into())
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(reason(&duplicate), "busy");
        assert_eq!(env.defaults.get(journal_key).as_ref(), Some(&journal));
        let event = mover.note_download_event(
            &json!({"id": "meanwhile", "path": "/elsewhere", "state": "downloading"}),
        );
        assert_eq!(event["path"], "/elsewhere");

        let other = env.finished(&mover, "other.bin", "another download", none.clone());
        let other_id = env.last_id();
        let landed = env.at("Downloads/Large/other.bin");
        let (checkpoints, observed) = mpsc::channel();
        let defaults = env.defaults.clone();
        hook(&mover, move |step, _| {
            if matches!(step, "hashing" | "placed") {
                checkpoints.send(defaults.get(journal_key).unwrap()).unwrap();
            }
        });
        let other_move = mover.start_move(other.clone(), folder.into()).unwrap();
        let inserted = observed.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(inserted["large"], journal["large"]);
        assert_eq!(inserted[&other_id]["source"], other);
        assert!(inserted[&other_id].get("placed").is_none());
        let placed = observed.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(placed["large"], journal["large"]);
        assert_eq!(placed[&other_id]["source"], other);
        assert_eq!(placed[&other_id]["placed"], landed);
        assert_eq!(reason(&other_move.join().unwrap()), "ok");
        assert_eq!(env.defaults.get(journal_key), Some(journal));
        unhook(&mover);

        resume.send(()).unwrap();
        let result = work.join().unwrap();
        assert_eq!(reason(&result), "ok");
        assert!(env.defaults.get(journal_key).is_none());
        assert_eq!(
            fs::metadata(env.at("Downloads/Large/large.bin"))
                .unwrap()
                .size(),
            size
        );
    }
    fs::remove_dir_all(&env.home).unwrap();
}
