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
    use std::cell::Cell;
    use std::fs;
    use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::process::{Command, ExitCode};
    use std::rc::Rc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use ria_host_mac::launch::host_launch::{self, LaunchEvent};
    use ria_host_mac::launch::launch_config::LaunchConfig;

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
            format!(r#"{{"version":11,"launchTab":{{"id":"fixture","profile":"","url":"{url}"}}}}"#),
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
        assert!(!has_crash_report(root), "a crash report exists");
        println!(
            "launch live: engine exit 0; one window; one tab; fixture title; shutdown; no crash report"
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

    pub fn child() -> ExitCode {
        let matched = Rc::new(Cell::new(false));
        let shutdown = Rc::new(Cell::new(false));
        let check_title = matched.clone();
        let check_shutdown = shutdown.clone();
        let status = super::app_launch::start(Rc::new(move |event| match event {
            LaunchEvent::EngineStarted => {
                println!("engineDidStart");
                host_launch::after(Duration::from_secs(45), host_launch::quit);
            }
            LaunchEvent::WindowOpened { windows, tabs } => {
                println!("windowOpened windows={windows} tabs={tabs}");
            }
            LaunchEvent::TabTitle(title) if title == TITLE && !check_title.replace(true) => {
                println!("tabTitle {title}");
                host_launch::later(host_launch::quit);
            }
            LaunchEvent::EngineWillShutDown => {
                check_shutdown.set(true);
                println!("engineWillShutDown");
            }
            LaunchEvent::Error(error) => eprintln!("{error}"),
            _ => {}
        }));
        if matched.get() && shutdown.get() {
            status
        } else {
            ExitCode::FAILURE
        }
    }
}
