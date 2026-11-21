//! What the machine the program runs on says to the page: the TCP ports its person's processes
//! listen on (with the name of the process), and whether one port answers. It knows nothing of the
//! page. `macos.rs` reads them with libproc, `linux.rs` from `/proc`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::time::Duration;

use serde::Serialize;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

/// How long a connection to a port is waited for: the loopback answers at once or not at all.
const PROBE: Duration = Duration::from_millis(250);

/// The most listeners the page is told of: a machine with more is a fork bomb, not a dev server.
pub const MOST: usize = 64;

/// A TCP port a process of the person listens on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
/// path (a title that was rewritten, and is no path, whole); where that is an interpreter, the
/// script it was given, which is what the person started.
#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn display_name(args: &[String], kernel_name: Option<&str>) -> Option<String> {
    const INTERPRETERS: [&str; 9] = [
        "node", "bun", "deno", "python", "python3", "ruby", "php", "java", "tsx",
    ];
    // A path is shown by its last part, spaces in the path or not (`Application Support`); a word that
    // is no path is a title the process gave itself, and stays whole.
    let last = |word: &str| -> String {
        let name = word.rsplit('/').find(|part| !part.is_empty());
        match name {
            Some(name) if word.starts_with('/') => name.to_owned(),
            _ => word.to_owned(),
        }
    };
    let first = args
        .first()
        .filter(|arg| !arg.is_empty())
        .map(|arg| last(arg));
    match first {
        Some(name) if INTERPRETERS.contains(&name.as_str()) => {
            let script = args
                .iter()
                .skip(1)
                .find(|arg| !arg.is_empty() && !arg.starts_with('-'))
                .map(|arg| last(arg));
            Some(script.unwrap_or(name))
        }
        Some(name) => Some(name),
        None => kernel_name
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
    }
}

/// Whether the executable at `path` is inside an application bundle (`Foo.app/Contents/MacOS/foo`):
/// what the person opened from the Dock or the system started, not what a developer started in a
/// terminal, which is what the page's list is for. A name that only ends in `.app` and is the
/// executable itself, or a directory that only contains it (`my.app-server`), is no bundle.
#[cfg(any(target_os = "macos", test))]
fn in_an_app_bundle(path: &str) -> bool {
    let mut parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    // The executable itself.
    parts.pop();
    parts
        .iter()
        .any(|part| part.len() > ".app".len() && part.to_ascii_lowercase().ends_with(".app"))
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
    fn a_program_inside_an_application_bundle_is_told_from_one_started_in_a_terminal() {
        for path in [
            "/System/Library/CoreServices/ControlCenter.app/Contents/MacOS/ControlCenter",
            "/Applications/OrbStack.app/Contents/Frameworks/OrbStack Helper.app/Contents/MacOS/OrbStack Helper",
            "/Users/me/Library/Application Support/Figma/FigmaAgent.app/Contents/MacOS/FigmaAgent",
            "/Applications/Postgres.APP/Contents/Versions/17/bin/postgres",
        ] {
            assert!(in_an_app_bundle(path), "{path}");
        }
        for path in [
            "/opt/homebrew/bin/node",
            "/Users/me/.cargo/bin/cargo",
            "/Users/me/Projects/my.app-server/target/debug/serve",
            "/Users/me/bin/Thing.app",
            "/Users/me/.app/serve",
            "node",
            "",
        ] {
            assert!(!in_an_app_bundle(path), "{path}");
        }
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
