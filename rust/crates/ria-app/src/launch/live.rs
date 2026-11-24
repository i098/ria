// Ria original, no Ardia source.

#[cfg(target_os = "macos")]
#[path = "mod.rs"]
mod app_launch;

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("Check L needs macOS and the prebuilt Chromium framework");
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--live-child") {
        mac::child()
    } else {
        mac::parent();
        std::process::ExitCode::SUCCESS
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::process::{Command, ExitCode};
    use std::rc::Rc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use objc2::rc::Retained;
    use objc2::{class, msg_send};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use ria_host_mac::core::bindings::{Engine, Tab, Window};
    use ria_host_mac::launch::host_launch::{self, LaunchEvent};
    use ria_host_mac::launch::launch_config::LaunchConfig;
    use ria_host_mac::launch::system_signals;
    use ria_host_mac::launch::window_host::WindowHost;
    use serde_json::{Value, json};

    const TITLE: &str = "Ria proof page";
    const PAGE: &str = include_str!("../../../../proofs/abc-host/src/page.html");
    const PLIST: &str = include_str!("Info.plist");

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub fn parent() {
        let engine = std::env::var_os("ENGINE_DIR")
            .expect("set ENGINE_DIR to the directory with Chromium Framework.framework");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = Path::new("/tmp").join(format!("ria-app-live-{}-{stamp}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let scratch = Scratch(root);
        let root = &scratch.0;
        let contents = root.join("Ria.app/Contents");
        let bin = contents.join("MacOS/ria-app");
        let frameworks = contents.join("Frameworks");
        fs::create_dir_all(bin.parent().unwrap()).unwrap();
        fs::create_dir(&frameworks).unwrap();
        // Chromium treats LSUIElement bundles as helpers and looks for resources above them.
        fs::write(contents.join("Info.plist"), PLIST).unwrap();
        fs::copy(std::env::current_exe().unwrap(), &bin).unwrap();
        run(Command::new("cp")
            .arg("-cR")
            .arg(Path::new(&engine).join("Chromium Framework.framework"))
            .arg(&frameworks));
        run(Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .arg(&bin));
        let home = root.join("home");
        fs::create_dir(&home).unwrap();
        let data = root.join("profile");
        let config = LaunchConfig::from_environment(
            [("RIA_DATA_DIR", data.to_str().unwrap())],
            &home,
            root,
            false,
        )
        .unwrap();
        assert!(config.test_instance);
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        fs::write(
            data.join("session.json"),
            format!(
                r#"{{"version":11,"launchTab":{{"id":"fixture","profile":"","url":"{url}"}}}}"#
            ),
        )
        .unwrap();
        let stdout = root.join("stdout");
        let stderr = root.join("stderr");
        let mut child = Command::new(&bin)
            .arg("--live-child")
            .env("HOME", &home)
            .env("CFFIXED_USER_HOME", &home)
            .env("RIA_DATA_DIR", &data)
            .env("RIA_BACKGROUND", "1")
            .env_remove("RIA_INSTANCE")
            .env_remove("RIA_SWITCHES")
            .env_remove("RIA_TEST")
            .env_remove("RIA_TRACE")
            .env_remove("RIA_REMOTE_DEBUGGING_PORT")
            .stdout(fs::File::create(&stdout).unwrap())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .expect("run Ria in a new profile");
        let deadline = Instant::now() + Duration::from_secs(60);
        let status = loop {
            if let Some(status) = child.try_wait().expect("wait for Ria") {
                break Some(status);
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                child.wait().expect("reap Ria after timeout");
                break None;
            }
            serve_fixture(&listener);
            std::thread::sleep(Duration::from_millis(50));
        };
        let out = fs::read(&stdout).unwrap();
        let out = String::from_utf8_lossy(&out);
        eprint!("{}", String::from_utf8_lossy(&fs::read(&stderr).unwrap()));
        print!("{out}");
        let status = status.expect("Ria did not exit within 60 seconds");
        assert!(status.success(), "engine exit: {status}");
        assert!(out.contains("engineDidStart"));
        assert!(
            out.lines()
                .filter(|line| line.starts_with("windowOpened "))
                .eq(["windowOpened windows=1 tabs=1"])
        );
        assert!(out.contains("tabTitle Ria proof page"));
        assert!(out.contains("engineWillShutDown"));
        assert!(
            out.contains("stripPinned"),
            "a click on the Rune pin button did not pin the tab"
        );
        assert!(
            out.contains("downloadMoved ok"),
            "the app's system-signals host did not move the download"
        );
        assert!(!has_crash_report(root), "a crash report exists");
        println!(
            "launch live: engine exit 0; one window; one tab; fixture title; Rune pin click pins the tab; download moved by the app's system-signals host; shutdown; no crash report"
        );
    }

    fn serve_fixture(listener: &TcpListener) {
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == ErrorKind::WouldBlock => return,
            Err(error) => panic!("accept fixture connection: {error}"),
        };
        stream.set_nonblocking(false).unwrap();
        let timeout = Some(Duration::from_millis(100));
        stream.set_read_timeout(timeout).unwrap();
        stream.set_write_timeout(timeout).unwrap();
        let mut request = BufReader::new((&mut stream).take(4096));
        let mut line = Vec::new();
        loop {
            line.clear();
            if !request.read_until(b'\n', &mut line).is_ok_and(|n| n > 0) {
                return;
            }
            if line == b"\r\n" {
                break;
            }
        }
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{PAGE}",
            PAGE.len()
        );
    }

    fn run(command: &mut Command) {
        assert!(command.status().unwrap().success(), "failed: {command:?}");
    }

    fn has_crash_report(path: &Path) -> bool {
        fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .any(|entry| {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    return has_crash_report(&entry.path());
                }
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| matches!(ext, "crash" | "ips" | "dmp"))
            })
    }

    /// The app's own system-signals host moves a real finished download into an approved folder.
    struct Move {
        pid: u32,
        content: String,
        folder: String,
        target: PathBuf,
        source: PathBuf,
        window: RefCell<Option<(Retained<Window>, Retained<Tab>)>>,
        started: Cell<bool>,
        done: Cell<bool>,
    }

    impl Move {
        fn new() -> Rc<Self> {
            let pid = std::process::id();
            let source = std::env::temp_dir().join(format!("ria-app-dl-{pid}/payload-{pid}.bin"));
            let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"));
            Rc::new(Self {
                pid,
                content: format!("ria app download {pid}\n"),
                folder: format!("~/Downloads/ria-app-{pid}"),
                target: home.join(format!("Downloads/ria-app-{pid}")),
                source,
                window: RefCell::default(),
                started: Cell::new(false),
                done: Cell::new(false),
            })
        }

        /// After `engineDidStart`: approve the folder through the app's host.
        fn approve(&self) {
            let signals = system_signals::current().expect("the app has a system-signals host");
            fs::create_dir_all(self.source.parent().unwrap()).unwrap();
            fs::write(&self.source, &self.content).unwrap();
            signals.call(
                "setApprovedDownloadFolders",
                &json!([[self.folder]]),
                Box::new(|reply| assert_eq!(reply, Ok(Value::Null))),
            );
        }

        /// After the first window: download the file in a hidden window.
        fn download(&self) {
            let engine = Engine::shared().expect("engine");
            let profile = engine.default_profile();
            let rect = NSRect::new(NSPoint::new(-3000.0, -3000.0), NSSize::new(400.0, 300.0));
            let window: Retained<Window> = unsafe {
                Window::init_with_content_rect(msg_send![class!(ArdiaCoreWindow), alloc], rect)
            };
            window.set_internal(true);
            window.prepare_profile(&profile);
            let url = format!("file://{}", self.source.display());
            let tab = window.open_tab(&NSString::from_str(&url), &profile, false);
            *self.window.borrow_mut() = Some((window, tab));
        }

        /// On each event: when the download has finished, move it through the app's host.
        fn emitted(self: &Rc<Self>, topic: &str, data: &Value, finished: Rc<dyn Fn()>) {
            let finished_here = topic == "downloads.changed"
                && data["state"] == "finished"
                && data["path"]
                    .as_str()
                    .is_some_and(|p| p.contains(&format!("payload-{}", self.pid)));
            if !finished_here || self.started.replace(true) {
                return;
            }
            let path = data["path"].as_str().unwrap().to_owned();
            let this = self.clone();
            let signals = system_signals::current().expect("the app has a system-signals host");
            signals.call(
                "moveCompletedDownload",
                &json!([path, self.folder]),
                Box::new(move |reply| {
                    let moved = reply.as_ref().ok().and_then(|r| r["path"].as_str());
                    let placed = moved.and_then(|p| fs::read_to_string(p).ok());
                    let target = fs::canonicalize(&this.target).ok();
                    let ok = moved
                        .and_then(|p| fs::canonicalize(p).ok())
                        .zip(target)
                        .is_some_and(|(moved, target)| moved.starts_with(target))
                        && placed.as_deref() == Some(this.content.as_str());
                    println!(
                        "downloadMoved {} {reply:?}",
                        if ok { "ok" } else { "FAILED" }
                    );
                    if let Some((window, tab)) = this.window.take() {
                        tab.close();
                        window.close();
                    }
                    let _ = fs::remove_dir_all(&this.target);
                    let _ = fs::remove_dir_all(this.source.parent().unwrap());
                    this.done.set(ok);
                    finished();
                }),
            );
        }
    }

    /// Click the Rune pin button through the window until the strip pins the launch tab, then run `done`.
    /// The button is at x 202, y 62: the strip starts 40 below the top, its row pads 12, the tab is 140 wide, the gap is 8, the pin is 84 wide and 28 high.
    fn click_pin(host: Rc<WindowHost>, pinned: Rc<Cell<bool>>, done: Rc<dyn Fn()>, tries: u32) {
        let Some(window) = host.main() else { return };
        let profile = window.active_profile().expect("window profile");
        let tab = window.tabs_for_profile(&profile).objectAtIndex(0);
        if host.strips().borrow().is_pinned(&tab) {
            println!("stripPinned");
            pinned.set(true);
            return done();
        }
        if tries == 160 {
            return println!("pin click never pinned the tab");
        }
        if tries % 4 == 0 {
            host.chrome()
                .dev_action(&window.window(), "click:202,62", true);
        }
        host_launch::after(Duration::from_millis(250), move || {
            click_pin(host, pinned, done, tries + 1)
        });
    }

    pub fn child() -> ExitCode {
        let matched = Rc::new(Cell::new(false));
        let shutdown = Rc::new(Cell::new(false));
        let moving = Move::new();
        let pinned = Rc::new(Cell::new(false));
        // Quit when both the fixture title and the download move are done.
        let quit_when_done: Rc<dyn Fn()> = {
            let (matched, moving, pinned) = (matched.clone(), moving.clone(), pinned.clone());
            Rc::new(move || {
                if matched.get() && moving.done.get() && pinned.get() {
                    host_launch::later(host_launch::quit);
                }
            })
        };
        let check_title = matched.clone();
        let check_pin = pinned.clone();
        let check_shutdown = shutdown.clone();
        let check_moving = moving.clone();
        let status = super::app_launch::start(Rc::new(move |event| match event {
            LaunchEvent::EngineStarted => {
                println!("engineDidStart");
                check_moving.approve();
                host_launch::after(Duration::from_secs(45), host_launch::quit);
            }
            LaunchEvent::WindowOpened { windows, tabs } => {
                println!("windowOpened windows={windows} tabs={tabs}");
                let moving = check_moving.clone();
                host_launch::later(move || moving.download());
            }
            LaunchEvent::WindowHostReady(host) => {
                click_pin(host, pinned.clone(), quit_when_done.clone(), 0);
            }
            LaunchEvent::Emit { topic, data } => {
                check_moving.emitted(&topic, &data, quit_when_done.clone());
            }
            LaunchEvent::TabTitle(title) if title == TITLE && !check_title.replace(true) => {
                println!("tabTitle {title}");
                quit_when_done();
            }
            LaunchEvent::EngineWillShutDown => {
                check_shutdown.set(true);
                println!("engineWillShutDown");
            }
            LaunchEvent::Error(error) => eprintln!("{error}"),
            _ => {}
        }));
        if matched.get() && shutdown.get() && moving.done.get() && check_pin.get() {
            status
        } else {
            ExitCode::FAILURE
        }
    }
}
