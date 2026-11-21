//! Everything the page talks to the program about: the calls of `docs/calls.md`. It knows the page;
//! `machine/` and `state/` do not, and what the page is told is made here from what they hold.
//!
//! A call's params are read before it runs (what does not fit is `bad-params`, and the work never
//! runs). A call that panics is `internal`, by pencere.

mod params;
mod session;

use std::sync::Arc;

use pencere::{Call, Options, ViewError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::session::Session;

/// The work of a call: the session and the params it read.
type Work<P, T> = fn(&Session, P) -> T;

/// Answers the calls of the page to `name` with `work`, given the params as `P` reads them. Calls
/// run at the same time as each other.
fn call<P: DeserializeOwned + 'static, T: Serialize + 'static>(
    options: Options,
    session: &Arc<Session>,
    name: &'static str,
    work: Work<P, T>,
) -> Options {
    let session = Arc::clone(session);
    options.call(name, move |params: Value, _: &Call| {
        let params =
            params::parse::<P>(params).map_err(|why| ViewError::with_message("bad-params", why))?;
        Ok(work(&session, params))
    })
}

/// The calls the page may use, answered from `session`.
pub fn serve(options: Options, session: &Arc<Session>) -> Options {
    session::calls(options, session)
}
