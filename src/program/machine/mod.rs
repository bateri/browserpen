//! What the machine the program runs on says to the page: the TCP ports its person's processes
//! listen on (with the name of the process), and whether one port answers. It knows nothing of the
//! page. `macos.rs` reads them with libproc, `linux.rs` from `/proc`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::time::Duration;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

/// How long a connection to a port is waited for: the loopback answers at once or not at all.
const PROBE: Duration = Duration::from_millis(250);

/// The most listeners the page is told of: a machine with more is a fork bomb, not a dev server.
pub const MOST: usize = 64;

/// A TCP port a process of the person listens on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listener {
    pub port: u16,
    /// The process as `ps` shows it: its `argv[0]`'s last component (never the path), or, for an
    /// interpreter (`node`, `python`), the script it runs; a rewritten title is kept whole.
    pub name: String,
}

/// Where the ports and the answers come from: the machine, or a stand-in in a test.
pub trait Machine: Send + Sync {
    /// The ports the person's processes listen on, ascending, each once.
    fn listeners(&self) -> Vec<Listener>;
    /// Whether something answers on `port` of this machine's loopback, whoever's it is.
    fn listening(&self, port: u16) -> bool;
}

/// The machine the program runs on.
pub struct System;

impl Machine for System {
    fn listeners(&self) -> Vec<Listener> {
        let mut found = scan();
        found.sort_by(|a, b| (a.port, &a.name).cmp(&(b.port, &b.name)));
        found.dedup_by(|later, kept| later.port == kept.port);
        found.truncate(MOST);
        found
    }

    fn listening(&self, port: u16) -> bool {
        // A server may listen on the IPv4 loopback or the IPv6 one only (`localhost` is either).
        [
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        ]
        .into_iter()
        .any(|ip| TcpStream::connect_timeout(&SocketAddr::new(ip, port), PROBE).is_ok())
    }
}

#[cfg(target_os = "macos")]
fn scan() -> Vec<Listener> {
    macos::listeners()
}

#[cfg(target_os = "linux")]
fn scan() -> Vec<Listener> {
    linux::listeners()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn scan() -> Vec<Listener> {
    Vec::new()
}

/// The name a process is shown by, from its arguments: the last component of `argv[0]`, never the
/// path, relative or not (a title that was rewritten, and is no path, whole); where that is an
/// interpreter, the script it was given (or the module, `python -m http.server`), which is what the
/// person started.
#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn display_name(args: &[String], kernel_name: Option<&str>) -> Option<String> {
    // The interpreters whose script is what the person started, by the name they run under in any
    // case and with a version after it (`python3.12`, `ruby3.3`, the framework's `Python`).
    const INTERPRETERS: [&str; 8] = [
        "node", "bun", "deno", "python", "ruby", "php", "java", "tsx",
    ];
    // The options of an interpreter that take the word after them, which is not the script.
    const WITH_A_VALUE: [&str; 11] = [
        "-r",
        "--require",
        "--import",
        "--loader",
        "--experimental-loader",
        "--env-file",
        "-X",
        "-W",
        "-I",
        "-d",
        "-cp",
    ];
    let is_interpreter = |name: &str| {
        let lower = name.to_ascii_lowercase();
        let base = lower.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
        INTERPRETERS.contains(&base)
    };
    let tail = |word: &str| -> String {
        word.rsplit('/')
            .find(|part| !part.is_empty())
            .unwrap_or(word)
            .to_owned()
    };
    // A path is shown by its last part, spaces in the path or not (`Application Support`); a word that
    // is no path is a title the process gave itself, and stays whole.
    let last = |word: &str| -> String {
        if word.starts_with('/') || (word.contains('/') && !word.contains(' ')) {
            tail(word)
        } else {
            word.to_owned()
        }
    };
    let script_of = |args: &[String]| -> Option<String> {
        let mut rest = args.iter().skip(1);
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "" => {}
                "-m" => return rest.next().cloned(),
                // Code on the command line, not a script.
                "-c" | "-e" | "--eval" | "-p" | "--print" => return None,
                flag if WITH_A_VALUE.contains(&flag) => {
                    rest.next();
                }
                flag if flag.starts_with('-') => {}
                script => return Some(tail(script)),
            }
        }
        None
    };
    let first = args
        .first()
        .filter(|arg| !arg.is_empty())
        .map(|arg| last(arg));
    match first {
        Some(name) if is_interpreter(&name) => Some(script_of(args).unwrap_or(name)),
        Some(name) => Some(name),
        None => kernel_name
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
    }
}

/// Whether the executable at `path` is one the person did not start in a terminal: a program inside
/// an application bundle (`Foo.app/Contents/MacOS/foo`, what they opened from the Dock, and its
/// helpers), or one of the system's own (`/System`, `/usr/libexec`, `/usr/sbin`, `/sbin`: Control
/// Center on :5000 and :7000, `rapportd`). The page lists what a developer started, so these are
/// left out, with four exceptions:
///
/// - the innermost bundle decides, and a bundle inside a framework is a program's, not an app's:
///   Python runs as `Python.framework/…/Resources/Python.app/Contents/MacOS/Python`, Homebrew's,
///   Apple's and Xcode's (`Xcode.app/…/Python3.framework/…/Python.app/…`) alike, and a dev server on
///   it is listed;
/// - what holds the ports of a container (`docker compose up`): OrbStack's helper and Docker
///   Desktop's `com.docker.backend` and `com.docker.vpnkit` are in an app, and the developer's
///   server in the container is behind them;
/// - a bundle under `node_modules` is the developer's own (Electron's `Electron.app`).
///
/// A name that only ends in `.app` and is the executable itself, or a directory that only contains
/// it (`my.app-server`), is no bundle.
#[cfg(any(target_os = "macos", test))]
fn started_by_the_system(path: &str) -> bool {
    const SYSTEM: [&str; 4] = ["/System/", "/usr/libexec/", "/usr/sbin/", "/sbin/"];
    // What holds a container's ports, by the start of the executable's name, in any case.
    const PORT_FORWARDERS: [&str; 4] = [
        "orbstack helper",
        "com.docker.backend",
        "com.docker.vpnkit",
        "vpnkit",
    ];
    if SYSTEM.iter().any(|dir| path.starts_with(dir)) {
        return true;
    }
    let mut parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    // The executable itself.
    let name = parts.pop().unwrap_or("").to_ascii_lowercase();
    if PORT_FORWARDERS.iter().any(|start| name.starts_with(start))
        || parts.contains(&"node_modules")
    {
        return false;
    }
    let ends_in = |part: &str, suffix: &str| {
        part.len() > suffix.len() && part.to_ascii_lowercase().ends_with(suffix)
    };
    match parts.iter().rposition(|part| ends_in(part, ".app")) {
        Some(app) => !parts[..app].iter().any(|part| ends_in(part, ".framework")),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    fn words(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn a_process_is_shown_by_its_program_or_for_an_interpreter_by_its_script() {
        let name = |args: &[&str]| display_name(&words(args), Some("kernel"));
        assert_eq!(name(&["/usr/bin/ruby"]).as_deref(), Some("ruby"));
        assert_eq!(
            name(&["node", "/app/node_modules/.bin/vite", "--port", "5173"]).as_deref(),
            Some("vite")
        );
        assert_eq!(
            name(&["/opt/bin/node", "--inspect", "server.js"]).as_deref(),
            Some("server.js")
        );
        assert_eq!(name(&["node"]).as_deref(), Some("node"));
        assert_eq!(
            name(&["next-server (v16.0.10)"]).as_deref(),
            Some("next-server (v16.0.10)")
        );
        assert_eq!(display_name(&[], Some("kernel")).as_deref(), Some("kernel"));
        assert_eq!(display_name(&[], None), None);
    }

    #[test]
    fn a_process_is_never_shown_by_its_path_whatever_the_path_holds() {
        let name = |args: &[&str]| display_name(&words(args), None);
        assert_eq!(
            name(&["/Applications/OrbStack.app/Contents/Frameworks/OrbStack Helper.app/Contents/MacOS/OrbStack Helper"])
                .as_deref(),
            Some("OrbStack Helper")
        );
        assert_eq!(
            name(&["/Users/me/Library/Application Support/Figma/FigmaAgent.app/Contents/MacOS/FigmaAgent"])
                .as_deref(),
            Some("FigmaAgent")
        );
        assert_eq!(
            name(&[
                "/Users/me/My Projects/bin/node",
                "/Users/me/My Projects/app/server.js"
            ])
            .as_deref(),
            Some("server.js")
        );
    }

    #[test]
    fn a_program_of_an_app_or_of_the_system_is_told_from_one_started_in_a_terminal() {
        for path in [
            "/System/Library/CoreServices/ControlCenter.app/Contents/MacOS/ControlCenter",
            "/Applications/Figma.app/Contents/MacOS/Figma",
            "/Users/me/Library/Application Support/Figma/FigmaAgent.app/Contents/MacOS/FigmaAgent",
            "/Applications/Postgres.APP/Contents/Versions/17/bin/postgres",
            "/usr/libexec/rapportd",
            "/usr/sbin/cupsd",
            "/sbin/launchd",
        ] {
            assert!(started_by_the_system(path), "{path}");
        }
        for path in [
            "/opt/homebrew/bin/node",
            "/Users/me/.cargo/bin/cargo",
            "/Users/me/Projects/my.app-server/target/debug/serve",
            "/Users/me/bin/Thing.app",
            "/Users/me/.app/serve",
            "/usr/bin/python3",
            "node",
            "",
        ] {
            assert!(!started_by_the_system(path), "{path}");
        }
    }

    #[test]
    fn python_in_a_framework_is_a_program_even_inside_xcode_and_the_innermost_bundle_decides() {
        for path in [
            "/opt/homebrew/Cellar/python@3.13/3.13.1/Frameworks/Python.framework/Versions/3.13/Resources/Python.app/Contents/MacOS/Python",
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
            "/Applications/Xcode.app/Contents/Developer/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
        ] {
            assert!(!started_by_the_system(path), "{path}");
        }
        // The same shape in an app of the system's own is the system's.
        assert!(started_by_the_system(
            "/System/Library/Frameworks/Python.framework/Versions/2.7/Resources/Python.app/Contents/MacOS/Python"
        ));
    }

    #[test]
    fn what_holds_the_ports_of_a_container_and_an_electron_of_the_developers_are_listed() {
        for path in [
            "/Applications/OrbStack.app/Contents/Frameworks/OrbStack Helper.app/Contents/MacOS/OrbStack Helper",
            "/Applications/Docker.app/Contents/MacOS/com.docker.backend",
            "/Applications/Docker.app/Contents/Resources/bin/com.docker.vpnkit",
            "/Applications/Docker.app/Contents/Resources/vpnkit",
            "/Users/me/app/node_modules/electron/dist/Electron.app/Contents/MacOS/Electron",
        ] {
            assert!(!started_by_the_system(path), "{path}");
        }
        // What else is in those apps is still an app's.
        assert!(started_by_the_system(
            "/Applications/OrbStack.app/Contents/MacOS/OrbStack"
        ));
        assert!(started_by_the_system(
            "/Applications/Docker.app/Contents/MacOS/Docker Desktop"
        ));
    }

    #[test]
    fn an_interpreter_is_known_in_any_case_with_a_version_and_shows_the_script_it_runs() {
        let name = |args: &[&str]| display_name(&words(args), None);
        // `python3.12`, `ruby3.3`, and the framework's `Python`.
        assert_eq!(
            name(&["python3.12", "-m", "http.server", "8765"]).as_deref(),
            Some("http.server")
        );
        assert_eq!(
            name(&["/usr/bin/ruby3.3", "bin/rails", "server"]).as_deref(),
            Some("rails")
        );
        assert_eq!(
            name(&[
                "/opt/homebrew/Cellar/python@3.13/3.13.1/Frameworks/Python.framework/Versions/3.13/Resources/Python.app/Contents/MacOS/Python",
                "-m",
                "http.server",
                "8765"
            ])
            .as_deref(),
            Some("http.server")
        );
        assert_eq!(
            name(&["/x/Python.app/Contents/MacOS/Python", "-u", "serve.py"]).as_deref(),
            Some("serve.py")
        );
        // With no script, the interpreter's own name, as it runs.
        assert_eq!(
            name(&["/x/Python.app/Contents/MacOS/Python"]).as_deref(),
            Some("Python")
        );
        assert_eq!(
            name(&["node", "-e", "require('http')"]).as_deref(),
            Some("node")
        );
        // A script by a relative path, and an option that takes a word.
        assert_eq!(
            name(&["node", "src/server.js"]).as_deref(),
            Some("server.js")
        );
        assert_eq!(
            name(&["node", "-r", "ts-node/register", "src/app/main.ts"]).as_deref(),
            Some("main.ts")
        );
        // A program by a relative path.
        assert_eq!(
            name(&["./node_modules/.bin/vite", "--port", "5173"]).as_deref(),
            Some("vite")
        );
        assert_eq!(name(&["bin/serve"]).as_deref(), Some("serve"));
        // A name that only starts like an interpreter is a program of its own.
        assert_eq!(name(&["nodemon", "app.js"]).as_deref(), Some("nodemon"));
        assert_eq!(name(&["phpstorm"]).as_deref(), Some("phpstorm"));
    }

    #[test]
    fn a_port_this_test_opens_answers_and_one_nobody_holds_does_not() {
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        assert!(System.listening(port));
        drop(socket);
        assert!(!System.listening(port));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_port_this_test_holds_is_found_among_the_listeners_with_a_name() {
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        let found = System.listeners();
        let mine = found
            .iter()
            .find(|listener| listener.port == port)
            .expect("the listener is found");
        assert!(!mine.name.is_empty());
    }
}
