//! The command line: `browserpen [URL] [--profile NAME]`.

/// The profile a run has where the command line names none: what its sites keep (cookies, storage)
/// stays under it from one run to the next.
pub const DEFAULT_PROFILE: &str = "default";

/// The most characters of a profile's name (the kit's own limit).
const PROFILE_CHARS: usize = 32;

const USAGE: &str = "\
browserpen: a web browser for the terminal pane

usage: browserpen [URL] [--profile <name>]

  URL                  the address to open: localhost:3000, :3000, 3000 or a web address
  --profile <name>     what the sites keep (cookies, storage) is kept apart under this name,
                       1 to 32 of a to z, 0 to 9 and - (default: default)
  -V, --version        print the version
  -h, --help           print this

In a terminal that shows pencere views (bateri) browserpen opens in the pane, and the pages it
shows are those of the machine the program runs on: localhost is that machine's.
";

/// What a run is asked to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The address as it was typed: the page makes an address of it, as it does of the person's.
    pub url: Option<String>,
    pub profile: String,
}

/// What the command line comes to.
#[derive(Debug, PartialEq, Eq)]
pub enum Asked {
    Run(Line),
    /// Words for the person, on stdout; the program ends with 0.
    Say(String),
    /// Why the command line is not understood, on stderr; the program ends with 2.
    Refuse(String),
}

/// Whether `name` is a profile's name the kit takes: 1 to 32 of `a` to `z`, `0` to `9` and `-`.
#[must_use]
pub fn profile_ok(name: &str) -> bool {
    (1..=PROFILE_CHARS).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn profile_words(name: &str) -> Result<String, String> {
    if profile_ok(name) {
        Ok(name.to_owned())
    } else {
        Err(format!(
            "browserpen: a profile is 1 to {PROFILE_CHARS} of a to z, 0 to 9 and -: {name:?}"
        ))
    }
}

#[must_use]
pub fn read(args: impl IntoIterator<Item = String>) -> Asked {
    let mut line = Line {
        url: None,
        profile: DEFAULT_PROFILE.to_owned(),
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Asked::Say(USAGE.to_owned()),
            "-V" | "--version" => {
                return Asked::Say(format!("browserpen {}\n", env!("CARGO_PKG_VERSION")));
            }
            "--profile" => {
                let Some(name) = args.next() else {
                    return Asked::Refuse("browserpen: --profile takes a name".to_owned());
                };
                match profile_words(&name) {
                    Ok(name) => line.profile = name,
                    Err(why) => return Asked::Refuse(why),
                }
            }
            other => {
                if let Some(name) = other.strip_prefix("--profile=") {
                    match profile_words(name) {
                        Ok(name) => line.profile = name,
                        Err(why) => return Asked::Refuse(why),
                    }
                } else if other.starts_with('-') && other.len() > 1 {
                    return Asked::Refuse(format!(
                        "browserpen: {other:?} is not understood (browserpen --help says what is)"
                    ));
                } else if line.url.is_some() {
                    return Asked::Refuse(
                        "browserpen: one address at a time (browserpen --help says how)".to_owned(),
                    );
                } else {
                    line.url = Some(other.to_owned());
                }
            }
        }
    }
    Asked::Run(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_words(words: &[&str]) -> Asked {
        read(words.iter().map(|word| (*word).to_owned()))
    }

    #[test]
    fn no_arguments_open_the_default_profile_on_a_blank_tab() {
        assert_eq!(
            read_words(&[]),
            Asked::Run(Line {
                url: None,
                profile: "default".to_owned()
            })
        );
    }

    #[test]
    fn an_address_and_a_profile_are_read_in_either_order() {
        let want = Asked::Run(Line {
            url: Some("localhost:3000".to_owned()),
            profile: "work".to_owned(),
        });
        assert_eq!(read_words(&["localhost:3000", "--profile", "work"]), want);
        assert_eq!(read_words(&["--profile=work", "localhost:3000"]), want);
    }

    #[test]
    fn a_profile_the_kit_would_refuse_is_refused_here_with_the_rule() {
        for name in ["", "Work", "a b", "under_score", &"a".repeat(33)] {
            let Asked::Refuse(why) = read_words(&["--profile", name]) else {
                panic!("a refusal: {name:?}");
            };
            assert!(why.contains("1 to 32"), "{why}");
        }
        assert!(matches!(read_words(&["--profile"]), Asked::Refuse(_)));
        assert!(profile_ok(&"a".repeat(32)));
        assert!(profile_ok("a-1"));
    }

    #[test]
    fn what_is_not_understood_is_refused_and_help_and_version_are_said() {
        assert!(matches!(read_words(&["--tabs"]), Asked::Refuse(why) if why.contains("--tabs")));
        assert!(matches!(read_words(&["a", "b"]), Asked::Refuse(_)));
        assert!(matches!(read_words(&["--help"]), Asked::Say(text) if text.contains("--profile")));
        assert!(
            matches!(read_words(&["-V"]), Asked::Say(text) if text.starts_with("browserpen 0."))
        );
    }
}
