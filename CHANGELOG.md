# Changelog

## [Unreleased]

### Added

- **A web browser for the terminal pane.** Tabs, an address bar, Back, Forward and Reload, find in
  page and zoom, in a terminal that shows pencere views (bateri). What a page loads comes from the
  machine the program runs on, so `localhost:3000` over ssh is that server's own.
- **A blank tab lists the ports the machine listens on and the addresses you have been to.** The
  arrow keys go through them and Enter goes to the one chosen; what you type narrows them. The ports
  are those of programs you started (on a Mac, not those of apps), by the program's name.
- **`browserpen [URL] [--profile NAME]`.** `3000` and `:3000` are `localhost:3000`; words that are not
  an address are a search. What the sites keep stays under the profile from one run to the next.
- **A page that did not load says why,** and where the address is a port nothing listens on, that
  nothing does yet (and when something starts to).
- **Every key is on the screen** (`F1` lists the commands, `⌘?` the shortcuts). The tab keys, `⌘L`,
  `⌘R` and `⌃Q` are browserpen's wherever the keyboard is; `⌘F`, `⌘G`, `⌘←`, `⌘→` and the zoom keys
  are too, unless the web page uses them.
- **In a terminal that shows no browser** the tabs and the address bar are drawn as pictures where
  the terminal speaks Kitty graphics, and an address opens in your own browser.
