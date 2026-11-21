//! The addresses a profile has been to, most recent first, on the machine the program runs on (the
//! machine whose `localhost` they are): at most [`MOST`], each address once. A file under the state
//! directory (`$BROWSERPEN_STATE_DIR`, `$XDG_STATE_HOME/browserpen`, `~/.local/state/browserpen`), one
//! folder for each profile. A file that is not read is left as it is, and the list is kept in memory
//! for the run.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

/// The most addresses kept.
pub const MOST: usize = 200;

/// The longest title kept, in characters.
const TITLE_CHARS: usize = 200;

/// One address the profile has been to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Visit {
    pub url: String,
    pub title: String,
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    visits: Vec<Visit>,
}

/// The state directory of the program, where the environment says.
#[must_use]
pub fn state_dir() -> Option<PathBuf> {
    let from = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if let Some(dir) = from("BROWSERPEN_STATE_DIR") {
        return Some(PathBuf::from(dir));
    }
    if let Some(dir) = from("XDG_STATE_HOME") {
        return Some(PathBuf::from(dir).join("browserpen"));
    }
    from("HOME").map(|home| PathBuf::from(home).join(".local/state/browserpen"))
}

pub struct History {
    path: Option<PathBuf>,
    visits: Mutex<Vec<Visit>>,
}

impl History {
    /// The history of `profile` under `dir`, as the file has it (none, where the file is not there or
    /// is not one).
    #[must_use]
    pub fn open(dir: &Path, profile: &str) -> Self {
        let path = dir.join(profile).join("recent.json");
        let visits = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<File>(&bytes).ok())
            .map(|file| file.visits)
            .unwrap_or_default();
        Self {
            path: Some(path),
            visits: Mutex::new(visits),
        }
    }

    /// A history that is kept in memory only, as the tests have it.
    #[must_use]
    pub fn memory() -> Self {
        Self {
            path: None,
            visits: Mutex::new(Vec::new()),
        }
    }

    /// The addresses, most recent first.
    pub fn list(&self) -> Vec<Visit> {
        self.visits
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The person was at `url`, titled `title`: it is the most recent now, and once.
    pub fn record(&self, url: &str, title: &str) {
        let title: String = title.chars().take(TITLE_CHARS).collect();
        let mut visits = self.visits.lock().unwrap_or_else(PoisonError::into_inner);
        visits.retain(|visit| visit.url != url);
        visits.insert(
            0,
            Visit {
                url: url.to_owned(),
                title,
            },
        );
        visits.truncate(MOST);
        if let Some(path) = &self.path {
            // A history that cannot be written is the person's loss of a suggestion, not of a page.
            let _ = write(
                path,
                &File {
                    visits: visits.clone(),
                },
            );
        }
    }
}

/// The file, written whole beside itself and moved in place, so that a reader never sees half.
fn write(path: &Path, file: &File) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec(file).map_err(std::io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_latest_is_first_and_an_address_is_there_once_with_its_latest_title() {
        let history = History::memory();
        history.record("http://localhost:3000/", "App");
        history.record("http://localhost:5173/", "Vite");
        history.record("http://localhost:3000/", "App, again");
        let list = history.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].url, "http://localhost:3000/");
        assert_eq!(list[0].title, "App, again");
        assert_eq!(list[1].url, "http://localhost:5173/");
    }

    #[test]
    fn no_more_than_the_most_are_kept_and_the_oldest_go() {
        let history = History::memory();
        for number in 0..MOST + 20 {
            history.record(&format!("http://localhost:{number}/"), "");
        }
        let list = history.list();
        assert_eq!(list.len(), MOST);
        assert_eq!(list[0].url, format!("http://localhost:{}/", MOST + 19));
        assert_eq!(list[MOST - 1].url, "http://localhost:20/");
    }

    #[test]
    fn a_profile_keeps_its_own_file_and_a_run_reads_what_the_one_before_wrote() {
        let home = tempfile::tempdir().unwrap();
        History::open(home.path(), "work").record("http://localhost:3000/", "App");
        History::open(home.path(), "other").record("http://localhost:4000/", "");
        let again = History::open(home.path(), "work").list();
        assert_eq!(
            again,
            [Visit {
                url: "http://localhost:3000/".to_owned(),
                title: "App".to_owned()
            }]
        );
    }

    #[test]
    fn a_file_that_is_not_a_history_is_read_as_none_and_the_run_still_records() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("p")).unwrap();
        std::fs::write(home.path().join("p/recent.json"), "not json").unwrap();
        let history = History::open(home.path(), "p");
        assert!(history.list().is_empty());
        history.record("http://localhost:1/", "");
        assert_eq!(history.list().len(), 1);
    }
}
