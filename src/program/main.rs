//! browserpen: a web browser in the terminal pane. In a terminal that shows pencere views and their
//! browsers (bateri) the tabs, the address bar and the pages open in the pane, and what the pages
//! load comes from the machine the program runs on: `localhost:3000` is that machine's, over ssh or
//! not. In one that speaks the Kitty graphics protocol and shows no browser, the program draws the
//! tabs and the address bar as pictures (`pencere-tui`, the `terminal` feature, on by default) and an
//! address opens in the person's own browser; where it cannot, it says so and ends.
//!
//! The program looks at the machine and remembers the addresses; the page (`src/view/`, in pencere's
//! kit) is the browser's face and asks the program for what it needs with the calls of
//! `docs/calls.md`.
//!
//! This file is the entry and nothing more: it reads the command line, makes the session and gives
//! the calls of `api/` to `serve`.
//!
//!   api/       everything the page talks to the program about; it knows the page
//!   machine/   the machine's listening ports and whether one answers; knows nothing of the page
//!   state/     the addresses a profile has been to; same

mod api;
mod command_line;
mod icons;
mod machine;
#[cfg(test)]
mod page_shots;
#[cfg(test)]
mod page_tests;
mod session;
mod state;

use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};

#[cfg(not(feature = "terminal"))]
use pencere::serve;
use pencere::{EndedBy, Options, Outcome, ViewControl};
#[cfg(feature = "terminal")]
use pencere_tui::serve;

use command_line::Asked;
use machine::System;
use session::Session;
use state::history::{History, state_dir};

/// What a run that shows no view says: browserpen has nothing to show a page with there.
#[cfg(feature = "terminal")]
const NO_VIEWS: &str = "browserpen shows pages in a terminal that hosts pencere (bateri), and draws its tabs and address bar itself in\none that speaks Kitty graphics (set PENCERE_TUI=1 to ask for that); here it was not shown.";
#[cfg(not(feature = "terminal"))]
const NO_VIEWS: &str = "browserpen shows pages in a terminal that hosts pencere (bateri); this build does not draw anything in any\nother. The build that does is the one `install.sh` fetches.";

/// What `serve` is given: the app, its name and icons, and the calls of `api/`.
fn options(session: &Arc<Session>, fallen_back: &Arc<Mutex<Option<u8>>>) -> Options {
    let mut options = Options::new()
        .app(pencere::app!())
        .program("browserpen")
        // What a terminal writes above the view.
        .title("Browser")
        // An address the person opens in the terminal while browserpen runs is handed to it, to open in
        // a tab of its own (the page subscribes to `pencere.take`).
        .takes(&[pencere::TakeKind::Url]);
    for icon in icons::all() {
        options = options.icon(icon);
    }
    let options = api::serve(options, session);
    let (start, status) = (session.start.clone(), Arc::clone(fallen_back));
    options.fallback(move |_| {
        eprintln!("browserpen: {NO_VIEWS}");
        if let Some(address) = &start {
            eprintln!("The address to open in your own browser: {address}");
        }
        *status.lock().unwrap_or_else(PoisonError::into_inner) = Some(1);
    })
}

fn main() -> ExitCode {
    let line = match command_line::read(std::env::args().skip(1)) {
        Asked::Run(line) => line,
        Asked::Say(words) => {
            print!("{words}");
            return ExitCode::SUCCESS;
        }
        Asked::Refuse(words) => {
            eprintln!("{words}");
            return ExitCode::from(2);
        }
    };
    let history =
        state_dir().map_or_else(History::memory, |dir| History::open(&dir, &line.profile));
    let view = ViewControl::new();
    let session = Arc::new(Session::new(
        &line.profile,
        line.url,
        Arc::new(System),
        history,
        view.clone(),
    ));
    let fallen_back = Arc::new(Mutex::new(None));
    let outcome = serve(options(&session, &fallen_back).control(view));
    let status = match outcome {
        // The terminal took the view down: what it said is the one trace of why, so it is left under
        // the prompt rather than lost with the view.
        Ok(Outcome::Closed(closed)) if closed.by == EndedBy::Error => {
            let words = closed
                .message
                .as_deref()
                .unwrap_or("the terminal gave no reason");
            eprintln!("browserpen: the view stopped: {words}");
            1
        }
        Ok(outcome) => fallen_back
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .unwrap_or_else(|| outcome.shell_status()),
        Err(why) => {
            eprintln!("browserpen: {why}");
            1
        }
    };
    ExitCode::from(status)
}
