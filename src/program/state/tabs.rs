//! The tabs the page has open, as the page last said them, kept in memory for as long as the program
//! runs. The page lives in a view, and a view can start again while the program goes on (the terminal
//! made it again, the connection to a server came back): the page begins from nothing then, and asks the
//! program what it had open. Nothing is written to a disk: when the program ends the tabs end with it.

use std::sync::{Mutex, PoisonError};

/// The most tabs kept: what the page says beyond them is dropped, the last ones first.
pub const MOST: usize = 200;

/// The longest address kept, in bytes: a longer one is none (the tab comes back blank), for it cannot be cut and
/// stay the address. With [`TITLE_CHARS`] and [`MOST`] it keeps what the program says of the tabs well under
/// the kit's 1 MiB for a message.
pub const URL_BYTES: usize = 2048;

/// The longest title kept, in characters.
pub const TITLE_CHARS: usize = 200;

/// One tab: its name (the host's browser is named by it), where it was, the title of the page there,
/// and how large it was shown.
#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    pub key: String,
    /// Where the tab was; none for a tab that has had no address yet.
    pub url: Option<String>,
    pub title: String,
    /// The zoom of the page where it was not its own size.
    pub zoom: Option<f64>,
}

/// The tabs in the order of the strip, the one in front, and the number the next tab is named by.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TabSet {
    pub tabs: Vec<Tab>,
    pub active: Option<String>,
    pub serial: u64,
}

/// What the page last said, for a page that starts again.
#[derive(Default)]
pub struct Tabs {
    kept: Mutex<Option<TabSet>>,
}

impl Tabs {
    /// Keeps `set` in place of what was kept. A set with no tab is no set: a page that has none has
    /// nothing to be given back.
    pub fn keep(&self, mut set: TabSet) {
        set.tabs.truncate(MOST);
        for tab in &mut set.tabs {
            if tab.url.as_ref().is_some_and(|url| url.len() > URL_BYTES) {
                tab.url = None;
            }
            if tab.title.chars().count() > TITLE_CHARS {
                tab.title = tab.title.chars().take(TITLE_CHARS).collect();
            }
        }
        let kept = if set.tabs.is_empty() { None } else { Some(set) };
        *self.kept.lock().unwrap_or_else(PoisonError::into_inner) = kept;
    }

    /// What the page last said, or none.
    pub fn kept(&self) -> Option<TabSet> {
        self.kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(key: &str, url: Option<&str>) -> Tab {
        Tab {
            key: key.to_owned(),
            url: url.map(str::to_owned),
            title: String::new(),
            zoom: None,
        }
    }

    #[test]
    fn nothing_is_kept_before_the_page_says_anything() {
        assert_eq!(Tabs::default().kept(), None);
    }

    #[test]
    fn what_the_page_said_last_is_what_is_given_back() {
        let tabs = Tabs::default();
        let first = TabSet {
            tabs: vec![tab("tab-1", Some("http://localhost:3000/"))],
            active: Some("tab-1".to_owned()),
            serial: 1,
        };
        tabs.keep(first);
        let second = TabSet {
            tabs: vec![tab("tab-2", None), tab("tab-1", Some("http://a.test/"))],
            active: Some("tab-2".to_owned()),
            serial: 2,
        };
        tabs.keep(second.clone());
        assert_eq!(tabs.kept(), Some(second));
    }

    #[test]
    fn a_page_with_no_tab_leaves_nothing_to_give_back() {
        let tabs = Tabs::default();
        tabs.keep(TabSet {
            tabs: vec![tab("tab-1", None)],
            active: None,
            serial: 1,
        });
        tabs.keep(TabSet::default());
        assert_eq!(tabs.kept(), None);
    }

    #[test]
    fn a_title_is_cut_and_an_address_too_long_to_be_kept_is_none() {
        let tabs = Tabs::default();
        let mut long = tab("tab-1", Some("http://a.test/"));
        long.title = "é".repeat(TITLE_CHARS + 50);
        let far = tab(
            "tab-2",
            Some(&format!("http://a.test/{}", "a".repeat(URL_BYTES))),
        );
        tabs.keep(TabSet {
            tabs: vec![long, far],
            active: None,
            serial: 2,
        });
        let kept = tabs.kept().expect("kept");
        assert_eq!(kept.tabs[0].title.chars().count(), TITLE_CHARS);
        assert_eq!(kept.tabs[0].url.as_deref(), Some("http://a.test/"));
        assert_eq!(kept.tabs[1].url, None);
    }

    #[test]
    fn no_more_than_the_most_are_kept() {
        let tabs = Tabs::default();
        let many = (0..MOST + 5)
            .map(|number| tab(&format!("tab-{number}"), None))
            .collect();
        tabs.keep(TabSet {
            tabs: many,
            active: None,
            serial: 0,
        });
        assert_eq!(tabs.kept().map(|set| set.tabs.len()), Some(MOST));
    }
}
