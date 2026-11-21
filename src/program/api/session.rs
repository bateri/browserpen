//! The calls of the page: `boot` (what it shows first), `ports` (the machine's listening ports),
//! `probe` (whether one answers), `visit` (an address the person has been to), `tabs` (the tabs it has
//! open, for a view that starts again) and `quit`.

use std::sync::Arc;

use pencere::{CloseResult, Options};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::call;
use crate::machine::Listener;
use crate::session::Session;
use crate::state::history::Visit;
use crate::state::tabs::{Tab, TabSet};

/// A listening port as the page is told it (`docs/calls.md`): made here from what `machine/` holds,
/// member by member, so that the page's words are this file's and not those of a type that knows
/// nothing of the page.
#[derive(Serialize)]
struct Port {
    port: u16,
    name: String,
}

/// An address the profile has been to, as the page is told it.
#[derive(Serialize)]
struct Recent {
    url: String,
    title: String,
}

fn ports_of(listeners: Vec<Listener>) -> Vec<Port> {
    listeners
        .into_iter()
        .map(|listener| Port {
            port: listener.port,
            name: listener.name,
        })
        .collect()
}

fn recent_of(visits: Vec<Visit>) -> Vec<Recent> {
    visits
        .into_iter()
        .map(|visit| Recent {
            url: visit.url,
            title: visit.title,
        })
        .collect()
}

/// A tab as the page says it and is told it.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TabOf {
    key: String,
    /// Where the tab was; missing for a tab with no address yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    title: String,
    /// How large the page was shown, where it was not its own size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    zoom: Option<f64>,
}

/// The tabs of the page, in the order of the strip.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TabsOf {
    tabs: Vec<TabOf>,
    /// The key of the tab in front.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active: Option<String>,
    /// The number the next tab is named by.
    #[serde(default)]
    serial: u64,
}

impl From<TabsOf> for TabSet {
    fn from(of: TabsOf) -> Self {
        Self {
            tabs: of
                .tabs
                .into_iter()
                .filter(|tab| !tab.key.is_empty())
                .map(|tab| Tab {
                    key: tab.key,
                    url: tab.url.filter(|url| !url.is_empty()),
                    title: tab.title,
                    zoom: tab.zoom.filter(|zoom| zoom.is_finite()),
                })
                .collect(),
            active: of.active,
            serial: of.serial,
        }
    }
}

impl From<TabSet> for TabsOf {
    fn from(set: TabSet) -> Self {
        Self {
            tabs: set
                .tabs
                .into_iter()
                .map(|tab| TabOf {
                    key: tab.key,
                    url: tab.url,
                    title: tab.title,
                    zoom: tab.zoom,
                })
                .collect(),
            active: set.active,
            serial: set.serial,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Boot {
    version: &'static str,
    profile: String,
    /// The address the command line gave, as it was typed: in the answer to the first `boot` only.
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<String>,
    ports: Vec<Port>,
    recent: Vec<Recent>,
    /// The tabs the page last said it had, where it said any: for a view that starts again.
    #[serde(skip_serializing_if = "Option::is_none")]
    tabs: Option<TabsOf>,
}

#[derive(Serialize)]
struct Ports {
    ports: Vec<Port>,
}

/// A call that takes nothing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Nothing {}

/// What the page shows first: the profile, the address asked for (the first time only), the ports, the
/// recent addresses and the tabs it had open if a page of this run said so.
fn boot(session: &Session, _: Nothing) -> Boot {
    Boot {
        version: env!("CARGO_PKG_VERSION"),
        profile: session.profile.clone(),
        start: session.take_start(),
        ports: ports_of(session.machine.listeners()),
        recent: recent_of(session.history.list()),
        tabs: session.tabs.kept().map(TabsOf::from),
    }
}

/// The ports the person's processes listen on, as of now: the page asks while it shows them.
fn ports(session: &Session, _: Nothing) -> Ports {
    Ports {
        ports: ports_of(session.machine.listeners()),
    }
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

/// The tabs the page has open now: kept for the run, for a page that starts again. It says them all
/// each time any of them changes.
fn tabs(session: &Session, args: TabsOf) -> Value {
    session.tabs.keep(args.into());
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
    let options = call(options, session, "tabs", tabs);
    call(options, session, "quit", quit)
}
