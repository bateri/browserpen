-- The tabs: a strip above the page, also with one tab, and what is done with them. A tab is a browser
-- of the host's (`pencere.browser("tab-N")`), kept while its tab is: the ones behind are hidden, and
-- their history stays. A tab that has no address yet is blank: the page's own beginning stands in the
-- room, and no address has been asked of its browser.

local kit = require("../lib/kit")
local address = require("../lib/address")
local commands = require("./commands")

local b, u, S, act = kit.b, kit.u, kit.S, kit.act

local tabs = {}

-- The tabs closed lately that `Reopen Closed Tab` brings back.
local MOST_CLOSED = 20

function tabs.by_key(key)
	for index, tab in S.tabs do
		if tab.key == key then
			return tab, index
		end
	end
	return nil, nil
end

function tabs.active()
	return (tabs.by_key(S.active))
end

-- The address a tab is at: what its browser says, else what it was asked to go to.
function tabs.url_of(tab)
	local state = tab.state
	-- A browser that has not been anywhere is at about:blank: that is no address of the tab's. One that
	-- has been somewhere and is at about:blank again went there itself, and the tab says so.
	if state ~= nil and state.url ~= "" and (tab.reported or string.lower(state.url) ~= "about:blank") then
		return state.url
	end
	return tab.url or ""
end

-- Whether a tab has had an address: a blank one shows the page's beginning.
function tabs.blank(tab)
	return not tab.opened
end

function tabs.status_of(tab)
	return tab.state and tab.state.status or "starting"
end

-- A tab made again from what the program kept (`tabs.restore`) has the title it had until its browser has
-- loaded the page (`browse.told`) and says one of its own.
function tabs.title_of(tab)
	local url = tabs.url_of(tab)
	local title = tab.state and tab.state.title
	if title == nil or title == "" then
		title = tab.title
	end
	return address.title_of(url, title)
end

-- A tab, last or after the tab `after`, with the address it is to open (none: blank). Its browser is
-- told nothing until it says it is ready.
function tabs.make(url, after)
	S.serial += 1
	local tab = { key = "tab-" .. S.serial, url = nil, state = nil, wanted = nil, opened = false }
	if url ~= nil then
		tab.url, tab.wanted, tab.opened = url, url, true
	end
	local at = #S.tabs + 1
	if after ~= nil then
		local _, index = tabs.by_key(after)
		at = (index or #S.tabs) + 1
	end
	table.insert(S.tabs, at, tab)
	return tab
end

-- --- What the program keeps ---------------------------------------------------------------------------------

-- The program keeps the tabs for as long as it runs, so that a view that starts again (this page is made
-- from nothing then) finds them: for each tab its name, address, title and zoom, in the order of the strip,
-- and the one in front. The page says them all each time any of them changes (`tabs.sync`).
--
-- A call is a message of 1 MiB at the most: an address longer than `URL_BYTES` is left out (that tab comes
-- back blank) and a title is cut to `TITLE_CHARS` characters. What a tab made again had (`tab.title`,
-- `tab.zoom`) stands in for what its browser has not said yet, until the page has loaded.
local URL_BYTES = 2048
local TITLE_CHARS = 200

local function cut(title)
	local length = utf8.len(title)
	if length == nil then
		return ""
	elseif length > TITLE_CHARS then
		return string.sub(title, 1, utf8.offset(title, TITLE_CHARS + 1) - 1)
	end
	return title
end

function tabs.saved()
	local list = {}
	for _, tab in S.tabs do
		local entry = { key = tab.key }
		local url = tab.opened and tabs.url_of(tab) or nil
		if url ~= nil and #url <= URL_BYTES then
			local state = tab.state
			local title = state ~= nil and state.title or ""
			if title == "" then
				title = tab.title or ""
			end
			entry.url = url
			entry.title = cut(title)
			local zoom = tab.zoom or (state ~= nil and state.zoom) or nil
			if zoom ~= nil and math.abs(zoom - 1) > 0.001 then
				entry.zoom = zoom
			end
		end
		list[#list + 1] = entry
	end
	return { tabs = list, active = S.active, serial = S.serial }
end

local function fingerprint(saved)
	local parts = { tostring(saved.active), tostring(saved.serial) }
	for _, entry in saved.tabs do
		parts[#parts + 1] = table.concat({ entry.key, entry.url or "", entry.title or "", tostring(entry.zoom or "") }, "\0")
	end
	return table.concat(parts, "\1")
end

-- What the program was last told of the tabs.
local told = nil

-- Tells the program the tabs where they are not what it was last told. Before the page has the tabs the
-- program had (`boot`), it has none to tell: that would take them from the program.
function tabs.sync()
	if S.boot == nil or #S.tabs == 0 then
		return
	end
	local saved = tabs.saved()
	local mark = fingerprint(saved)
	if mark ~= told then
		told = mark
		kit.call("tabs", saved)
	end
end

-- Makes the tabs the program kept: in their order, each to go to its address (its browser is asked once it
-- is ready, with the zoom it had) and the one in front in front. The history of each browser starts again.
function tabs.restore(saved)
	local top = 0
	S.tabs = {}
	for _, entry in saved.tabs do
		local tab = { key = entry.key, url = nil, state = nil, wanted = nil, opened = false }
		if entry.url ~= nil and entry.url ~= "" then
			tab.url, tab.wanted, tab.opened = entry.url, entry.url, true
			tab.title, tab.zoom = entry.title, entry.zoom
		end
		table.insert(S.tabs, tab)
		local number = tonumber(string.match(entry.key, "^tab%-(%d+)$"))
		if number ~= nil and number > top then
			top = number
		end
	end
	S.serial = math.max(saved.serial or 0, top)
	S.active = nil
	local front = tabs.by_key(saved.active) or S.tabs[1]
	tabs.select(front.key)
end

-- Puts the keyboard where the tab in front wants it: in its browser, or in the address field of a
-- blank one (selected, for the address to be written at once).
function tabs.focus()
	local tab = tabs.active()
	if tab == nil then
		return
	end
	if tabs.blank(tab) then
		act.focus_address()
	else
		-- What was being written in the address field is dropped with the keyboard (the suggestions of it
		-- do not wait for a press that is not coming).
		S.typed = false
		S.cursor = nil
		kit.gather(u:focus(tab.key))
	end
end

-- Brings `key` to the front, with the keyboard where its page wants it, and the address field says
-- its address. The find bar is of one page: it closes.
function tabs.select(key)
	if tabs.by_key(key) == nil or S.active == key then
		return
	end
	act.close_find(true)
	S.active = key
	S.typed = false
	S.cursor = nil
	act.fill_address()
	tabs.focus()
end

-- A tab at the end (or after `after`), blank or at `url`; in front unless `background`.
function tabs.open(url, options)
	options = options or {}
	local tab = tabs.make(url, options.after)
	if not options.background then
		tabs.select(tab.key)
	end
	return tab
end

-- Closes the tab `key`. The tab that takes its place is the one after it, else the one before; the
-- last tab is replaced by a blank one, so that there is always a tab (⌃Q ends the program). The
-- address is kept for `Reopen Closed Tab`.
function tabs.close(key)
	local tab, index = tabs.by_key(key)
	if tab == nil then
		return
	end
	local url = tabs.url_of(tab)
	if tab.opened and address.is_page(url) then
		table.insert(S.closed, url)
		if #S.closed > MOST_CLOSED then
			table.remove(S.closed, 1)
		end
	end
	local was_active = S.active == key
	if #S.tabs == 1 then
		table.remove(S.tabs, index)
		local blank = tabs.make(nil)
		S.active = nil
		tabs.select(blank.key)
		return
	end
	table.remove(S.tabs, index)
	if was_active then
		act.close_find(true)
		local next_tab = S.tabs[math.min(index, #S.tabs)]
		S.active = next_tab.key
		S.typed = false
		S.cursor = nil
		act.fill_address()
		tabs.focus()
	end
end

function tabs.close_others(key)
	for index = #S.tabs, 1, -1 do
		if S.tabs[index].key ~= key then
			tabs.close(S.tabs[index].key)
		end
	end
	tabs.select(key)
end

function tabs.close_right(key)
	local _, index = tabs.by_key(key)
	for at = #S.tabs, (index or #S.tabs) + 1, -1 do
		tabs.close(S.tabs[at].key)
	end
end

-- The tab `step` places from the one in front, round the ends.
function tabs.step(step)
	local _, index = tabs.by_key(S.active)
	if index == nil or #S.tabs < 2 then
		return
	end
	tabs.select(S.tabs[(index - 1 + step) % #S.tabs + 1].key)
end

-- The tab at `number` (1 to 8), or the last (nil).
function tabs.go_to(number)
	local tab = number and S.tabs[number] or S.tabs[#S.tabs]
	if tab ~= nil then
		tabs.select(tab.key)
	end
end

-- Puts the tab `key` in front of the tab `before` (the end where nil).
function tabs.move(key, before)
	local tab, from = tabs.by_key(key)
	if tab == nil or key == before then
		return
	end
	table.remove(S.tabs, from)
	local at = #S.tabs + 1
	if before ~= nil then
		local _, index = tabs.by_key(before)
		at = index or at
	end
	table.insert(S.tabs, at, tab)
end

-- The tab `key` `step` places along the strip (not past either end).
function tabs.shift(key, step)
	local tab, index = tabs.by_key(key)
	if tab == nil then
		return
	end
	local to = index + step
	if to < 1 or to > #S.tabs then
		return
	end
	table.remove(S.tabs, index)
	table.insert(S.tabs, to, tab)
end

-- Brings the last tab closed back, in front, at the address it had.
function tabs.reopen()
	local url = table.remove(S.closed)
	if url ~= nil then
		tabs.open(url)
	end
end

-- --- The strip ----------------------------------------------------------------------------------------------

-- The slot at the head of a tab: a ring that turns while the page loads, a quiet dot while it does
-- not. Twelve points either way, so that a tab's title does not move when the load ends.
local function slot(tab)
	local box = b.div():size(12, 12):no_shrink():items_center():justify_center()
	if tab.state ~= nil and tab.state.loading then
		return box:child(kit.spinner(12, "dim"))
	end
	return box:child(b.div():size(4, 4):rounded(2):bg("quiet"))
end

local function tip_of(tab)
	local title, shown = tabs.title_of(tab), address.display(tabs.url_of(tab))
	if shown ~= "" and shown ~= title then
		return title .. " — " .. shown
	end
	return title
end

function tabs.view()
	local t = u.theme
	local items = {}
	for _, tab in S.tabs do
		items[#items + 1] = { key = tab.key, title = tabs.title_of(tab), icon = slot(tab), tip = tip_of(tab) }
	end
	local strip = u:tabs("tabs", { items = items, active = S.active, reorder = true })
	local plus = u:icon_button("new-tab", {
		icon = "plus",
		label = "New Tab",
		keys = commands.keys("new_tab"),
		size = t.size.control,
	})
	return b.div():row():items_center():gap(2):pr(4):h(t.size.bar):bg(t.surface)
		:child(strip:grow(1):min_w(0))
		:child(plus)
end

-- What a tab's menu has: commands of the table (its titles and keys are the table's), and a rule.
local MENU = { "reload", "duplicate_tab", "copy_address", "-", "close_others", "close_right" }

-- Whether the tab at `index` can have the menu's command done to it (the table's conditions are those
-- of the tab in front; a menu is of the tab that was pressed).
local function enabled_for(id, tab, index)
	if id == "close_others" then
		return #S.tabs > 1
	elseif id == "close_right" then
		return index < #S.tabs
	end
	return tab.opened
end

-- An action of the strip or of the button after it. Whether it was theirs.
function tabs.action(action)
	local from, kind = action.from, action.kind
	if from == "tabs" then
		if kind == "select" then
			tabs.select(action.key)
		elseif kind == "close" then
			tabs.close(action.key)
		elseif kind == "move" then
			tabs.move(action.key, action.before)
		elseif kind == "context" then
			local tab, index = tabs.by_key(action.key)
			if tab ~= nil then
				S.menu_tab = action.key
				local entries = {}
				for _, id in MENU do
					if id == "-" then
						entries[#entries + 1] = id
					else
						local command = commands.get(id)
						entries[#entries + 1] = { key = id, title = command.title, keys = commands.keys(id), enabled = enabled_for(id, tab, index) }
					end
				end
				kit.gather(u:menu("tab-menu", entries, { at = action.at }))
			end
		end
		return true
	elseif from == "new-tab" and kind == "press" then
		act.run("new_tab")
		return true
	elseif from == "tab-menu" and kind == "menu" then
		local tab = tabs.by_key(S.menu_tab)
		S.menu_tab = nil
		if tab == nil then
			return true
		end
		if commands.get(action.entry) ~= nil then
			act.run(action.entry, tab)
		end
		return true
	end
	return nil
end

return tabs
