# The program

browserpen's program is small: it looks at the machine the page is about and remembers what the page
asks it to; the browsing itself is the host's browsers' and the page's (`src/view/`).

```
main.rs         the entry: reads the command line, makes the session, gives the calls to `serve`
command_line.rs `browserpen [URL] [--profile NAME]`
session.rs      what the calls share: the profile, the address asked for, the machine, the history
icons.rs        the app's icons, carried in the file
api/            everything the page talks to the program about (docs/calls.md); it knows the page
machine/        the machine's listening ports and whether one answers; knows nothing of the page
state/          the addresses a profile has been to; same
```

`api/` is the only place that builds a call's answer; `machine/` and `state/` know nothing of the page
or of `serde`'s wire shapes, so each can be tested alone (their unit tests) and the page's tests
(`page_tests.rs`) run the page over a stand-in `Machine`, never the machine's own ports.

The page's tests and the pictures are `page_tests.rs` and `page_shots.rs`: the page (`src/view/`) run
as a terminal runs it, on pencere's host with no screen (`pencere-drive`), with this program's handlers
in the same process. `make test-page` runs the tests, `make shots` writes the pictures to
`target/shots/`.
