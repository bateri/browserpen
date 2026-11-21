//! What a run of the program holds for the page: the profile, the address the command line asked for,
//! the machine's ports, the profile's recent addresses and the tabs the page has open.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pencere::ViewControl;

use crate::machine::Machine;
use crate::state::history::History;
use crate::state::tabs::Tabs;

pub struct Session {
    /// The name the pages' sites are kept under.
    pub profile: String,
    /// The address the command line gave, as it was typed.
    pub start: Option<String>,
    /// The ports of the machine the program runs on, and whether one answers.
    pub machine: Arc<dyn Machine>,
    /// The addresses the profile has been to.
    pub history: History,
    /// The program's hand on the view, to end it when the person quits.
    pub view: ViewControl,
    /// The tabs the page has open, for a view that starts again.
    pub tabs: Tabs,
    /// Whether a page has been told what the program is for yet (`take_start`).
    started: AtomicBool,
}

impl Session {
    #[must_use]
    pub fn new(
        profile: &str,
        start: Option<String>,
        machine: Arc<dyn Machine>,
        history: History,
        view: ViewControl,
    ) -> Self {
        Self {
            profile: profile.to_owned(),
            start,
            machine,
            history,
            view,
            tabs: Tabs::default(),
            started: AtomicBool::new(false),
        }
    }

    /// The address the command line gave, for the first view of the program only: a view that starts
    /// again is not the command line's to send to the address again, it has the tabs it had.
    pub fn take_start(&self) -> Option<String> {
        if self.started.swap(true, Ordering::SeqCst) {
            None
        } else {
            self.start.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::System;

    #[test]
    fn the_address_of_the_command_line_is_given_to_the_first_view_only() {
        let session = Session::new(
            "test",
            Some("3000".to_owned()),
            Arc::new(System),
            History::memory(),
            ViewControl::new(),
        );
        assert_eq!(session.take_start().as_deref(), Some("3000"));
        assert_eq!(session.take_start(), None);
        assert_eq!(session.start.as_deref(), Some("3000"));
    }
}
