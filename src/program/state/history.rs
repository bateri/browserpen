//! The addresses a profile has been to, most recent first, on the machine the program runs on (the
//! machine whose `localhost` they are): at most [`MOST`], each address once. A file under the state
//! directory (`$BROWSERPEN_STATE_DIR`, `$XDG_STATE_HOME/browserpen`, `~/.local/state/browserpen`), one
//! folder for each profile. Several runs may share a profile (two panes): each address is written into
//! what the file holds at that moment, not over what the run read when it began. A file that is not a
//! history is read as none, and the first address a run keeps replaces it; the list is kept in memory
//! for the run, and stays where the file cannot be written.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

/// The most addresses kept.
pub const MOST: usize = 200;

/// The longest title kept, in characters.
const TITLE_CHARS: usize = 200;

/// One address the profile has been to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visit {
    pub url: String,
    pub title: String,
}

/// An address as the file has it: the file's own shape, which is not the page's.
#[derive(Serialize, Deserialize)]
struct Stored {
    url: String,
    title: String,
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    visits: Vec<Stored>,
}

impl From<Stored> for Visit {
    fn from(stored: Stored) -> Self {
        Self {
            url: stored.url,
            title: stored.title,
        }
    }
}

impl From<&Visit> for Stored {
    fn from(visit: &Visit) -> Self {
        Self {
            url: visit.url.clone(),
            title: visit.title.clone(),
        }
    }
}

/// What the file at `path` holds: none where it is not there or is not a history.
fn read(path: &Path) -> Option<Vec<Visit>> {
    let bytes = std::fs::read(path).ok()?;
    let file = serde_json::from_slice::<File>(&bytes).ok()?;
    Some(file.visits.into_iter().map(Visit::from).collect())
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
        let visits = read(&path).unwrap_or_default();
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

    /// The person was at `url`, titled `title`: it is the most recent now, and once. It goes into what
    /// the file holds now, which another run of the same profile may have added to since this one read
    /// it, and what this run holds that the file does not (a write that failed) stays behind.
    pub fn record(&self, url: &str, title: &str) {
        let title: String = title.chars().take(TITLE_CHARS).collect();
        let mut visits = self.visits.lock().unwrap_or_else(PoisonError::into_inner);
        let mut merged = vec![Visit {
            url: url.to_owned(),
            title,
        }];
        let on_file = self.path.as_deref().and_then(read).unwrap_or_default();
        for visit in on_file.into_iter().chain(visits.iter().cloned()) {
            if merged.iter().all(|kept| kept.url != visit.url) {
                merged.push(visit);
            }
        }
        merged.truncate(MOST);
        *visits = merged;
        if let Some(path) = &self.path {
            // A history that cannot be written is the person's loss of a suggestion, not of a page.
            let _ = write(path, &visits);
        }
    }
}

/// The file, written whole beside itself (under a name of this run's own, for another run of the
/// profile writes beside it at the same time) and moved in place, so that a reader never sees half.
fn write(path: &Path, visits: &[Visit]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = File {
        visits: visits.iter().map(Stored::from).collect(),
    };
    let bytes = serde_json::to_vec(&file).map_err(std::io::Error::other)?;
    let temporary = temporary_of(path);
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

/// Where this run writes before it moves the file: the process's number is in the name.
fn temporary_of(path: &Path) -> PathBuf {
    path.with_extension(format!("json.{}.tmp", std::process::id()))
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

    #[test]
    fn two_runs_of_one_profile_keep_each_others_addresses() {
        let home = tempfile::tempdir().unwrap();
        // Two panes: both read the file before either wrote.
        let first = History::open(home.path(), "p");
        let second = History::open(home.path(), "p");
        first.record("http://localhost:3000/", "One");
        second.record("http://localhost:4000/", "Two");
        first.record("http://localhost:5000/", "Three");
        let urls = |history: &History| -> Vec<String> {
            history.list().into_iter().map(|visit| visit.url).collect()
        };
        let expected = [
            "http://localhost:5000/",
            "http://localhost:4000/",
            "http://localhost:3000/",
        ];
        assert_eq!(urls(&first), expected);
        assert_eq!(urls(&History::open(home.path(), "p")), expected);
    }

    #[test]
    fn what_a_run_could_not_write_is_kept_for_the_next_write() {
        let home = tempfile::tempdir().unwrap();
        let history = History::open(home.path(), "p");
        // The profile's folder is a file: nothing can be written under it.
        std::fs::write(home.path().join("p"), "in the way").unwrap();
        history.record("http://localhost:1/", "");
        std::fs::remove_file(home.path().join("p")).unwrap();
        history.record("http://localhost:2/", "");
        let again = History::open(home.path(), "p").list();
        assert_eq!(again.len(), 2);
        assert_eq!(again[0].url, "http://localhost:2/");
        assert_eq!(again[1].url, "http://localhost:1/");
    }

    #[test]
    fn the_file_is_written_beside_itself_under_a_name_with_the_process_in_it_and_none_is_left() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("p/recent.json");
        assert!(
            temporary_of(&path)
                .to_string_lossy()
                .contains(&std::process::id().to_string())
        );
        History::open(home.path(), "p").record("http://localhost:1/", "");
        let left: Vec<_> = std::fs::read_dir(home.path().join("p"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, ["recent.json"]);
    }
}
