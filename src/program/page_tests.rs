//! The page (src/view/) and the program's own handlers in one process, as a terminal runs them: the
//! kit's interpreter under pencere's host with no screen (`pencere-drive`), a clock the test moves, the
//! calls of `api/` served by `pencere::Conversation` as `serve` serves them over a stand-in machine
//! (its ports and what answers on them are the test's), and the host's browsers told what the test says
//! they are (`browser_state`): the room of each is a placeholder, and what the page asks of it is read
//! back (`browser_asks`). What the page shows is read as words; what the person does is done to it as a
//! terminal does it. A call of the page is held to the kit's 64 MiB, and to its 250 ms a call when asked
//! (`make check-timing`; `limits`).
//!
//! Each opens the interpreter (compiled once a run): `make test-page` runs them, in `make check`.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use pencere::ViewControl;
use pencere_drive::headless::{Error, Focus, Headless, InProcess};
use pencere_view::{
    BrowserAsk, BrowserError, BrowserFound, BrowserState, Id, Key, KeyConvention, MenuEntry, Mods,
    Point,
};

use crate::machine::{Listener, Machine};
use crate::session::Session;
use crate::state::history::History;
use crate::state::tabs::{Tab, TabSet};

/// A page as wide as the docs' pictures, and one as narrow as a pane of 80 columns.
pub(crate) const WIDE: (f32, f32) = (1100.0, 720.0);
pub(crate) const NARROW: (f32, f32) = (480.0, 640.0);

/// How long a test waits for what the program says on its own (a call).
const WITHIN: Duration = Duration::from_secs(20);

pub(crate) type Page = Headless<InProcess>;

/// What the page's module is held to. The tests say what the page shows and does, not how fast a
/// machine is: a runner under load takes ten times as long as one at rest, so the kit's 250 ms a call
/// and 2 s to start, which are the wall clock's, are far wider here by default (`GENEROUS_CALL`). With
/// `BROWSERPEN_CALL_MS` set (`make check-timing`) a call is held to that many milliseconds, the rest to
/// the kit's own. Meant for a machine at rest.
pub(crate) fn limits(call_ms: Option<u64>) -> pencere_sandbox::ModuleLimits {
    let kit = pencere_sandbox::ModuleLimits::default();
    match call_ms {
        Some(ms) => pencere_sandbox::ModuleLimits {
            call: Duration::from_millis(ms),
            ..kit
        },
        None => pencere_sandbox::ModuleLimits {
            call: GENEROUS_CALL,
            start: GENEROUS_CALL * 6,
            ..kit
        },
    }
}

/// A call of the page, in a test that is not about time: forty times the kit's.
const GENEROUS_CALL: Duration = Duration::from_secs(10);

/// A machine of the test's making: the ports it lists and the ports that answer.
#[derive(Default)]
pub(crate) struct Stand {
    ports: Mutex<Vec<Listener>>,
    answering: Mutex<BTreeSet<u16>>,
    probes: Mutex<Vec<u16>>,
}

impl Stand {
    /// The stand-in with these ports listed (and answering).
    pub(crate) fn with(ports: &[(u16, &str)]) -> Arc<Self> {
        let stand = Arc::new(Self::default());
        for (port, name) in ports {
            stand.listen(*port, name);
        }
        stand
    }

    /// Something starts listening on `port`.
    pub(crate) fn listen(&self, port: u16, name: &str) {
        self.ports
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Listener {
                port,
                name: name.to_owned(),
            });
        self.answer(port);
    }

    /// `port` answers, though no process of the person's lists it (root's, a container's).
    pub(crate) fn answer(&self, port: u16) {
        self.answering
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(port);
    }

    /// The ports the page asked about, in order.
    pub(crate) fn probed(&self) -> Vec<u16> {
        self.probes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Machine for Stand {
    fn listeners(&self) -> Vec<Listener> {
        let mut ports = self
            .ports
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        ports.sort_by_key(|listener| listener.port);
        ports
    }

    fn listening(&self, port: u16) -> bool {
        self.probes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(port);
        self.answering
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&port)
    }
}

/// A session over `machine`, with the profile `profile`, asked to open `start`.
pub(crate) fn session_over(
    machine: Arc<Stand>,
    history: History,
    start: Option<&str>,
) -> Arc<Session> {
    Arc::new(Session::new(
        "test",
        start.map(str::to_owned),
        machine,
        history,
        ViewControl::new(),
    ))
}

/// The page of this checkout's src/view/ (and its pictures, src/assets/), on the handlers of
/// `session`, at `size`, with its first tab drawn. Where `browsers` is false the host shows none.
pub(crate) fn open_with(
    session: &Arc<Session>,
    size: (f32, f32),
    fonts: Option<&pencere_drive::shot::Fonts>,
    browsers: bool,
) -> Result<Page, Error> {
    open_keyed(session, size, fonts, browsers, None)
}

/// As [`open_with`], on the keyboard of `keyboard` (a Mac's where none is said).
pub(crate) fn open_keyed(
    session: &Arc<Session>,
    size: (f32, f32),
    fonts: Option<&pencere_drive::shot::Fonts>,
    browsers: bool,
    keyboard: Option<KeyConvention>,
) -> Result<Page, Error> {
    let program = InProcess::new(crate::options(session, &Arc::new(Mutex::new(None))));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64());
    let mut opening = Headless::app(env!("CARGO_MANIFEST_DIR"))
        .size(size.0, size.1)
        .browsers(browsers)
        .wall_clock(now, 0)
        .limits(limits(
            std::env::var("BROWSERPEN_CALL_MS")
                .ok()
                .and_then(|ms| ms.parse().ok()),
        ));
    if let Some(keyboard) = keyboard {
        opening = opening.keyboard(keyboard);
    }
    if let Some(fonts) = fonts {
        opening = opening.measure(fonts.clone()).scale(2.0);
    }
    let mut page = opening.peer(program).open()?;
    // What the page shows first comes from the program on its own threads: a blank tab, or the tab
    // (and its browser) of the address the command line gave.
    page.wait_for(WITHIN, |page| {
        !page.browsers().is_empty() || page.says("New Tab")
    })?;
    page.advance(0.3)?;
    Ok(page)
}

/// The page over a machine with a few ports, and nothing asked for.
pub(crate) fn open(size: (f32, f32)) -> (Arc<Stand>, Arc<Session>, Page) {
    let stand = Stand::with(&[(3000, "next-server"), (5173, "vite")]);
    let session = session_over(Arc::clone(&stand), History::memory(), None);
    let page = open_with(&session, size, None, true).expect("the page opens");
    (stand, session, page)
}

/// The words of the page, one string.
pub(crate) fn said(page: &Page) -> String {
    page.words().join("|")
}

/// What the page has asked of its browsers, as `(tab, ask)`.
pub(crate) fn asks(page: &Page) -> Vec<(String, BrowserAsk)> {
    page.browser_asks()
        .iter()
        .map(|(id, ask)| (format!("{id:?}"), ask.clone()))
        .collect()
}

/// The asks of one tab's browser, in order.
pub(crate) fn asked_of(page: &Page, tab: &str) -> Vec<BrowserAsk> {
    let id = Id::named(tab);
    page.browser_asks()
        .iter()
        .filter(|(browser, _)| *browser == id)
        .map(|(_, ask)| ask.clone())
        .collect()
}

/// The browser `tab` says it is ready, at `url`, with `title`.
pub(crate) fn ready(page: &mut Page, tab: &str, url: &str, title: &str) {
    page.browser_state(
        tab,
        BrowserState {
            url: url.to_owned(),
            title: title.to_owned(),
            ..BrowserState::default()
        },
    )
    .expect("the browser is told");
}

/// The browser `tab` says what it is.
pub(crate) fn tell(page: &mut Page, tab: &str, change: impl FnOnce(&mut BrowserState)) {
    let mut state = BrowserState::default();
    change(&mut state);
    page.browser_state(tab, state).expect("the browser is told");
}

/// Where the keyboard is, by the name of the thing that has it (the field's name, or the tab's).
pub(crate) fn keyboard(page: &Page) -> String {
    match page.focus() {
        Focus::Page => "page".to_owned(),
        Focus::Field(id) | Focus::Browser(id) | Focus::Editor(id) => format!("{id:?}"),
    }
}

/// The page asked its browser `tab` to go to `url`.
fn went_to(page: &Page, tab: &str, url: &str) -> bool {
    asked_of(page, tab)
        .iter()
        .any(|ask| matches!(ask, BrowserAsk::Go(got) if got.trim_end_matches('/') == url.trim_end_matches('/')))
}

/// The person writes `words` in the address field and sends it.
pub(crate) fn write_address(page: &mut Page, words: &str) {
    page.press("cmd+l").expect("the address");
    // What the page put in the field (the address, selected) reaches it on the next frame, before a
    // person's first key.
    page.advance(0.05).expect("the field is filled");
    page.type_text(words).expect("typed");
    page.press("enter").expect("sent");
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_new_run_opens_a_blank_tab_with_the_address_field_ready_and_the_ports_listed() {
    let (_stand, _session, page) = open(WIDE);
    assert!(page.says("New Tab"), "{}", said(&page));
    assert!(page.says("Ports"), "{}", said(&page));
    assert!(page.says(":5173"), "{}", said(&page));
    assert!(page.says("vite"), "{}", said(&page));
    assert!(
        matches!(page.focus(), Focus::Field(_)),
        "{:?}",
        page.focus()
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_blank_tab_has_no_browser_and_gets_one_with_its_address_that_is_not_called_about_blank() {
    let (_stand, _session, mut page) = open(WIDE);
    // A browser made for nothing would show "about:blank" for its title and have it behind it.
    assert!(page.browsers().is_empty(), "{:?}", page.browsers().len());
    assert!(page.says("New Tab"), "{}", said(&page));
    page.press("ctrl+t").expect("a new tab");
    assert!(page.browsers().is_empty(), "{:?}", page.browsers().len());
    write_address(&mut page, "3000");
    let made: Vec<_> = page.browsers().iter().map(|browser| browser.id).collect();
    assert_eq!(made, [Id::named("tab-2")]);
    // What a browser says of itself before it has been anywhere is neither the tab's title nor its
    // address.
    ready(&mut page, "tab-2", "about:blank", "about:blank");
    page.advance(0.1).expect("settled");
    assert!(!page.says("about:blank"), "{}", said(&page));
    assert!(page.says("localhost:3000"), "{}", said(&page));
    assert_eq!(page.field("address/field"), Some("localhost:3000"));
    assert!(
        went_to(&page, "tab-2", "http://localhost:3000"),
        "{:?}",
        asks(&page)
    );
    // Once the browser has been to an address, its about:blank is where the page itself went: the old
    // address does not stay in the tab.
    tell(&mut page, "tab-2", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "App".to_owned();
    });
    page.advance(0.1).expect("settled");
    assert!(page.says("App"), "{}", said(&page));
    tell(&mut page, "tab-2", |state| {
        state.url = "about:blank".to_owned();
        state.title = "about:blank".to_owned();
    });
    page.advance(0.1).expect("settled");
    assert!(
        !page.says("App") && !page.says("localhost:3000"),
        "{}",
        said(&page)
    );
    assert_eq!(page.field("address/field").unwrap_or(""), "");
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn an_address_is_asked_of_the_browser_only_once_it_says_it_is_ready() {
    let (_stand, _session, mut page) = open(WIDE);
    write_address(&mut page, "localhost:3000");
    assert!(asked_of(&page, "tab-1").is_empty(), "{:?}", asks(&page));
    ready(&mut page, "tab-1", "", "");
    assert!(
        went_to(&page, "tab-1", "http://localhost:3000"),
        "{:?}",
        asks(&page)
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn what_is_typed_is_an_address_a_port_or_a_search() {
    let table = [
        ("3000", "http://localhost:3000"),
        (":5173/docs", "http://localhost:5173/docs"),
        ("localhost", "http://localhost"),
        ("example.com/docs?x=1", "https://example.com/docs?x=1"),
        (
            "example.com/a b",
            "https://www.google.com/search?q=example.com%2Fa+b",
        ),
        (
            "rust lifetimes",
            "https://www.google.com/search?q=rust+lifetimes",
        ),
        (
            "çay demlemek",
            "https://www.google.com/search?q=%C3%A7ay+demlemek",
        ),
        ("3000abc", "https://www.google.com/search?q=3000abc"),
        ("https://a.test/ü", "https://a.test/%C3%BC"),
    ];
    for (typed, expected) in table {
        let (_stand, _session, mut page) = open(WIDE);
        write_address(&mut page, typed);
        ready(&mut page, "tab-1", "", "");
        assert!(
            went_to(&page, "tab-1", expected),
            "{typed}: {:?}",
            asked_of(&page, "tab-1")
        );
    }
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_address_on_the_command_line_opens_in_the_first_tab() {
    let stand = Stand::with(&[]);
    let session = session_over(stand, History::memory(), Some("3000"));
    let mut page = open_with(&session, WIDE, None, true).expect("opens");
    ready(&mut page, "tab-1", "", "");
    assert!(
        went_to(&page, "tab-1", "http://localhost:3000"),
        "{:?}",
        asks(&page)
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_tabs_are_the_same_when_the_view_starts_again_over_the_same_program() {
    let stand = Stand::with(&[(3000, "next-server"), (5173, "vite")]);
    let session = session_over(stand, History::memory(), Some("3000"));
    let mut page = open_with(&session, WIDE, None, true).expect("opens");
    // Three tabs: the first at the command line's address and shown at 150%, the second at another,
    // and a blank one put between them, the second in front.
    ready(&mut page, "tab-1", "", "");
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "App".to_owned();
        state.zoom = 1.5;
    });
    page.press("ctrl+t").expect("a new tab");
    write_address(&mut page, "5173");
    ready(&mut page, "tab-2", "", "");
    tell(&mut page, "tab-2", |state| {
        state.url = "http://localhost:5173/".to_owned();
        state.title = "Vite".to_owned();
    });
    page.press("ctrl+t").expect("a new tab");
    page.press("ctrl+shift+page_up").expect("left");
    page.press("ctrl+page_down").expect("next");
    let tab = |key: &str, url: Option<&str>, title: &str, zoom: Option<f64>| Tab {
        key: key.to_owned(),
        url: url.map(str::to_owned),
        title: title.to_owned(),
        zoom,
    };
    let kept = TabSet {
        tabs: vec![
            tab("tab-1", Some("http://localhost:3000/"), "App", Some(1.5)),
            tab("tab-3", None, "", None),
            tab("tab-2", Some("http://localhost:5173/"), "Vite", None),
        ],
        active: Some("tab-2".to_owned()),
        serial: 3,
    };
    page.wait_for(WITHIN, |_| session.tabs.kept().as_ref() == Some(&kept))
        .unwrap_or_else(|_| panic!("the program kept {:?}", session.tabs.kept()));
    drop(page);

    // The view starts again from nothing, over the program that goes on.
    let mut page = open_with(&session, WIDE, None, true).expect("opens again");
    let strip = ["tab-1", "tab-3", "tab-2"].map(|key| x_of(&page, key));
    assert!(strip[0] < strip[1] && strip[1] < strip[2], "{strip:?}");
    assert!(
        page.says("App") && page.says("Vite") && page.says("New Tab"),
        "{}",
        said(&page)
    );
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-2")));
    let made: BTreeSet<_> = page
        .browsers()
        .iter()
        .map(|browser| format!("{:?}", browser.id))
        .collect();
    let wanted: BTreeSet<_> = ["tab-1", "tab-2"]
        .map(|key| format!("{:?}", Id::named(key)))
        .into();
    assert_eq!(made, wanted, "a blank tab has no browser");
    // Each goes to its address when its browser is ready, the first at the size it had; the command
    // line's address is not sent again.
    ready(&mut page, "tab-1", "", "");
    ready(&mut page, "tab-2", "", "");
    assert_eq!(
        asked_of(&page, "tab-1")
            .into_iter()
            .filter(|ask| matches!(ask, BrowserAsk::Go(_) | BrowserAsk::Zoom(_)))
            .collect::<Vec<_>>(),
        [
            BrowserAsk::Go("http://localhost:3000/".to_owned()),
            BrowserAsk::Zoom(1.5)
        ]
    );
    assert!(
        went_to(&page, "tab-2", "http://localhost:5173/"),
        "{:?}",
        asks(&page)
    );
    assert!(
        !asks(&page).iter().any(|(tab, _)| tab.contains("tab-4")),
        "{:?}",
        asks(&page)
    );
    // The browsers are ready and have not said where they are: the program has what it had, the titles
    // and the size of the first among them. Where a load fails they stay as they were.
    page.advance(0.5).expect("settled");
    assert_eq!(session.tabs.kept().as_ref(), Some(&kept));
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.error = Some(BrowserError {
            url: "http://localhost:3000/".to_owned(),
            code: -102,
            message: "Connection refused".to_owned(),
        });
    });
    page.advance(0.5).expect("settled");
    assert_eq!(session.tabs.kept().as_ref(), Some(&kept));
    // The browsers load them and say where they are, at what size: the program has what it had.
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "App".to_owned();
        state.zoom = 1.5;
    });
    tell(&mut page, "tab-2", |state| {
        state.url = "http://localhost:5173/".to_owned();
        state.title = "Vite".to_owned();
    });
    page.advance(0.5).expect("settled");
    assert_eq!(session.tabs.kept().as_ref(), Some(&kept));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_long_title_is_cut_and_an_address_too_long_is_left_out_before_the_program_is_told() {
    let (_stand, session, mut page) = at_a_page();
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "é".repeat(500);
    });
    page.wait_for(WITHIN, |_| {
        session
            .tabs
            .kept()
            .is_some_and(|set| set.tabs[0].title.chars().count() == 200)
    })
    .unwrap_or_else(|_| panic!("the title was cut: {:?}", session.tabs.kept()));
    tell(&mut page, "tab-1", |state| {
        state.url = format!("http://localhost:3000/?q={}", "a".repeat(3000));
        state.title = "App".to_owned();
    });
    page.wait_for(WITHIN, |_| {
        session
            .tabs
            .kept()
            .is_some_and(|set| set.tabs[0].url.is_none())
    })
    .unwrap_or_else(|_| panic!("the address was left out: {:?}", session.tabs.kept()));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_keys_the_page_keeps_for_itself_are_told_to_the_browser_and_are_at_most_thirty_two() {
    let (_stand, _session, page) = at_a_page();
    let mut kept = 0;
    for browser in page.browsers() {
        assert!(
            browser.options.keys.len() <= 32,
            "{}",
            browser.options.keys.len()
        );
        kept += browser.options.keys.len();
    }
    assert!(kept > 0);
}

/// A page with tab 1 at `http://localhost:3000/` (titled "App"), its browser ready and the keyboard in it.
fn at_a_page() -> (Arc<Stand>, Arc<Session>, Page) {
    let (stand, session, mut page) = open(WIDE);
    write_address(&mut page, "3000");
    ready(&mut page, "tab-1", "", "");
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "App".to_owned();
    });
    page.advance(0.3).expect("settled");
    (stand, session, page)
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_page_is_shown_with_its_title_in_the_tab_and_its_address_without_the_scheme() {
    let (_stand, _session, page) = at_a_page();
    assert!(page.says("App"), "{}", said(&page));
    assert_eq!(page.field("address/field"), Some("localhost:3000"));
    assert!(!page.says("http://"), "{}", said(&page));
    assert_eq!(
        keyboard(&page),
        format!("{:?}", Id::named("tab-1")),
        "the keyboard is in the page"
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_page_keeps_its_own_keys_while_the_browser_has_the_keyboard() {
    let (_stand, _session, mut page) = at_a_page();
    // ⌘L is always the page's: the web page never sees it.
    page.press("cmd+l").expect("key");
    assert!(
        matches!(page.focus(), Focus::Field(_)),
        "{:?}",
        page.focus()
    );
    assert!(page.browser_keys().is_empty(), "{:?}", page.browser_keys());
    // ⌃T opens a blank tab whose address field has the keyboard.
    page.press("ctrl+t").expect("key");
    assert!(page.says("Ports"), "{}", said(&page));
    assert!(matches!(page.focus(), Focus::Field(_)));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_key_the_browser_has_no_use_for_comes_back_as_the_pages_own() {
    let (_stand, _session, mut page) = at_a_page();
    // Back is not possible yet: the key goes to the browser and, unused, to the page, which asks nothing.
    page.press("cmd+left").expect("key");
    page.browser_unused("tab-1", "cmd+left").expect("unused");
    assert!(
        !asked_of(&page, "tab-1").contains(&BrowserAsk::Back),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    // There is a page to go back to now.
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/two".to_owned();
        state.back = true;
    });
    page.press("cmd+left").expect("key");
    page.browser_unused("tab-1", "cmd+left").expect("unused");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::Back),
        "{:?}",
        asked_of(&page, "tab-1")
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn tabs_open_close_move_and_come_back() {
    let (_stand, _session, mut page) = at_a_page();
    page.press("ctrl+t").expect("a new tab");
    write_address(&mut page, "5173");
    ready(&mut page, "tab-2", "", "");
    assert!(
        went_to(&page, "tab-2", "http://localhost:5173"),
        "{:?}",
        asks(&page)
    );
    tell(&mut page, "tab-2", |state| {
        state.url = "http://localhost:5173/".to_owned();
        state.title = "Vite".to_owned();
    });
    assert!(page.says("Vite") && page.says("App"), "{}", said(&page));
    page.press("ctrl+1").expect("tab one");
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
    page.press("ctrl+page_down").expect("next");
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-2")));
    page.press("ctrl+w").expect("close");
    assert!(!page.says("Vite"), "{}", said(&page));
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
    // The closed tab comes back, as a tab of its own that goes to the address it had.
    page.press("cmd+shift+t").expect("reopen");
    ready(&mut page, "tab-3", "", "");
    assert!(
        went_to(&page, "tab-3", "http://localhost:5173/"),
        "{:?}",
        asks(&page)
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_last_tab_closed_leaves_a_blank_one() {
    let (_stand, _session, mut page) = at_a_page();
    page.press("ctrl+w").expect("close");
    assert!(
        page.says("New Tab") && page.says("Ports"),
        "{}",
        said(&page)
    );
    assert!(matches!(page.focus(), Focus::Field(_)));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_link_that_opens_a_window_is_a_tab_and_behind_this_one_where_that_was_asked() {
    let (_stand, _session, mut page) = at_a_page();
    page.browser_window("tab-1", "https://example.org/a", true)
        .expect("window");
    page.advance(0.1).expect("settled");
    assert!(page.says("Opened behind this tab"), "{}", said(&page));
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
    ready(&mut page, "tab-2", "", "");
    assert!(
        went_to(&page, "tab-2", "https://example.org/a"),
        "{:?}",
        asks(&page)
    );
    page.browser_window("tab-1", "https://example.org/b", false)
        .expect("window");
    ready(&mut page, "tab-3", "", "");
    assert!(went_to(&page, "tab-3", "https://example.org/b"));
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-3")));
}

/// A key that is not one the page keeps for itself (`always`): where the browser has the keyboard it is
/// the browser's first, and is the page's only as the host gives it back when the browser, and the web
/// page in it, had no use for it.
fn key(page: &mut Page, keys: &str) {
    let in_browser = matches!(page.focus(), Focus::Browser(_));
    page.press(keys).expect("key");
    if in_browser {
        page.browser_unused("tab-1", keys).expect("unused");
    }
}

fn escape(page: &mut Page) {
    key(page, "escape");
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn escape_stops_a_load_and_takes_the_find_bar_and_the_lists_away_and_leaves_the_address() {
    let (_stand, _session, mut page) = at_a_page();
    // 1. A load stops.
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.loading = true;
    });
    escape(&mut page);
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::Stop),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned()
    });
    // 2. The find bar closes, the search ends and the keyboard is the browser's.
    key(&mut page, "cmd+f");
    assert!(
        matches!(page.focus(), Focus::Field(_)),
        "{:?}",
        page.focus()
    );
    page.type_text("abc").expect("typed");
    page.press("escape").expect("escape");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::StopFinding),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
    // 3. The list of commands closes.
    key(&mut page, "f1");
    page.advance(0.3).expect("drawn");
    assert!(page.says("Reload"), "{}", said(&page));
    page.press("escape").expect("escape");
    assert!(
        !page.says("Show All Commands") && !page.says("Reload Without Cache"),
        "{}",
        said(&page)
    );
    // 4. The address field gives the address back, with the keyboard in the page.
    page.press("cmd+l").expect("address");
    page.advance(0.1).expect("filled");
    assert_eq!(page.field("address/field"), Some("http://localhost:3000/"));
    assert_eq!(page.field_selection("address/field"), Some(0..22));
    page.fill("address/field", "zzz").expect("typed");
    page.press("escape").expect("escape");
    page.advance(0.1).expect("settled");
    assert_eq!(page.field("address/field"), Some("localhost:3000"));
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_search_the_host_refuses_is_said_in_the_find_bar_and_not_as_an_address() {
    let (_stand, _session, mut page) = at_a_page();
    key(&mut page, "cmd+f");
    page.fill("find/field", "needle").expect("typed");
    page.advance(0.2).expect("settled");
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.found = Some(BrowserFound {
            count: 12,
            current: Some(3),
        });
    });
    assert!(page.says("3 of 12"), "{}", said(&page));
    // Longer than a browser is asked to find (1024 bytes): the host refuses it.
    page.fill("find/field", &"a".repeat(1100)).expect("pasted");
    page.advance(0.2).expect("settled");
    assert!(page.says("Not valid"), "{}", said(&page));
    assert!(
        !page.says("3 of 12") && !page.says("not an address"),
        "{}",
        said(&page)
    );
    // With a control character in it, the same.
    page.fill("find/field", "a\u{7}b").expect("typed");
    page.advance(0.2).expect("settled");
    assert!(page.says("Not valid"), "{}", said(&page));
    // What the browser can find is found again, and the bar says what it found.
    page.fill("find/field", "needle").expect("typed");
    page.advance(0.2).expect("settled");
    assert!(!page.says("Not valid"), "{}", said(&page));
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.found = Some(BrowserFound {
            count: 2,
            current: Some(1),
        });
    });
    assert!(page.says("1 of 2"), "{}", said(&page));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_find_bar_asks_the_browser_and_says_what_it_found() {
    let (_stand, _session, mut page) = at_a_page();
    key(&mut page, "cmd+f");
    assert!(matches!(page.focus(), Focus::Field(_)));
    page.type_text("needle").expect("typed");
    let asked = asked_of(&page, "tab-1");
    assert!(
        asked.iter().any(
            |ask| matches!(ask, BrowserAsk::Find { pattern, case: false } if pattern == "needle")
        ),
        "{asked:?}"
    );
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "App".to_owned();
        state.found = Some(BrowserFound {
            count: 12,
            current: Some(3),
        });
    });
    assert!(page.says("3 of 12"), "{}", said(&page));
    page.press("shift+enter").expect("previous");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::FindPrevious),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    page.press("enter").expect("next");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::FindNext),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    key(&mut page, "cmd+shift+g");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::FindPrevious),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    page.click("find-case").expect("match case");
    assert!(
        asked_of(&page, "tab-1")
            .iter()
            .any(|ask| matches!(ask, BrowserAsk::Find { case: true, .. })),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.found = Some(BrowserFound {
            count: 0,
            current: None,
        });
    });
    assert!(page.says("No matches"), "{}", said(&page));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn zoom_steps_from_the_size_the_browser_says_and_a_size_not_its_own_is_a_chip() {
    let (_stand, _session, mut page) = at_a_page();
    assert!(!page.says("100%"));
    key(&mut page, "cmd+=");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::Zoom(1.1)),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.zoom = 1.25;
    });
    assert!(page.says("125%"), "{}", said(&page));
    key(&mut page, "cmd+-");
    assert!(asked_of(&page, "tab-1").contains(&BrowserAsk::Zoom(1.1)));
    page.click("zoom-reset").expect("the chip");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::Zoom(1.0)),
        "{:?}",
        asked_of(&page, "tab-1")
    );
}

/// Zoom In is every key that types + or = with ⌘, so that a keyboard where neither is a key of its own
/// (Turkish Q: ⌘⇧4 for +, ⌘⇧0 for =) has it as a US one does.
#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn zoom_in_is_command_with_plus_or_equals_whatever_the_keyboard_types_them_with() {
    for keys in ["cmd++", "cmd+shift++", "cmd+=", "cmd+shift+="] {
        let (_stand, _session, mut page) = at_a_page();
        key(&mut page, keys);
        assert!(
            asked_of(&page, "tab-1").contains(&BrowserAsk::Zoom(1.1)),
            "{keys}: {:?}",
            asked_of(&page, "tab-1")
        );
    }
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_page_that_did_not_load_says_why_once_and_reload_asks_again() {
    let (_stand, _session, mut page) = at_a_page();
    tell(&mut page, "tab-1", |state| {
        state.url = "https://nope.test/".to_owned();
        state.error = Some(BrowserError {
            url: "https://nope.test/".to_owned(),
            code: -105,
            message: "The server's name could not be found".to_owned(),
        });
    });
    assert!(page.says("Couldn't load nope.test"), "{}", said(&page));
    assert!(
        page.says("The server's name could not be found"),
        "{}",
        said(&page)
    );
    page.click("card-reload").expect("reload");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::Reload { cache: true }),
        "{:?}",
        asked_of(&page, "tab-1")
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_browser_that_stopped_or_has_no_way_is_a_card_with_the_hosts_sentence() {
    let (_stand, _session, mut page) = at_a_page();
    page.browser_state(
        "tab-1",
        BrowserState::with_status(BrowserState::CRASHED, Some("The browser stopped.")),
    )
    .expect("told");
    assert!(page.says("The browser stopped."), "{}", said(&page));
    page.click("card-reload").expect("reload");
    assert!(asked_of(&page, "tab-1").contains(&BrowserAsk::Reload { cache: true }));
    page.browser_state(
        "tab-1",
        BrowserState::with_status(
            BrowserState::NO_ROUTE,
            Some("No way to this machine from here."),
        ),
    )
    .expect("told");
    assert!(
        page.says("No way to this machine from here."),
        "{}",
        said(&page)
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_port_nothing_listens_on_is_said_so_and_the_page_goes_there_by_itself_once_something_does() {
    let (stand, _session, mut page) = at_a_page();
    let failed = || BrowserError {
        url: "http://localhost:4000/".to_owned(),
        code: -102,
        message: "Connection refused".to_owned(),
    };
    let goes = |page: &Page| {
        asked_of(page, "tab-1")
            .iter()
            .filter(|ask| **ask == BrowserAsk::Go("http://localhost:4000/".to_owned()))
            .count()
    };
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:4000/".to_owned();
        state.error = Some(failed());
    });
    page.wait_for(WITHIN, |page| {
        page.says("Nothing is listening on :4000 yet")
    })
    .expect("said");
    assert!(!page.says("Connection refused"), "{}", said(&page));
    assert!(stand.probed().contains(&4000));
    assert_eq!(goes(&page), 0);
    // A server starts: the page asks the browser to go there again, as a person would with Reload,
    // and says that something is listening while it does.
    stand.listen(4000, "dev-server");
    page.advance(1.6).expect("time passes");
    page.wait_for(WITHIN, |page| goes(page) == 1)
        .expect("it goes");
    assert!(
        page.says("Something is listening on :4000 now"),
        "{}",
        said(&page)
    );
    // The browser loads it: no error, no card.
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:4000/".to_owned();
        state.loading = true;
    });
    page.advance(0.2).expect("settled");
    assert!(
        !page.says("Something is listening") && !page.says("Nothing is listening"),
        "{}",
        said(&page)
    );
    // Where that fails again, the page does not go on asking: the card says so and Reload is the
    // person's.
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:4000/".to_owned();
        state.error = Some(failed());
    });
    page.wait_for(WITHIN, |page| {
        page.says("Something is listening on :4000 now")
    })
    .expect("said again");
    page.advance(3.2).expect("time passes");
    assert_eq!(goes(&page), 1, "{:?}", asked_of(&page, "tab-1"));
    page.click("card-reload").expect("reload");
    assert!(
        asked_of(&page, "tab-1").contains(&BrowserAsk::Reload { cache: true }),
        "{:?}",
        asked_of(&page, "tab-1")
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn an_address_is_read_without_its_scheme_where_what_is_written_comes_back_to_it() {
    let (_stand, _session, mut page) = at_a_page();
    tell(&mut page, "tab-1", |state| {
        state.url = "https://example.com/docs".to_owned()
    });
    page.advance(0.1).expect("settled");
    assert_eq!(page.field("address/field"), Some("example.com/docs"));
    tell(&mut page, "tab-1", |state| {
        state.url = "http://example.com/docs".to_owned()
    });
    page.advance(0.1).expect("settled");
    assert_eq!(page.field("address/field"), Some("http://example.com/docs"));
    tell(&mut page, "tab-1", |state| {
        state.url = "https://localhost:3000/".to_owned()
    });
    page.advance(0.1).expect("settled");
    assert_eq!(page.field("address/field"), Some("https://localhost:3000/"));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn where_the_host_shows_no_browser_an_address_opens_in_the_persons_own() {
    let stand = Stand::with(&[]);
    let session = session_over(stand, History::memory(), None);
    let mut page = open_with(&session, WIDE, None, false).expect("opens");
    write_address(&mut page, "3000");
    assert_eq!(
        page.links(),
        ["http://localhost:3000".to_owned()].as_slice(),
        "{:?}",
        page.links()
    );
    assert!(page.says("Open in Your Browser"), "{}", said(&page));
    page.click("card-outside").expect("the card's button");
    assert_eq!(page.links().len(), 2, "{:?}", page.links());
    // The button is a command of the table: the palette has it, and does the same.
    key(&mut page, "f1");
    page.advance(0.3).expect("drawn");
    page.type_text("open in").expect("typed");
    page.advance(0.2).expect("settled");
    page.press("enter").expect("run");
    assert_eq!(page.links().len(), 3, "{:?}", page.links());
    page.advance(0.3).expect("settled");
    // Reload does the same, and says nothing of a browser that is not there.
    page.press("cmd+r").expect("reload");
    assert_eq!(page.links().len(), 4, "{:?}", page.links());
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_addresses_the_profile_has_been_to_are_offered_in_the_next_run() {
    let home = tempfile::tempdir().expect("a home");
    let stand = Stand::with(&[]);
    {
        let session = session_over(Arc::clone(&stand), History::open(home.path(), "test"), None);
        let mut page = open_with(&session, WIDE, None, true).expect("opens");
        write_address(&mut page, "3000");
        ready(&mut page, "tab-1", "", "");
        tell(&mut page, "tab-1", |state| {
            state.url = "http://localhost:3000/".to_owned();
            state.title = "Dashboard".to_owned();
        });
        page.wait_for(WITHIN, |page| page.says("Dashboard"))
            .expect("shown");
        page.advance(0.2).expect("kept");
    }
    let session = session_over(stand, History::open(home.path(), "test"), None);
    let page = open_with(&session, WIDE, None, true).expect("opens again");
    assert!(
        page.says("Recent") && page.says("Dashboard"),
        "{}",
        said(&page)
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_arrow_keys_go_through_the_ports_and_enter_goes_to_the_one_they_are_on() {
    let (_stand, _session, mut page) = open(WIDE);
    page.press("down").expect("down");
    page.press("down").expect("down");
    page.press("enter").expect("enter");
    ready(&mut page, "tab-1", "", "");
    assert!(
        went_to(&page, "tab-1", "http://localhost:5173"),
        "{:?}",
        asked_of(&page, "tab-1")
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn what_is_typed_narrows_the_ports_and_nothing_chosen_goes_to_what_was_typed() {
    let (_stand, _session, mut page) = open(WIDE);
    page.fill("address/field", "517").expect("typed");
    page.advance(0.1).expect("settled");
    assert!(page.says(":5173") && !page.says(":3000"), "{}", said(&page));
    page.press("enter").expect("enter");
    ready(&mut page, "tab-1", "", "");
    assert!(
        went_to(&page, "tab-1", "http://localhost:517"),
        "{:?}",
        asked_of(&page, "tab-1")
    );
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_list_of_commands_and_the_sheet_of_shortcuts_are_made_of_the_one_table() {
    let (_stand, _session, mut page) = at_a_page();
    key(&mut page, "f1");
    page.advance(0.3).expect("drawn");
    for title in [
        "Open Address",
        "Reload",
        "Zoom In",
        "Find",
        "Show All Commands",
    ] {
        assert!(page.says(title), "{title}: {}", said(&page));
    }
    // Back is not possible here, so it is not offered; nor is Stop with nothing loading.
    assert!(!page.says("Back") && !page.says("Stop"), "{}", said(&page));
    page.press("escape").expect("escape");
    key(&mut page, "cmd+?");
    page.advance(0.3).expect("drawn");
    assert!(
        page.says("Keyboard Shortcuts") && page.says("Go to Tab 1 to 8"),
        "{}",
        said(&page)
    );
}

/// The places of the bars: where each is, as the page says it.
fn places(page: &Page) -> Vec<(String, [f32; 4])> {
    // The slot at the head of the first tab is where its title starts: it keeps its place, empty or turning.
    [
        "back",
        "forward",
        "reload",
        "address-box",
        "new-tab",
        "tab-slot/tab-1",
    ]
    .iter()
    .map(|name| {
        let at = page
            .bounds(name)
            .unwrap_or_else(|| panic!("{name} is not on the page"));
        ((*name).to_owned(), [at.x, at.y, at.w, at.h])
    })
    .collect()
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn nothing_of_the_bars_moves_whatever_the_page_is_doing() {
    for size in [WIDE, NARROW] {
        let (stand, _session, mut page) = open(size);
        let blank = places(&page);
        let mut states = vec![("blank", blank.clone())];
        write_address(&mut page, "3000");
        ready(&mut page, "tab-1", "", "");
        tell(&mut page, "tab-1", |state| {
            state.url = "http://localhost:3000/".to_owned();
            state.title = "A page with a rather long title that does not fit".to_owned();
            state.loading = true;
        });
        states.push(("loading", places(&page)));
        tell(&mut page, "tab-1", |state| {
            state.url = "http://localhost:3000/a/long/path?with=a&query=string".to_owned();
            state.title = "Done".to_owned();
            state.back = true;
            state.forward = true;
        });
        for (tab, number) in [("tab-2", "5173"), ("tab-3", "3000")] {
            page.press("ctrl+t").expect("a tab");
            write_address(&mut page, number);
            ready(&mut page, tab, "", "");
        }
        states.push(("three tabs", places(&page)));
        key(&mut page, "cmd+f");
        page.type_text("x").expect("typed");
        states.push(("find bar", places(&page)));
        page.press("escape").expect("closed");
        tell(&mut page, "tab-3", |state| {
            state.url = "http://localhost:3000/".to_owned();
            state.zoom = 1.25;
        });
        states.push(("zoom 125%", places(&page)));
        tell(&mut page, "tab-3", |state| {
            state.url = "http://localhost:4000/".to_owned();
            state.error = Some(BrowserError {
                url: "http://localhost:4000/".to_owned(),
                code: -102,
                message: "Connection refused".to_owned(),
            });
        });
        page.wait_for(WITHIN, |page| page.says("Nothing is listening"))
            .expect("said");
        states.push(("nothing listening", places(&page)));
        stand.listen(4000, "dev");
        page.advance(1.6).expect("time");
        page.wait_for(WITHIN, |page| page.says("Something is listening"))
            .expect("said");
        states.push(("something listening", places(&page)));
        tell(&mut page, "tab-3", |state| {
            state.url = "https://nope.test/".to_owned();
            state.error = Some(BrowserError {
                url: "https://nope.test/".to_owned(),
                code: -105,
                message: "The server's name could not be found".to_owned(),
            });
        });
        states.push(("load error", places(&page)));
        page.browser_state(
            "tab-3",
            BrowserState::with_status(BrowserState::CRASHED, Some("The browser stopped.")),
        )
        .expect("told");
        states.push(("crashed", places(&page)));
        for (name, at) in &states {
            assert_eq!(at, &blank, "{name} at {size:?}: the bars moved");
        }
    }
}

/// Runs the command whose title has `words` from the palette, as a person does.
fn from_the_palette(page: &mut Page, words: &str) {
    key(page, "f1");
    page.advance(0.3).expect("drawn");
    page.type_text(words).expect("typed");
    page.advance(0.2).expect("settled");
    page.press("enter").expect("run");
    page.advance(0.2).expect("settled");
}

fn x_of(page: &Page, tab: &str) -> f32 {
    page.bounds(&format!("tabs/tab/{tab}")).expect("a tab").x
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn what_a_tabs_menu_and_the_drag_do_the_keyboard_does_too_and_the_menu_says_the_tables_words() {
    let (_stand, _session, mut page) = at_a_page();
    // The menu's words are the table's.
    page.press("ctrl+t").expect("a new tab");
    write_address(&mut page, "5173");
    ready(&mut page, "tab-2", "", "");
    page.right_click("tabs/tab/tab-1").expect("the menu");
    let titles: Vec<String> = page
        .menu()
        .expect("a menu")
        .entries
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item { title, .. } => Some(title.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        titles,
        [
            "Reload",
            "Duplicate Tab",
            "Copy Address",
            "Close Other Tabs",
            "Close Tabs to the Right"
        ]
    );
    page.press("escape").expect("closed");
    // Tab 2 is in front. Moving a tab left and right is the drag's: ⌃⇧PgUp and ⌃⇧PgDn.
    assert!(x_of(&page, "tab-1") < x_of(&page, "tab-2"));
    page.press("ctrl+shift+page_up").expect("left");
    assert!(x_of(&page, "tab-2") < x_of(&page, "tab-1"));
    page.press("ctrl+shift+page_down").expect("right");
    assert!(x_of(&page, "tab-1") < x_of(&page, "tab-2"));
    // Duplicate Tab, Close Other Tabs and Close Tabs to the Right are in the palette.
    from_the_palette(&mut page, "duplicate");
    ready(&mut page, "tab-3", "", "");
    assert!(
        went_to(&page, "tab-3", "http://localhost:5173/"),
        "{:?}",
        asks(&page)
    );
    // Tab 3 is the copy, after tab 2: close what is to the right of tab 1.
    page.press("ctrl+1").expect("tab one");
    from_the_palette(&mut page, "close tabs to the right");
    assert!(
        page.bounds("tabs/tab/tab-2").is_none() && page.bounds("tabs/tab/tab-3").is_none(),
        "{}",
        said(&page)
    );
    // And with only one tab left, there is nothing to move or to close.
    key(&mut page, "f1");
    page.advance(0.3).expect("drawn");
    assert!(
        !page.says("Move Tab") && !page.says("Close Other Tabs") && !page.says("to the Right"),
        "{}",
        said(&page)
    );
    page.press("escape").expect("closed");
    page.press("ctrl+t").expect("a new tab");
    write_address(&mut page, "8080");
    ready(&mut page, "tab-4", "", "");
    from_the_palette(&mut page, "close other");
    assert!(page.bounds("tabs/tab/tab-1").is_none() && page.bounds("tabs/tab/tab-4").is_some());
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_menu_of_a_tab_duplicates_it_and_a_tab_dragged_is_put_in_front_of_another() {
    let (_stand, _session, mut page) = at_a_page();
    page.right_click("tabs/tab/tab-1").expect("the menu");
    let menu = page.menu().expect("a menu").entries.len();
    assert_eq!(
        menu, 6,
        "Reload, Duplicate, Copy Address, a rule, and two to close"
    );
    page.pick_titled("Duplicate Tab").expect("duplicate");
    ready(&mut page, "tab-2", "", "");
    assert!(
        went_to(&page, "tab-2", "http://localhost:3000/"),
        "{:?}",
        asks(&page)
    );
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-2")));
    // The copy is the second tab, after the one it is of.
    let first = page.bounds("tabs/tab/tab-1").expect("tab 1").x;
    let second = page.bounds("tabs/tab/tab-2").expect("tab 2").x;
    assert!(first < second);
    // Copy Address is the pasteboard's.
    page.right_click("tabs/tab/tab-1").expect("the menu");
    page.pick_titled("Copy Address").expect("copied");
    assert_eq!(page.pasteboard(), Some("http://localhost:3000/"));
    // Dragged onto the other, the first goes behind it.
    tell(&mut page, "tab-2", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "Copy".to_owned();
    });
    page.drag("tabs/tab/tab-1", "tabs/tab/tab-2")
        .expect("dragged");
    let first = page.bounds("tabs/tab/tab-1").expect("tab 1").x;
    let second = page.bounds("tabs/tab/tab-2").expect("tab 2").x;
    assert!(second < first, "{second} {first}");
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_suggestions_under_the_address_field_of_a_page_are_chosen_by_the_arrows_and_by_a_click() {
    // The arrows and Enter.
    let (_stand, _session, mut page) = at_a_page();
    page.press("cmd+l").expect("address");
    page.fill("address/field", "51").expect("typed");
    page.advance(0.2).expect("settled");
    assert!(page.says(":5173") && !page.says(":3000"), "{}", said(&page));
    page.press("down").expect("down");
    page.press("enter").expect("enter");
    assert!(
        went_to(&page, "tab-1", "http://localhost:5173/"),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
    // A click.
    let (_stand, _session, mut page) = at_a_page();
    page.press("cmd+l").expect("address");
    page.fill("address/field", "51").expect("typed");
    page.advance(0.2).expect("settled");
    page.click("row/port:5173").expect("a click on the row");
    assert!(
        went_to(&page, "tab-1", "http://localhost:5173/"),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
    // And on a blank tab, where the rows stand in the room.
    let (_stand, _session, mut page) = open(WIDE);
    page.fill("address/field", "51").expect("typed");
    page.advance(0.2).expect("settled");
    page.click("row/port:5173").expect("a click on the row");
    ready(&mut page, "tab-1", "", "");
    assert!(
        went_to(&page, "tab-1", "http://localhost:5173/"),
        "{:?}",
        asked_of(&page, "tab-1")
    );
}

/// Where the middle of the element named `name` is, the host's points.
fn middle_of(page: &Page, name: &str) -> Point {
    let rect = page.bounds(name).expect("the element is on show");
    Point::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
}

/// The host takes the keyboard out of the field, and a frame is drawn for it: what a press on the
/// page does before the press itself is told (the headless click gives both at once).
fn the_keyboard_leaves_the_field(page: &mut Page) {
    page.page_mut().focus(None);
    page.settle().expect("settled");
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_click_on_a_suggestion_goes_there_before_or_after_the_keyboard_leaves_the_field() {
    // The keyboard leaves the field, a frame is drawn, and then the press is on the row.
    let (_stand, _session, mut page) = at_a_page();
    page.press("cmd+l").expect("address");
    page.fill("address/field", "51").expect("typed");
    page.advance(0.2).expect("settled");
    let row = middle_of(&page, "row/port:5173");
    the_keyboard_leaves_the_field(&mut page);
    assert!(
        page.says(":5173"),
        "the card waits for the press: {}",
        said(&page)
    );
    page.click_at(row).expect("a click where the row is");
    assert!(
        went_to(&page, "tab-1", "http://localhost:5173/"),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
    page.advance(0.1).expect("the field filled");
    assert!(!page.says(":5173"), "{}", said(&page));
    assert_eq!(page.field("address/field"), Some("localhost:5173"));

    // The same on a blank tab, where the rows stand in the room and what is not typed narrows them
    // the other way: the press is where the narrowed row was.
    let (_stand, _session, mut page) = open(WIDE);
    page.fill("address/field", "51").expect("typed");
    page.advance(0.2).expect("settled");
    let row = middle_of(&page, "row/port:5173");
    the_keyboard_leaves_the_field(&mut page);
    page.click_at(row).expect("a click where the row is");
    ready(&mut page, "tab-1", "", "");
    assert!(
        went_to(&page, "tab-1", "http://localhost:5173/"),
        "{:?}",
        asked_of(&page, "tab-1")
    );

    // The press first, and the keyboard leaves after it.
    let (_stand, _session, mut page) = at_a_page();
    page.press("cmd+l").expect("address");
    page.fill("address/field", "51").expect("typed");
    page.advance(0.2).expect("settled");
    page.click("row/port:5173").expect("a click on the row");
    the_keyboard_leaves_the_field(&mut page);
    assert!(
        went_to(&page, "tab-1", "http://localhost:5173/"),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    page.advance(0.1).expect("the field filled");
    assert!(!page.says(":5173"), "{}", said(&page));
    assert_eq!(page.field("address/field"), Some("localhost:5173"));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_suggestions_do_not_wait_when_the_page_itself_takes_the_keyboard_to_another_tab() {
    let (_stand, _session, mut page) = at_a_page();
    page.press("ctrl+t").expect("a new tab");
    write_address(&mut page, "5173");
    ready(&mut page, "tab-2", "", "");
    tell(&mut page, "tab-2", |state| {
        state.url = "http://localhost:5173/".to_owned();
        state.title = "Vite".to_owned();
    });
    page.press("cmd+l").expect("address");
    page.fill("address/field", "30").expect("typed");
    page.advance(0.2).expect("settled");
    assert!(page.says(":3000"), "{}", said(&page));
    // ⌃1 brings the first tab in front: what was written goes with the keyboard, and no press is coming.
    page.press("ctrl+1").expect("tab one");
    assert!(!page.says(":3000"), "{}", said(&page));
}

/// A page with tab 1 at localhost:3000, "51" written in the address field, its suggestions in a card, and the
/// keyboard let go of the field by the host (what a press on the page does before the press is told).
fn with_the_keyboard_let_go_of_the_field() -> (Arc<Stand>, Arc<Session>, Page) {
    let (stand, session, mut page) = at_a_page();
    page.press("cmd+l").expect("address");
    page.fill("address/field", "51").expect("typed");
    page.advance(0.2).expect("settled");
    the_keyboard_leaves_the_field(&mut page);
    assert!(page.says(":5173"), "the card waits: {}", said(&page));
    (stand, session, page)
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_suggestions_the_keyboard_let_go_of_stay_for_a_press_and_a_press_elsewhere_closes_them_at_once()
 {
    // What the browser says meanwhile (it loads, a title comes) does not close them.
    let (_stand, _session, mut page) = with_the_keyboard_let_go_of_the_field();
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "App again".to_owned();
    });
    assert!(page.says(":5173"), "{}", said(&page));

    // A press on the page's own parts that is not on a row: they close in the frame of the press.
    let (_stand, _session, mut page) = with_the_keyboard_let_go_of_the_field();
    page.click_at(Point::new(1050.0, 12.0))
        .expect("a click in the strip of tabs");
    assert!(!page.says(":5173"), "{}", said(&page));
    assert!(
        !went_to(&page, "tab-1", "http://localhost:5173/"),
        "{:?}",
        asked_of(&page, "tab-1")
    );
    page.advance(0.1).expect("the field filled");
    assert_eq!(page.field("address/field"), Some("localhost:3000"));

    // A press on the browser's room is the browser's and the page is told only that the keyboard is its.
    let (_stand, _session, mut page) = with_the_keyboard_let_go_of_the_field();
    page.click_at(Point::new(700.0, 600.0))
        .expect("a click in the browser");
    assert!(!page.says(":5173"), "{}", said(&page));
    assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
    page.advance(0.1).expect("the field filled");
    assert_eq!(page.field("address/field"), Some("localhost:3000"));

    // The keyboard going to a browser by itself does the same.
    let (_stand, _session, mut page) = with_the_keyboard_let_go_of_the_field();
    page.page_mut().focus(Some(Id::named("tab-1")));
    page.settle().expect("settled");
    assert!(!page.says(":5173"), "{}", said(&page));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn esc_and_enter_and_a_change_of_tab_close_the_suggestions_the_keyboard_let_go_of() {
    // The keyboard is the page's now, so the key is told to the page (the headless host still holds the
    // field it let go of as the keyboard's). Esc ends what was written; Enter, as in the field, goes to it.
    for key in [Key::Escape, Key::Enter] {
        let (_stand, _session, mut page) = with_the_keyboard_let_go_of_the_field();
        page.page_mut().key(key, Mods::default());
        page.settle().expect("settled");
        assert!(!page.says(":5173"), "{key:?}: {}", said(&page));
        assert_eq!(keyboard(&page), format!("{:?}", Id::named("tab-1")));
        assert!(
            !went_to(&page, "tab-1", "http://localhost:5173/"),
            "{key:?}: {:?}",
            asked_of(&page, "tab-1")
        );
        if key == Key::Escape {
            page.advance(0.1).expect("the field filled");
            assert_eq!(page.field("address/field"), Some("localhost:3000"));
        }
    }
    // A change of tab ends it too.
    let (_stand, _session, mut page) = at_a_page();
    page.press("ctrl+t").expect("a new tab");
    write_address(&mut page, "5173");
    ready(&mut page, "tab-2", "", "");
    tell(&mut page, "tab-2", |state| {
        state.url = "http://localhost:5173/".to_owned();
        state.title = "Vite".to_owned();
    });
    page.press("cmd+l").expect("address");
    page.fill("address/field", "30").expect("typed");
    page.advance(0.2).expect("settled");
    the_keyboard_leaves_the_field(&mut page);
    assert!(page.says(":3000"), "{}", said(&page));
    page.press("ctrl+1").expect("tab one");
    assert!(!page.says(":3000"), "{}", said(&page));
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn on_a_keyboard_that_is_not_a_macs_control_is_what_command_is() {
    let stand = Stand::with(&[(3000, "node")]);
    let session = session_over(stand, History::memory(), Some("3000"));
    let mut page =
        open_keyed(&session, WIDE, None, true, Some(KeyConvention::Linux)).expect("opens");
    ready(&mut page, "tab-1", "", "");
    tell(&mut page, "tab-1", |state| {
        state.url = "http://localhost:3000/".to_owned();
        state.title = "App".to_owned();
    });
    page.advance(0.2).expect("settled");
    page.press("ctrl+l").expect("address");
    assert!(
        matches!(page.focus(), Focus::Field(_)),
        "{:?}",
        page.focus()
    );
    page.press("ctrl+t").expect("a new tab");
    assert!(page.says("Ports"), "{}", said(&page));
    page.press("ctrl+1").expect("tab one");
    page.press("ctrl+w").expect("close");
    page.press("ctrl+shift+t").expect("reopen");
    ready(&mut page, "tab-3", "", "");
    assert!(
        went_to(&page, "tab-3", "http://localhost:3000/"),
        "{:?}",
        asks(&page)
    );
}

/// No key of the table is a shift with a mark or a digit (`rules/penler.md`: the host gives the page the
/// character the shift made, which the kit does not match to a written `shift+.`): `cmd+?` is the way.
#[test]
fn no_key_of_the_command_table_is_a_shift_with_a_mark_or_a_digit() {
    let table = include_str!("../view/parts/commands.lua");
    for line in table.lines().filter(|line| line.contains("keys = \"")) {
        let keys = line
            .split("keys = \"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or("");
        let last = keys.rsplit('+').next().unwrap_or("");
        let shifted = keys.split('+').any(|part| part == "shift");
        let plain = last.chars().count() == 1 && last.chars().all(|c| c.is_ascii_lowercase());
        assert!(
            !shifted || plain || last.len() > 1,
            "{keys}: a shift with a mark or a digit"
        );
        assert!(
            !shifted || plain || last.chars().all(|c| c.is_ascii_alphabetic() || c == '_'),
            "{keys}"
        );
    }
}

/// What a page asks of its browsers in the reply to a timer reaches the host as it does in the reply to a
/// key (pencere `b452c9d`): the page goes to a server it found listening (`browse.lua`, `watch`) by it.
#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_browsers_ask_in_the_reply_to_a_timer_reaches_the_host_as_one_to_a_key_does() {
    use pencere_view::{Key, Mods};
    let source = r#"
return {
	render = function()
		return pencere.div():child(pencere.browser("web"))
	end,
	handle = function(event)
		if event.type == "key" and event.key == "y" then
			pencere.time.after(0.1, function()
				return { browser = { web = { go = "https://timer.example/" } } }
			end)
			return true
		elseif event.type == "key" then
			return { browser = { web = { go = "https://key.example/" } } }
		end
	end,
}
"#;
    let mut page = Headless::script(source)
        .size(800.0, 600.0)
        .open()
        .expect("opens");
    page.key(Key::Character('y'), Mods::default())
        .expect("arms");
    page.advance(0.5).expect("the timer runs");
    assert_eq!(
        page.browser_asks()
            .iter()
            .map(|(_, ask)| ask.clone())
            .collect::<Vec<_>>(),
        [BrowserAsk::Go("https://timer.example/".to_owned())]
    );
    page.key(Key::Character('x'), Mods::default()).expect("key");
    assert_eq!(page.browser_asks().len(), 2, "{:?}", page.browser_asks());
}

/// Two asks of a browser put into one reply (`kit.gather`) go once each and in the order they were
/// decided, and so do the asks of two browsers: the kit's `merge` keeps them all, and the page puts none
/// together by hand on top of it.
#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn asks_gathered_into_one_reply_go_once_each_and_in_order() {
    use pencere_drive::headless::Files;
    use pencere_view::{Key, Mods};
    let dir = tempfile::tempdir().expect("a directory");
    std::fs::create_dir_all(dir.path().join("lib")).expect("lib");
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/view/lib/kit.lua"),
        dir.path().join("lib/kit.lua"),
    )
    .expect("the page's kit module");
    std::fs::write(
        dir.path().join("page.lua"),
        r#"
local kit = require("./lib/kit")
local b = kit.b
return {
	render = function()
		return b.div():child(b.browser("web")):child(b.browser("other"))
	end,
	handle = function(event)
		if event.type == "key" then
			kit.gather({ redraw = true, browser = { web = { { go = "https://a.test/" } } } })
			kit.gather({ redraw = true, browser = { web = { { command = "reload", cache = true } } } })
			kit.gather({ redraw = true, browser = { other = { { zoom = 1.5 } } } })
			kit.gather({ redraw = true, browser = { web = { zoom = 2 } } })
			return kit.take(true)
		end
	end,
}
"#,
    )
    .expect("the test page");
    let mut page = Headless::bundle(Files::dir(dir.path()), "page.lua")
        .size(800.0, 600.0)
        .open()
        .expect("opens");
    page.key(Key::Character('x'), Mods::default()).expect("key");
    let of = |name: &str| -> Vec<BrowserAsk> {
        page.browser_asks()
            .iter()
            .filter(|(id, _)| *id == Id::named(name))
            .map(|(_, ask)| ask.clone())
            .collect()
    };
    assert_eq!(
        of("web"),
        [
            BrowserAsk::Go("https://a.test/".to_owned()),
            BrowserAsk::Reload { cache: true },
            BrowserAsk::Zoom(2.0),
        ]
    );
    assert_eq!(of("other"), [BrowserAsk::Zoom(1.5)]);
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn the_palette_opened_again_starts_empty_whatever_was_written_in_it_before() {
    let (_stand, _session, mut page) = at_a_page();
    for round in 0..3 {
        key(&mut page, "f1");
        page.advance(0.3).expect("drawn");
        assert_eq!(
            page.field("palette/input/field").unwrap_or(""),
            "",
            "round {round}"
        );
        page.type_text("dup").expect("typed");
        page.advance(0.2).expect("settled");
        // The match is drawn in two pieces ("Dup", "licate Tab"); nothing else is found by it.
        assert!(
            page.says("licate Tab") && !page.says("Reload"),
            "round {round}: {}",
            said(&page)
        );
        page.press("escape").expect("closed");
        page.advance(0.2).expect("settled");
    }
}

#[test]
#[ignore = "opens the kit's interpreter: make test-page"]
fn a_click_in_the_address_field_gives_the_whole_address_selected_as_the_key_for_it_does() {
    let (_stand, _session, mut page) = at_a_page();
    assert_eq!(page.field("address/field"), Some("localhost:3000"));
    page.click("address/field").expect("a click in the field");
    page.advance(0.1).expect("filled");
    assert_eq!(page.field("address/field"), Some("http://localhost:3000/"));
    assert_eq!(page.field_selection("address/field"), Some(0..22));
}
