# browserpen

A web browser for the terminal pane: tabs, an address bar, Back, Forward and Reload, find in page and
zoom, drawn by the terminal in its own font and colours. It runs on the machine it is started on and
the pages it opens load from that machine, so over plain `ssh` `localhost:3000` is the server's own
dev server, with no port to forward.

## Install

browserpen is one file, with nothing to install beside it, for macOS on Apple silicon and Linux on
arm64 and x86_64:

```sh
curl -fsSL https://raw.githubusercontent.com/bateri/browserpen/main/install.sh | sh
```

A terminal that hosts pencere views ([bateri](https://github.com/bateri/bateri)) shows the browser in
its pane. A terminal that speaks Kitty's graphics protocol (Ghostty, kitty) shows the tabs and the
address bar as pictures, and an address opens in your own browser. `PENCERE_TUI=1` asks for that in any
terminal.

The installer takes the newest release (or `BROWSERPEN_VERSION=0.1.0`), checks it against the release's
`SHA256SUMS`, and puts it in `~/.local/bin` (`BROWSERPEN_INSTALL=<dir>` puts it elsewhere). A file a
browser downloads is quarantined by macOS; one curl downloads is not.

## Use

browserpen is a [pencere](https://github.com/bateri/pencere) app: the program is Rust, the view is a
page of pencere's kit, and both are one file.

```
browserpen                       # a blank tab: the ports this machine listens on, and where you have been
browserpen localhost:3000        # open an address (3000 and :3000 are localhost:3000)
browserpen --profile work        # what the sites keep (cookies, storage) is kept apart under a name
```

What you type in the address bar is an address, a port or a search: `3000` is `localhost:3000`,
`localhost`, `127.0.0.1`, `[::1]` and a name with a port and no dot (`myhost:8080`) are `http`, a name
with a dot and no space is `https`, and anything else is a search.

A blank tab (`⌃T`) lists the TCP ports the programs you started listen on (on a Mac, not the apps'),
with the program's name, and the addresses you have been to (kept per profile, on the machine the program runs on). The arrow keys go
through them and Enter goes to the one chosen. A page that did not load says why; where the address is
a port nothing listens on, it says so, and says when something does.

### Keys

Every key is on the screen: `F1` lists the commands, `⌘?` the keys.

| | |
|---|---|
| `⌘L` | the address bar |
| `⌃T` `⌃W` `⌘⇧T` | new tab, close tab, reopen the tab you closed |
| `⌃PgDn` `⌃PgUp` `⌃1` to `⌃9` | next, previous tab; tab 1 to 8, the last |
| `⌃⇧PgDn` `⌃⇧PgUp` | move the tab right, left |
| `⌘R` `⌘⇧R` | reload, reload without the cache |
| `⌘←` `⌘→` `Esc` | back, forward, stop |
| `⌘F` `⌘G` `⌘⇧G` | find in page, next, previous |
| `⌘=` `⌘-` `⌘0` | zoom in, zoom out, actual size |
| `⌥⌘I` | developer tools (the terminal's) |
| `⌃Q` | quit |

On a keyboard that is not a Mac's, Ctrl is what ⌘ is on a Mac. A web page that has a use for `⌘←`,
`⌘F`, `Esc` or the zoom keys has it; the tab keys, `⌘L`, `⌘R` and `⌃Q` are always browserpen's. Duplicate Tab, Close Other Tabs, Close Tabs to
the Right (also in a tab's menu) and Open in Your Browser are in the command list, `F1`.

### For agents and scripts

In bateri, `bateri browser` reads and drives the browser of a pane from the command line (`text`,
`snapshot`, `shot`, `open URL`, `click`, `type`, `back`, `reload`); `bateri browser text p9@prod` is
the page of pane 9 on the server `prod`.

## License

GPL-3.0-or-later.
