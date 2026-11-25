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
    use objc2::{MainThreadMarker, Message};
    use objc2::{class, msg_send};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use ria_host_mac::core::bindings::{Engine, Tab, Window};
    use ria_host_mac::diagnostics::ardia_field_timing;
    use ria_host_mac::launch::host_launch::{self, LaunchEvent};
    use ria_host_mac::launch::launch_config::LaunchConfig;
    use ria_host_mac::launch::strip_layer::{Button, Row, button_center};
    use ria_host_mac::launch::system_signals;
    use ria_host_mac::launch::window_host::WindowHost;
    use ria_host_mac::tabs::ardia_web_view::ArdiaWebView;
    use ria_host_mac::tabs::chromium_tab_view::ChromiumTabView;
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
        let out = launch(root, &listener, 1, "", Duration::from_millis(300));
        assert!(out.contains("engineDidStart"));
        assert!(
            out.lines()
                .filter(|line| line.starts_with("windowOpened "))
                .eq(["windowOpened windows=1 tabs=1"])
        );
        assert!(out.contains("tabTitle Ria proof page"));
        assert!(out.contains("engineWillShutDown"));
        assert!(
            ["stripTabs 3", "stripSelected", "stripPinned", "stripClosed"]
                .iter()
                .all(|line| out.lines().any(|out| out == *line)),
            "the Rune strip did not show, select, pin and close tabs"
        );
        assert!(
            out.lines().any(|line| line == "focusAfterSwitch true"),
            "a press that selected a tab did not move focus to its page"
        );
        assert!(
            out.lines().any(|line| line == "pageFocused true"),
            "the launch page did not take focus after its load"
        );
        assert!(
            out.lines()
                .any(|line| line == "liveViews 1 paintViews 1 detached false"),
            "the app did not build a live page view for the launch tab"
        );
        assert!(
            out.lines().any(|line| line == "attachLoading slow true"),
            "the slow-load launch tab was not loading at attach"
        );
        assert!(
            out.lines().any(|line| line == "firstPaintHeld true"),
            "the launch page was not held for first paint at attach"
        );
        let reveal_ms = revealed_after_ms(&out, "slow");
        assert!(
            reveal_ms < 1500,
            "the slow-load launch page waited {reveal_ms} ms: the load-finished reveal did not run before the 4 s timeout"
        );
        assert!(
            out.lines().any(|line| line == "firstPaintOpacity 1"),
            "the launch page is not opaque after the reveal"
        );
        assert!(
            out.lines().any(|line| line == "fieldTiming on")
                && out.lines().any(|line| line == "fieldTiming off"),
            "the app's field-timing switch did not reach set_enabled"
        );
        assert!(
            out.contains("downloadMoved ok"),
            "the app's system-signals host did not move the download"
        );
        // The saved layout only restores under the appearance it was saved in.
        let dark = Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
            .is_ok_and(|out| out.stdout.starts_with(b"Dark"));
        // Second launch on the same profile with a saved window layout: the strip layer must still be installed.
        fs::write(
            data.join("session.json"),
            format!(
                r#"{{"version":11,"focusedWindowId":"w","windows":[{{"id":"w","profile":"","tabIds":["fixture"],"frame":[100,100,1000,700]}}],"launchTab":{{"id":"fixture","profile":"","url":"{url}"}},"launchBackdrop":{{"dark":{dark}}}}}"#
            ),
        )
        .unwrap();
        fs::write(
            data.join("launch-layout.json"),
            r#"{"version":5,"windows":{"w":{"state":"{}","page":[8,44,8,8],"card":[8,44,8,8],"radius":10}}}"#,
        )
        .unwrap();
        let again = launch(root, &listener, 2, "launchTab=on", Duration::ZERO);
        assert!(
            again.lines().any(|line| line == "attachLoading fast false"),
            "the fast-case tab was still loading when it was attached"
        );
        assert!(
            again.lines().any(|line| line == "firstPaintHeldFast false"),
            "a tab that finished loading was held for first paint"
        );
        assert!(
            again
                .lines()
                .any(|line| line == "firstPaintOpacityAtAttachFast 1"),
            "the page was not opaque at attach"
        );
        let reveal_ms = revealed_after_ms(&again, "fast");
        assert!(
            reveal_ms < 1500,
            "the fast-load launch page waited {reveal_ms} ms for its reveal"
        );
        assert!(
            again.contains("windowFrame 1000x700"),
            "the saved layout was not restored"
        );
        assert!(
            again.lines().any(|line| line == "stripClosed"),
            "the strip layer is missing on a restored layout"
        );
        assert!(again.contains("engineWillShutDown"));
        assert!(!has_crash_report(root), "a crash report exists");
        println!(
            "launch live: engine exit 0; one window; one tab; fixture title; Rune strip shows 3 tabs and real clicks select, pin and close them; restored-layout relaunch on the same profile keeps the strip; download moved by the app's system-signals host; shutdown; no crash report"
        );
    }

    fn revealed_after_ms(out: &str, case: &str) -> u64 {
        let prefix = format!("firstPaintRevealMs {case} ");
        out.lines()
            .find_map(|line| line.strip_prefix(prefix.as_str()))
            .and_then(|ms| ms.parse().ok())
            .expect("the launch page was never revealed")
    }

    /// Run the app once on `data` and return what it printed.
    fn launch(
        root: &Path,
        listener: &TcpListener,
        launch: u32,
        switches: &str,
        delay: Duration,
    ) -> String {
        let bin = root.join("Ria.app/Contents/MacOS/ria-app");
        let (home, data) = (root.join("home"), root.join("profile"));
        let stdout = root.join(format!("stdout-{launch}"));
        let stderr = root.join(format!("stderr-{launch}"));
        let mut child = Command::new(bin)
            .arg("--live-child")
            .env("HOME", &home)
            .env("CFFIXED_USER_HOME", &home)
            .env("RIA_DATA_DIR", &data)
            .env("RIA_BACKGROUND", "1")
            .envs(delay.is_zero().then_some(("RIA_LIVE_FAST", "1")))
            .env_remove("RIA_INSTANCE")
            .env("RIA_SWITCHES", switches)
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
            serve_fixture(listener, delay);
            std::thread::sleep(Duration::from_millis(50));
        };
        let out = fs::read(&stdout).unwrap();
        let out = String::from_utf8_lossy(&out).into_owned();
        eprint!("{}", String::from_utf8_lossy(&fs::read(&stderr).unwrap()));
        print!("{out}");
        let status = status.expect("Ria did not exit within 60 seconds");
        assert!(status.success(), "engine exit: {status}");
        out
    }

    fn serve_fixture(listener: &TcpListener, delay: Duration) {
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == ErrorKind::WouldBlock => return,
            Err(error) => panic!("accept fixture connection: {error}"),
        };
        stream.set_nonblocking(false).unwrap();
        std::thread::sleep(delay);
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

    /// Prints the time from the first-paint hold to the reveal of the launch page, and the opacity after it.
    fn reveal_poll(
        view: Retained<ArdiaWebView>,
        held: Instant,
        case: &'static str,
        revealed: Rc<Cell<bool>>,
        done: Rc<dyn Fn()>,
    ) {
        if view.page_opacity() == Some(1.0) {
            if case == "slow" {
                println!("firstPaintRevealMs {case} {}", held.elapsed().as_millis());
            }
            println!("firstPaintOpacity 1");
            if case == "slow" {
                println!("pageFocused {}", view.has_page_focus());
            }
            revealed.set(true);
            done();
        } else if held.elapsed() < Duration::from_secs(10) {
            host_launch::after(Duration::from_millis(10), move || {
                reveal_poll(view, held, case, revealed, done)
            });
        }
    }

    /// The launch page is revealed, so its tab finished loading: attach the tab to a new view the way `page_for` does.
    fn attach_after_load(host: &WindowHost) {
        let core = host.main().expect("browser window");
        let profile = core.active_profile().expect("active profile");
        let tab = core
            .tabs_for_profile(&profile)
            .firstObject()
            .expect("launch tab");
        assert!(!tab.loading(), "the launch tab is still loading");
        let mtm = MainThreadMarker::new().expect("main thread");
        let page = ChromiumTabView::make_page(mtm, host.hooks());
        let attached = Instant::now();
        let _ = page.attach_tab(
            tab.retain(),
            "fastproof".into(),
            host.chrome().tab_registry(),
            true,
        );
        println!("attachLoading fast {}", tab.loading());
        println!("firstPaintHeldFast {}", page.page_opacity() == Some(0.0));
        println!(
            "firstPaintOpacityAtAttachFast {}",
            page.page_opacity().unwrap_or(-1.0)
        );
        println!("firstPaintRevealMs fast {}", attached.elapsed().as_millis());
    }

    fn all_set(flags: &[&Cell<bool>]) -> bool {
        flags.iter().all(|flag| flag.get())
    }

    /// Drives the Rune strip with real clicks: three tabs show, a press selects one, pins one and closes one.
    struct Drive {
        host: Rc<WindowHost>,
        stage: Cell<u32>,
        pinned: Rc<Cell<bool>>,
        done: Rc<dyn Fn()>,
    }

    impl Drive {
        /// Click `at` until the strip leaves `stage`. A press that the strip handled changes the rows at once.
        fn press(self: &Rc<Self>, at: (f64, f64), stage: u32, tries: u32) {
            if self.stage.get() != stage || tries == 8 {
                return;
            }
            if let Some(window) = self.host.main() {
                let action = format!("click:{},{}", at.0, at.1);
                self.host
                    .chrome()
                    .dev_action(&window.window(), &action, true);
            }
            let this = self.clone();
            host_launch::after(Duration::from_millis(1500), move || {
                this.press(at, stage, tries + 1)
            });
        }

        fn advance(self: &Rc<Self>, stage: u32, at: Option<(f64, f64)>) {
            self.stage.set(stage);
            if let Some(at) = at {
                self.press(at, stage, 0);
            }
        }

        /// Print whether a visible page holds the window's focus half a second after the press selected a tab, then
        /// go on to `next`. The next press would give the focus to the strip again, so it waits for the check.
        fn focus_after_switch(self: &Rc<Self>, next: u32, at: Option<(f64, f64)>) {
            self.stage.set(9);
            let this = self.clone();
            host_launch::after(Duration::from_millis(500), move || {
                let focused = this
                    .host
                    .hooks()
                    .live()
                    .all()
                    .iter()
                    .any(|view| view.visible() && view.has_page_focus());
                println!("focusAfterSwitch {focused}");
                this.advance(next, at);
            });
        }

        fn shot(&self, name: &str) {
            let (Some(dir), Some(window)) = (std::env::var_os("RIA_LIVE_SHOT"), self.host.main())
            else {
                return;
            };
            let id = window.window().windowNumber().to_string();
            let _ = Command::new("screencapture")
                .args(["-x", "-o", "-l", &id])
                .arg(Path::new(&dir).join(format!("{name}.png")))
                .status();
        }

        /// Let the strip draw the new rows, take the picture when `RIA_LIVE_SHOT` names a folder, then go to `next`.
        fn shot_then(self: &Rc<Self>, name: &'static str, next: u32, at: Option<(f64, f64)>) {
            self.stage.set(9);
            let this = self.clone();
            host_launch::after(Duration::from_millis(800), move || {
                this.shot(&format!("{}-{name}", std::process::id()));
                this.advance(next, at);
                if next == 5 {
                    (this.done)();
                }
            });
        }

        /// Open two more tabs after the strip installed.
        fn start(&self) {
            self.stage.set(1);
            if let Some(window) = self.host.main() {
                let profile = window.active_profile().expect("window profile");
                for _ in 0..2 {
                    let tab = window.open_tab(&NSString::from_str("about:blank"), &profile, true);
                    window.activate_tab(&tab);
                }
            }
        }

        fn rows(self: &Rc<Self>, rows: &[Row]) {
            let keys: Vec<_> = rows
                .iter()
                .map(|row| {
                    format!(
                        "{}{}{}",
                        row.key,
                        if row.active { "*" } else { "" },
                        if row.pinned { "^" } else { "" }
                    )
                })
                .collect();
            println!("stripRows {keys:?}");
            match self.stage.get() {
                1 if rows.len() == 3 => {
                    println!("stripTabs 3");
                    self.shot_then("three-tabs", 2, Some(button_center(2, Button::Select)));
                }
                2 if rows.get(2).is_some_and(|row| row.active) => {
                    println!("stripSelected");
                    self.focus_after_switch(3, Some(button_center(1, Button::Pin)));
                }
                3 if rows.first().is_some_and(|row| row.pinned) => {
                    println!("stripPinned");
                    self.pinned.set(true);
                    self.shot_then("pinned", 4, Some(button_center(2, Button::Close)));
                }
                4 if rows.len() == 2 => {
                    println!("stripClosed");
                    self.shot_then("closed", 5, None);
                }
                _ => {}
            }
        }
    }

    pub fn child() -> ExitCode {
        let matched = Rc::new(Cell::new(false));
        let shutdown = Rc::new(Cell::new(false));
        let moving = Move::new();
        let pinned = Rc::new(Cell::new(false));
        let revealed = Rc::new(Cell::new(false));
        let fast = std::env::var_os("RIA_LIVE_FAST").is_some();
        let case = if fast { "fast" } else { "slow" };
        let ready_host: Rc<RefCell<Option<Rc<WindowHost>>>> = Rc::default();
        // Quit when the fixture title, the download move, the pin and the first-paint reveal are done.
        let quit_when_done: Rc<dyn Fn()> = {
            let (matched, moving, pinned, revealed) = (
                matched.clone(),
                moving.clone(),
                pinned.clone(),
                revealed.clone(),
            );
            let ready_host = ready_host.clone();
            let proven = Cell::new(false);
            Rc::new(move || {
                if all_set(&[&matched, &moving.done, &pinned, &revealed]) {
                    if fast && !proven.replace(true) {
                        let host = ready_host.borrow().clone();
                        attach_after_load(&host.expect("the window host was ready"));
                    }
                    host_launch::later(host_launch::quit);
                }
            })
        };
        let drive: Rc<RefCell<Option<Rc<Drive>>>> = Rc::default();
        let check_drive = drive.clone();
        let check_title = matched.clone();
        let check_pin = pinned.clone();
        let check_revealed = revealed.clone();
        let check_ready = ready_host.clone();
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
                if let Some(window) = host.main() {
                    let size = window.window().frame().size;
                    println!("windowFrame {}x{}", size.width, size.height);
                }
                let window = host.main().expect("browser window").window();
                let facts = host.hooks().paint_views(&window);
                println!(
                    "liveViews {} paintViews {} detached {}",
                    host.hooks().live().all().len(),
                    facts.len(),
                    facts.iter().any(|view| view.page_detached)
                );
                *check_ready.borrow_mut() = Some(host.clone());
                let view = host.hooks().live().all().remove(0);
                println!(
                    "attachLoading {case} {}",
                    host.launch_attach_loading()
                        .expect("the launch tab attached")
                );
                println!("firstPaintHeld {}", view.page_opacity() == Some(0.0));
                reveal_poll(
                    view,
                    Instant::now(),
                    case,
                    check_revealed.clone(),
                    quit_when_done.clone(),
                );
                let signals = system_signals::current().expect("the app has a system-signals host");
                signals.call(
                    "ACFieldTiming.setEnabled",
                    &json!([true]),
                    Box::new(|reply| assert_eq!(reply, Ok(Value::Null))),
                );
                println!(
                    "fieldTiming {}",
                    if ardia_field_timing::enabled() {
                        "on"
                    } else {
                        "off"
                    }
                );
                signals.call(
                    "ACFieldTiming.setEnabled",
                    &json!([false]),
                    Box::new(|reply| assert_eq!(reply, Ok(Value::Null))),
                );
                println!(
                    "fieldTiming {}",
                    if ardia_field_timing::enabled() {
                        "on"
                    } else {
                        "off"
                    }
                );
                let drive = Rc::new(Drive {
                    host,
                    stage: Cell::new(0),
                    pinned: pinned.clone(),
                    done: quit_when_done.clone(),
                });
                *check_drive.borrow_mut() = Some(drive.clone());
                host_launch::later(move || drive.start());
            }
            LaunchEvent::StripRows(rows) => {
                let drive = check_drive.borrow().clone();
                if let Some(drive) = drive {
                    drive.rows(&rows);
                }
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
        if all_set(&[&matched, &shutdown, &moving.done, &check_pin, &revealed]) {
            status
        } else {
            ExitCode::FAILURE
        }
    }
}
