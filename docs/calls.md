# The page and the program

The page (`src/view/`) and the program (`src/program/`) talk only through what is written here: the
calls the page makes. A change to one starts in this file. The code that answers is
`src/program/api/`; the page's own side is `src/view/parts/` (`browse.lua` is where the page asks the
program, and the host's browsers, for things).

What the page shows of the web is not here: each tab is a browser of the terminal's
(`pencere.browser`), which loads and draws the pages, and the page and the host talk about it in the
kit's own words (`browser` events, `browser = { ... }` asks in a reply). The program never sees a web
page, an address typed or a click; it knows the machine's ports and the addresses the profile has been
to.

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
  "recent": [ { "url": "http://localhost:3000/", "title": "Dashboard" } ]
}
```

`profile` is the name the browsers keep what sites store under (`--profile`, `default` where none was
given). `start` is the address the command line gave, as it was typed, and is missing where it gave
none: the page makes an address of it as it does of what the person types. `ports` is `ports` below and
`recent` is the profile's recent addresses, the latest first.

### `ports`

The TCP ports the person's own processes listen on, on the machine the program runs on, as of now: the
page asks every two seconds while it shows them (a blank tab, or an address being written). Params:
none.

```json
{ "ports": [ { "port": 3000, "name": "node" } ] }
```

`name` is the process's name: the last part of its path, never the path (or, for `node`, `python` and
the like, the script it runs). A port is listed once, the lowest first. The list is the person's
processes' alone: a server that root, another user or a container holds is not in it. On macOS a
program that runs from inside an application bundle (`Foo.app/Contents/...`: Control Center, a helper of
a Dock app, a database that comes as an app) is not in it either: the list is what a developer started
in a terminal. Linux lists all of the person's own.

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

### `quit`

Ends browserpen: a page of the kit has no close of its own, its view is the program's to end. Params:
none. Answers `{}`, or `{ "unsupported": "..." }` in a terminal that cannot close the view, which the
page says to the person.

## Limits

An answer is a few hundred bytes at most (the ports are asked for by a page that lists a few dozen at
most), so no call comes near the kit's limits.
