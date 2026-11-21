//! What a run of the program holds for the page: the profile, the address the command line asked for,
//! the machine's ports and the profile's recent addresses.

use std::sync::Arc;

use pencere::ViewControl;

use crate::machine::Machine;
use crate::state::history::History;

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
        }
    }
}
