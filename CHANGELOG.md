# Changelog

## [Unreleased]

### Added

- **A web browser for the terminal pane.** Tabs, an address bar, back, forward, reload, find and zoom.
  Pages load from the machine browserpen runs on, so `localhost:3000` over ssh is that server's own.
- **A new tab suggests where to go.** It lists the ports your programs listen on and the pages you
  visited; typing narrows the list.
- **Tabs come back when the view reconnects.**
- **`browserpen 3000` opens `localhost:3000`.** Words that are not an address are a search.
- **A page that did not load says why,** and loads by itself once the server it waits for starts.
- **Open a development server with its own certificate.** When a certificate isn't trusted the page says
  so and offers Continue Anyway.
- **Clear Cache and Clear Site Data** in the command list (`F1`). Clearing a site's data asks first, since
  it signs you out.
- **Every shortcut is on the screen:** `F1` lists the commands, `⌘?` the keys.
- **In a terminal that shows no browser** the tabs and the address bar are drawn as pictures, and
  pages open in your own browser.
