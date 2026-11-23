# The page and the program

The page (`src/view/`) and the program (`src/program/`) talk only through what is written here: the
calls the page makes. A change to one starts in this file. The code that answers is
`src/program/api/`; the page's own side is `src/view/parts/` (`browse.lua` is where the page asks the
program, and the host's browsers, for things).

What the page shows of the web is not here: each tab is a browser of the terminal's
(`pencere.browser`), which loads and draws the pages, and the page and the host talk about it in the
kit's own words (`browser` events, `browser = { ... }` asks in a reply). The program never sees a web
page, an address typed or a click; it knows the machine's ports, the addresses the profile has been
to and the tabs the page has open (their addresses, titles and sizes), which it keeps in memory for a
view that starts again.

**Errors.** A call that cannot be done answers with an error: `bad-params` (the params are not what
the call takes: a member it does not know, a value out of range), `not-found` (no such call),
`internal` (the program failed). The words of an error are for the person.

## Calls

### `boot`

What the page needs before it draws, once. Params: none.

```json
{
  "version": "0.1.0",
  "profile": "default",
  "start": "localhost:3000",
  "ports": [ { "port": 3000, "name": "node" }, { "port": 5173, "name": "vite" } ],
  "recent": [ { "url": "http://localhost:3000/", "title": "Dashboard" } ],
  "tabs": {
    "tabs": [
      { "key": "tab-1", "url": "http://localhost:3000/", "title": "Dashboard", "zoom": 1.25 },
      { "key": "tab-3" },
      { "key": "tab-2", "url": "http://localhost:5173/", "title": "Vite" }
    ],
    "active": "tab-2",
    "serial": 3
  }
}
```

`profile` is the name the browsers keep what sites store under (`--profile`, `default` where none was
given). `start` is the address the command line gave, as it was typed, and is missing where it gave
none: the page makes an address of it as it does of what the person types. It is in the answer to the
program's first `boot` only: the address of the command line is for the first view, and a view that
starts again (the connection to a server came back, the terminal made the view anew) does not open it
again. `ports` is `ports` below and `recent` is the profile's recent addresses, the latest first.

`tabs` is what the page last said with `tabs` below, and is missing where it has said nothing (the first
view, or one that was made again before its first tab was told): the page makes those tabs again, in
that order, each going to its own address (a tab with no `url` is blank), the one named by `active` in
front, and names the next tab by `serial`. The history behind a tab (Back and Forward) is its browser's
and starts again.

### `ports`

The TCP ports the person's own processes listen on, on the machine the program runs on, as of now: the
page asks every two seconds while it shows them (a blank tab, or an address being written). Params:
none.

```json
{ "ports": [ { "port": 3000, "name": "node" } ] }
```

`name` is the process's name: the last part of its path, never the path (or, for `node`, `python`,
`ruby` and the like, in any case and with a version (`python3.12`), the script or module it runs:
`http.server`, `server.js`). A port is listed once, the lowest first. The list is the person's
processes' alone: a server that root, another user or a container holds is not in it. On macOS it is
what a developer started in a terminal: a program that runs from inside an application bundle
(`Foo.app/Contents/...`: a helper of a Dock app, a database that comes as an app) or from the system's
own directories (`/System`, `/usr/libexec`, `/usr/sbin`, `/sbin`: Control Center on :5000 and :7000) is
not in it. Four things are, though they are in a bundle: a program in a bundle that is inside a
framework (Python runs as `Python.framework/.../Python.app`, Xcode's too), what holds the ports of a
container (OrbStack's helper, Docker Desktop's `com.docker.backend` and `com.docker.vpnkit`: the
server in the container is behind them), and a bundle under `node_modules` (Electron). Linux lists all
of the person's own.

### `probe`

Whether anything answers on a port of this machine's loopback. The page asks it of a `localhost`
address that did not load, to say "Nothing is listening on :3000 yet" (or that something is, now), and
asks again every 1.5 seconds while the address stays failed. Params: `{ "port": 3000 }`.

```json
{ "listening": false }
```

### `visit`

The person has been to an address: the profile's recent addresses have it first (an address already
there moves to the front; at most 200 are kept, on the machine the program runs on, in the
profile's own file). Params: `{ "url": "http://localhost:3000/", "title": "Dashboard" }` (`title` may be
missing). Answers `{}`.

### `tabs`

The tabs the page has open, in the order of the strip: the page says all of them each time any of them
changes (a tab opened, closed, moved or brought to the front, an address or a title reached, a zoom
set), and the program keeps what it was last told in memory, not on a disk: when the program ends the
tabs end with it, and a program that is run again begins with none. At most 200 tabs are kept (see Limits).
Params:

```json
{
  "tabs": [
    { "key": "tab-1", "url": "http://localhost:3000/", "title": "Dashboard", "zoom": 1.25 },
    { "key": "tab-3" }
  ],
  "active": "tab-1",
  "serial": 3
}
```

`key` names the tab (and its browser); `url`, `title` and `zoom` (only where the page is not shown at its
own size) are missing for a tab that has had no address yet, and `title` where the page has none.
`active` is the key of the tab in front and `serial` the number the next tab is named by. Answers `{}`.

### `quit`

Ends browserpen: a page of the kit has no close of its own, its view is the program's to end. Params:
none. Answers `{}`, or `{ "unsupported": "..." }` in a terminal that cannot close the view, which the
page says to the person.

## Topic

### `pencere.take`

What the terminal hands the program while it runs. The program says in its `open` that it takes an
address (`takes: ["url"]`), and a terminal that has a link for it, followed in another pane of the same
tab, hands it over instead of starting another browserpen. The page subscribes once, after `boot`
(params: none), and is pushed each address as it comes: `{ "url": "https://example.org/a" }`, `http` or
`https`. The program's library holds the few that came before the page subscribed and gives them to it
first. The page opens each as a tab of its own, in front, at once. The program does not read the
address: it does not keep it either, and `tabs` says it with the others once the tab is open.

## Limits

An answer is a few hundred bytes at most (the ports are asked for by a page that lists a few dozen at
most), so no call comes near the kit's limits, with one exception: the tabs, which `tabs` and the `tabs`
of `boot` carry. A message is 1 MiB at the most in the kit; at most 200 tabs are kept, an address longer
than 2048 bytes is left out (that tab comes back blank) and a title is cut to 200 characters, which is
about 600 KB for the most there can be. The page does the same before it says them, and the program holds
to it for what it is told.
