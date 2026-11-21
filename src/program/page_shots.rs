//! Pictures of the page for the eye, in `target/shots/`: the page and the program's handlers in one
//! process (as `page_tests` runs them), over a stand-in machine, laid out and drawn in the fonts of
//! this machine by pencere's host with no screen. Not a terminal's drawing, and the room of a browser
//! is empty in them: what the host's browser draws there is not the page's. Each scene at the width of a
//! narrow pane (480 points) and of a wide one (1100), where the page's bars and cards are looked at.
//!
//! They draw in the fonts of the machine they are made on, so they are not part of the gate: `make
//! shots` makes them.

use std::path::{Path, PathBuf};

use pencere_drive::headless::Error;
use pencere_drive::shot::{Fonts, Palette};
use pencere_view::{BrowserError, BrowserFound, BrowserState};

use crate::page_tests::{
    NARROW, Page, Stand, WIDE, open_with, ready, session_over, tell, write_address,
};
use crate::state::history::History;

fn shots(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/shots");
    std::fs::create_dir_all(&dir).expect("a directory for the pictures");
    dir.join(name)
}

fn save(page: &Page, fonts: &Fonts, name: &str, width: &str) {
    let png = page.shot_in(fonts, Palette::Dark).expect("a picture");
    std::fs::write(shots(&format!("{name}-{width}.png")), png).expect("the picture written");
}

/// A machine with a few of the ports a developer has open, and a history with a few pages.
fn machine() -> (std::sync::Arc<Stand>, History) {
    let stand = Stand::with(&[
        (3000, "next-server"),
        (5173, "vite"),
        (8080, "python3"),
        (9229, "node"),
    ]);
    let history = History::memory();
    history.record("http://localhost:3000/", "Dashboard");
    history.record("http://localhost:5173/docs", "Docs - local");
    history.record("https://github.com/bateri/pencere", "bateri/pencere");
    history.record(
        "https://www.google.com/search?q=rust+lifetimes",
        "rust lifetimes - Google Search",
    );
    (stand, history)
}

/// A page whose tab 1 is at `localhost:3000`, titled "Dashboard", and two more tabs, tab 3 in front.
fn at_pages(size: (f32, f32), fonts: &Fonts) -> Result<Page, Error> {
    let (stand, history) = machine();
    let session = session_over(stand, history, None);
    let mut page = open_with(&session, size, Some(fonts), true)?;
    ready(&mut page, "tab-1", "", "");
    write_address(&mut page, "3000");
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "Dashboard".to_owned();
        state.back = true;
    });
    for (tab, number, title) in [
        ("tab-2", "5173", "Docs - local"),
        (
            "tab-3",
            "https://github.com/bateri/pencere",
            "bateri/pencere",
        ),
    ] {
        page.press("ctrl+t")?;
        ready(&mut page, tab, "", "");
        write_address(&mut page, number);
        let url = if number.starts_with("http") {
            number.to_owned()
        } else {
            format!("http://localhost:{number}/")
        };
        tell(&mut page, tab, |state| {
            state.url = url;
            state.title = title.to_owned();
        });
    }
    page.advance(0.4)?;
    Ok(page)
}

#[test]
#[ignore = "shots: draws the page in the fonts of this machine into target/shots/; make shots"]
fn the_pictures_of_the_page() -> Result<(), Error> {
    let fonts = Fonts::installed();
    for (width, size) in [("narrow", NARROW), ("wide", WIDE)] {
        // An empty tab: the ports and the recent addresses.
        let (stand, history) = machine();
        let session = session_over(stand, history, None);
        let page = open_with(&session, size, Some(&fonts), true)?;
        save(&page, &fonts, "empty-tab", width);

        // An open page, with three tabs and the keyboard in it.
        let mut page = at_pages(size, &fonts)?;
        save(&page, &fonts, "three-tabs", width);

        // Loading.
        tell(&mut page, "tab-3", |state| {
            state.url = "https://github.com/bateri/pencere".to_owned();
            state.title = "bateri/pencere".to_owned();
            state.loading = true;
            state.back = true;
        });
        page.advance(0.3)?;
        save(&page, &fonts, "loading", width);

        // The find bar, with what it found.
        tell(&mut page, "tab-3", |state| {
            state.url = "https://github.com/bateri/pencere".to_owned();
            state.title = "bateri/pencere".to_owned();
            state.back = true;
        });
        page.press("cmd+f")?;
        page.browser_unused("tab-3", "cmd+f")?;
        page.type_text("browser")?;
        tell(&mut page, "tab-3", |state| {
            state.url = "https://github.com/bateri/pencere".to_owned();
            state.title = "bateri/pencere".to_owned();
            state.back = true;
            state.found = Some(BrowserFound {
                count: 12,
                current: Some(3),
            });
        });
        page.advance(0.3)?;
        save(&page, &fonts, "find-bar", width);
        page.press("escape")?;

        // Zoom 125%.
        tell(&mut page, "tab-3", |state| {
            state.url = "https://github.com/bateri/pencere".to_owned();
            state.title = "bateri/pencere".to_owned();
            state.back = true;
            state.zoom = 1.25;
        });
        page.advance(0.3)?;
        save(&page, &fonts, "zoom-125", width);

        // The list of commands.
        page.press("f1")?;
        page.browser_unused("tab-3", "f1")?;
        page.advance(0.4)?;
        save(&page, &fonts, "commands", width);
        page.press("escape")?;

        // A load that failed.
        tell(&mut page, "tab-3", |state| {
            state.url = "https://nope.test/".to_owned();
            state.title = String::new();
            state.back = true;
            state.error = Some(BrowserError {
                url: "https://nope.test/".to_owned(),
                code: -105,
                message: "The server's name could not be found".to_owned(),
            });
        });
        page.advance(0.3)?;
        save(&page, &fonts, "load-error", width);

        // A port nothing listens on.
        tell(&mut page, "tab-3", |state| {
            state.url = "http://localhost:4000/".to_owned();
            state.back = true;
            state.error = Some(BrowserError {
                url: "http://localhost:4000/".to_owned(),
                code: -102,
                message: "Connection refused".to_owned(),
            });
        });
        page.wait_for(std::time::Duration::from_secs(20), |page| {
            page.says("Nothing is listening")
        })?;
        page.advance(0.3)?;
        save(&page, &fonts, "nothing-listening", width);

        // A browser that stopped.
        page.browser_state(
            "tab-3",
            BrowserState::with_status(BrowserState::CRASHED, Some("The browser stopped.")),
        )?;
        page.advance(0.3)?;
        save(&page, &fonts, "crashed", width);

        // A host that shows no browser.
        let (stand, history) = machine();
        let session = session_over(stand, history, None);
        let mut page = open_with(&session, size, Some(&fonts), false)?;
        write_address(&mut page, "3000");
        page.advance(0.4)?;
        save(&page, &fonts, "unsupported", width);
    }
    Ok(())
}
