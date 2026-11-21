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
    /// The process as `ps` shows it: its `argv[0]`'s last component, or, for an interpreter (`node`,
    /// `python`), the script it runs; a rewritten title is kept whole.
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

/// The name a process is shown by, from its arguments: the last component of `argv[0]` (a title
/// that was rewritten, with spaces, whole); where that is an interpreter, the script it was given,
/// which is what the person started.
#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn display_name(args: &[String], kernel_name: Option<&str>) -> Option<String> {
    const INTERPRETERS: [&str; 9] = [
        "node", "bun", "deno", "python", "python3", "ruby", "php", "java", "tsx",
    ];
    let last = |word: &str| -> String {
        if word.starts_with('/') && !word.contains(' ') {
            word.rsplit('/').next().unwrap_or(word).to_owned()
        } else {
            word.to_owned()
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
