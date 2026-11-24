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
    // The folder goes away when the test ends, also when an assertion fails.
    let base = tempfile::tempdir().unwrap();
    let home = fs::canonicalize(base.path())
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
    drift_and_edit(&env, &mover);
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
    parity_deviations(&env, &mover);
    recovery_and_revocation(&env, &mover);
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
        let journal = env
            .defaults
            .get(journal_key)
            .expect("active move is journaled");
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
                checkpoints
                    .send(defaults.get(journal_key).unwrap())
                    .unwrap();
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

fn drift_and_edit(env: &Env, mover: &DownloadMover) {
    for step in ["verified", "renamed"] {
        let name = format!("drift-{step}.txt");
        let path = env.finished(mover, &name, "genuine", json!({}));
        let folder = format!("Downloads/Drift-{step}");
        fs::create_dir(env.at(&folder)).unwrap();
        let (from, to) = (env.at(&folder), env.at(&format!("outside-{step}")));
        hook(mover, move |at, _| {
            if at == step {
                fs::rename(&from, &to).unwrap();
            }
        });
        let result = mover.move_download(&path, &format!("~/{folder}"));
        unhook(mover);
        assert_eq!(reason(&result), "folderNotAllowed");
        assert_eq!(text(&path), "genuine");
        assert!(!exists(&env.at(&format!("outside-{step}/{name}"))));
        assert_eq!(env.staging_dirs(), 0);
    }
    {
        let path = env.finished(mover, "edited.txt", "genuine", json!({}));
        let placed = env.at("Downloads/Bills/edited.txt");
        hook(mover, move |step, _| {
            if step == "renamed" {
                let mut file = fs::OpenOptions::new().append(true).open(&placed).unwrap();
                file.write_all(b" plus").unwrap();
            }
        });
        let result = mover.move_download(&path, "~/Downloads/Bills");
        unhook(mover);
        assert_eq!(reason(&result), "changed");
        assert_eq!(text(&path), "genuine plus");
        assert!(!exists(&env.at("Downloads/Bills/edited.txt")));
        assert_eq!(env.staging_dirs(), 0);
    }
}

/// The checks for the four approved deviations from Ardia (issues 62 to 65).
fn parity_deviations(env: &Env, mover: &DownloadMover) {
    let none = json!({});
    {
        // Issue 62: the shortest approved root decides where folders may be created.
        let path = env.finished(mover, "tax.pdf", "tax", none.clone());
        mover.set_approved_folders(&["~/Downloads/Tax/2026".into()]);
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads/Tax/2026")),
            "ok"
        );
        assert_eq!(text(&env.at("Downloads/Tax/2026/tax.pdf")), "tax");
        mover.set_approved_folders(&[]);
    }
    {
        // Issue 63: a FIFO put in place of the claimed file must not block the mover.
        let mover = Arc::new(env.mover());
        let path = env.finished(&mover, "pipe.txt", "pipe", none.clone());
        hook(&mover, |step, staging| {
            if step == "claimed" {
                let staged = format!("{staging}/pipe.txt");
                fs::remove_file(&staged).unwrap();
                let fifo = std::ffi::CString::new(staged).unwrap();
                // SAFETY: `fifo` is a valid C string.
                assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
            }
        });
        let work = mover
            .start_move(path.clone(), "~/Downloads/Pipes".into())
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !work.is_finished() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(work.is_finished(), "the mover blocked on a FIFO");
        assert_eq!(reason(&work.join().unwrap()), "changed");
        unhook(&mover);
        fs::remove_file(&path).unwrap();
    }
    {
        // Issue 65: a move into the folder that holds the file verifies it first, and does not rename it.
        let path = env.make("Downloads/same.txt", "as downloaded");
        mover.note_download_event(&env.finished_event(&path, "as downloaded"));
        let before = fs::metadata(&path).unwrap().ino();
        assert_eq!(reason(&mover.move_download(&path, "~/Downloads")), "ok");
        assert_eq!(fs::metadata(&path).unwrap().ino(), before);
        fs::write(&path, "edited later!!").unwrap();
        assert_eq!(
            reason(&mover.move_download(&path, "~/Downloads")),
            "changed"
        );
        assert_eq!(text(&path), "edited later!!");
        assert_eq!(env.staging_dirs(), 0);
    }
    #[cfg(target_os = "linux")]
    {
        // Issue 64: a staging path with no UTF-8 form refuses the move and leaves the file and no journal entry.
        use std::os::unix::ffi::OsStrExt;
        let real = std::path::PathBuf::from(std::ffi::OsStr::from_bytes(
            &[env.home.as_bytes(), b"/odd-\xff"].concat(),
        ));
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("odd.txt"), "odd").unwrap();
        let link = env.at("odd-link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let path = format!("{link}/odd.txt");
        mover.note_download_event(&env.finished_event(&path, "odd"));
        assert_eq!(reason(&mover.move_download(&path, "~/Downloads")), "failed");
        assert_eq!(text(&path), "odd");
        assert!(env.defaults.get("ArdiaDownloadMoveJournal").is_none());
        assert_eq!(fs::read_dir(&real).unwrap().count(), 1);
        fs::remove_dir_all(&real).unwrap();
        fs::remove_file(&link).unwrap();
    }
}

/// Issues 52 and 61: crash recovery after the destination rename, identity checks in recovery, and live approval.
fn recovery_and_revocation(env: &Env, mover: &DownloadMover) {
    let crash_at = |mover: &DownloadMover, name: &str, content: &str, at: &'static str| {
        let path = env.finished(mover, name, content, json!({}));
        hook(mover, move |step, _| {
            assert_ne!(step, at, "simulated crash")
        });
        assert!(
            catch_unwind(AssertUnwindSafe(
                || mover.move_download(&path, "~/Downloads/Crash52")
            ))
            .is_err()
        );
        path
    };
    {
        // Issue 52: a crash after the rename and before the path is recorded still reports the moved file.
        let path = crash_at(mover, "renamed.txt", "moved", "renamed");
        let id = env.last_id();
        let restarted = env.mover();
        let landed = env.at("Downloads/Crash52/renamed.txt");
        assert_eq!(
            restarted.note_download_event(&json!({"id": id, "path": path}))["path"],
            json!(landed)
        );
        assert_eq!(text(&landed), "moved");
        assert_eq!(env.staging_dirs(), 0);
    }
    {
        // Issue 52: a staged file that no longer matches the download stays in staging, unrecorded and unreported.
        let path = crash_at(mover, "tampered.txt", "original", "claimed");
        let id = env.last_id();
        let staged = fs::read_dir(&env.source)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".ardia-moving-")
            })
            .unwrap();
        fs::write(staged.join("tampered.txt"), "tampered").unwrap();
        let restarted = env.mover();
        assert!(!exists(&path));
        let reported = restarted.note_download_event(&json!({"id": id, "path": path}))["path"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(reported, path);
        assert_eq!(
            text(staged.join("tampered.txt").to_str().unwrap()),
            "tampered"
        );
        fs::remove_dir_all(staged).unwrap();
    }
    {
        // Issue 52: a placed file that no longer matches the download is not recorded as the move.
        let path = crash_at(mover, "swapped.txt", "genuine", "renamed");
        let id = env.last_id();
        fs::write(env.at("Downloads/Crash52/swapped.txt"), "swapped").unwrap();
        let restarted = env.mover();
        assert_eq!(
            restarted.note_download_event(&json!({"id": id, "path": path}))["path"],
            json!(path)
        );
    }
    {
        // Issue 61: revoking the approval while the file is hashed stops the move before placement.
        let path = env.finished(mover, "revoked.txt", "revoke me", json!({}));
        mover.set_approved_folders(&["~/Revoke61".into()]);
        let revoker = mover as *const DownloadMover as usize;
        hook(mover, move |step, _| {
            if step == "hashing" {
                // SAFETY: the mover outlives the synchronous move that runs this hook.
                unsafe { &*(revoker as *const DownloadMover) }.set_approved_folders(&[]);
            }
        });
        let result = mover.move_download(&path, "~/Revoke61/Sub");
        unhook(mover);
        assert_eq!(reason(&result), "folderNotAllowed");
        assert_eq!(text(&path), "revoke me");
        assert!(!exists(&env.at("Revoke61/Sub/revoked.txt")));
        assert_eq!(env.staging_dirs(), 0);
    }
}
