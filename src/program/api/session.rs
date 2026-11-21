//! The calls of the page: `boot` (what it shows first), `ports` (the machine's listening ports),
//! `probe` (whether one answers), `visit` (an address the person has been to) and `quit`.

use std::sync::Arc;

use pencere::{CloseResult, Options};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::call;
use crate::machine::Listener;
use crate::session::Session;
use crate::state::history::Visit;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Boot {
    version: &'static str,
    profile: String,
    /// The address the command line gave, as it was typed.
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<String>,
    ports: Vec<Listener>,
    recent: Vec<Visit>,
}

/// A call that takes nothing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Nothing {}

/// What the page shows first: the profile, the address asked for, the ports and the recent addresses.
fn boot(session: &Session, _: Nothing) -> Boot {
    Boot {
        version: env!("CARGO_PKG_VERSION"),
        profile: session.profile.clone(),
        start: session.start.clone(),
        ports: session.machine.listeners(),
        recent: session.history.list(),
    }
}

/// The ports the person's processes listen on, as of now: the page asks while it shows them.
fn ports(session: &Session, _: Nothing) -> Value {
    json!({ "ports": session.machine.listeners() })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    port: u16,
}

/// Whether anything answers on the loopback of this machine at `port`: the page asks it of a
/// `localhost` address that did not load, to say that nothing is listening yet (the Ports list is
/// the person's own processes' alone, and a server another user, root or a container holds is
/// listening all the same).
fn probe(session: &Session, args: Probe) -> Value {
    json!({ "listening": session.machine.listening(args.port) })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Visited {
    url: String,
    #[serde(default)]
    title: String,
}

/// The person has been to `url`: the profile's recent addresses have it first.
fn visit(session: &Session, args: Visited) -> Value {
    session.history.record(&args.url, &args.title);
    json!({})
}

/// Ends browserpen: a page of the kit has no close of its own, its view is the program's to end.
fn quit(session: &Session, _: Nothing) -> Value {
    match session.view.close() {
        // Closed, the answer has no view to go to; with no view to close, there is nothing to do.
        CloseResult::Closed | CloseResult::Unavailable => json!({}),
        CloseResult::Unsupported => {
            json!({ "unsupported": "This terminal cannot close the view: ⌃C ends browserpen." })
        }
        // What a later version of the library says, which this one does not know.
        _ => json!({}),
    }
}

pub fn calls(options: Options, session: &Arc<Session>) -> Options {
    let options = call(options, session, "boot", boot);
    let options = call(options, session, "ports", ports);
    let options = call(options, session, "probe", probe);
    let options = call(options, session, "visit", visit);
    call(options, session, "quit", quit)
}
