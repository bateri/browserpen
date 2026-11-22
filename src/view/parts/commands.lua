-- Every command, once: the id, the title, the group and the keys each has. The palette, the
-- shortcuts sheet, the host's list of commands and the keys that reach the page while a web page has
-- the keyboard (the browser's `keys`) are all made of this table, so a key is changed in one place.
--
-- Two kinds of keys. `always` is a key the page keeps while the browser has the keyboard: the web page
-- in it never sees it (the browser's `keys`, at most 32). The others go to the browser first and come
-- to the page only where the web page and the browser have no use for them (⌘← where the browser
-- would not go back, Esc, ⌘F, the zoom keys), so a web page that has a use for one has it.
--
-- The keys bateri keeps for itself never reach a pen (⌘T ⌘W ⌘N ⌘O ⌘P ⌘D ⌘[ ⌘] ⌘1–9, ⌃Tab, ⌥⌘ and ⌃⌘ with
-- the arrows) and ⌃C ends the program, so no command depends on one. A key with a shift and a mark
-- or a digit is written as the character the host gives (`mod+?`, not `mod+shift+/`): the kit
-- matches no other.
--
-- `shown` is a key that is written beside a command and bound by someone else: Esc is the page's own
-- key by its place (the address field, the find bar and the suggestions each take it first, the
-- browser stops a load with it), and the tools of the browser are the host's (⌥⌘I).

local kit = require("../lib/kit")
local keys = require("@pencere/keys")

local S = kit.S

local commands = {}

-- `when` names the condition a command can be done under (page.lua gives them: `commands.when`): what
-- the page is showing, never where the keyboard is.
local TABLE = {
	{ id = "address", title = "Open Address", group = "Address", keys = "mod+l", always = true },
	{ id = "copy_address", title = "Copy Address", group = "Address", when = "page" },

	{ id = "new_tab", title = "New Tab", group = "Tabs", keys = "ctrl+t", always = true },
	{ id = "close_tab", title = "Close Tab", group = "Tabs", keys = "ctrl+w", always = true },
	{ id = "reopen_tab", title = "Reopen Closed Tab", group = "Tabs", keys = "mod+shift+t", always = true, when = "closed" },
	{ id = "duplicate_tab", title = "Duplicate Tab", group = "Tabs", when = "page" },
	{ id = "close_others", title = "Close Other Tabs", group = "Tabs", when = "tabs" },
	{ id = "close_right", title = "Close Tabs to the Right", group = "Tabs", when = "tab_after" },
	{ id = "next_tab", title = "Next Tab", group = "Tabs", keys = "ctrl+page_down", always = true, when = "tabs" },
	{ id = "previous_tab", title = "Previous Tab", group = "Tabs", keys = "ctrl+page_up", always = true, when = "tabs" },
	{ id = "move_left", title = "Move Tab Left", group = "Tabs", keys = "ctrl+shift+page_up", always = true, when = "tab_before" },
	{ id = "move_right", title = "Move Tab Right", group = "Tabs", keys = "ctrl+shift+page_down", always = true, when = "tab_after" },
	{ id = "tab_1", title = "Go to Tab 1", group = "Tabs", keys = "ctrl+1", always = true, hidden = true, when = "tabs" },
	{ id = "tab_2", title = "Go to Tab 2", group = "Tabs", keys = "ctrl+2", always = true, hidden = true, when = "tabs" },
	{ id = "tab_3", title = "Go to Tab 3", group = "Tabs", keys = "ctrl+3", always = true, hidden = true, when = "tabs" },
	{ id = "tab_4", title = "Go to Tab 4", group = "Tabs", keys = "ctrl+4", always = true, hidden = true, when = "tabs" },
	{ id = "tab_5", title = "Go to Tab 5", group = "Tabs", keys = "ctrl+5", always = true, hidden = true, when = "tabs" },
	{ id = "tab_6", title = "Go to Tab 6", group = "Tabs", keys = "ctrl+6", always = true, hidden = true, when = "tabs" },
	{ id = "tab_7", title = "Go to Tab 7", group = "Tabs", keys = "ctrl+7", always = true, hidden = true, when = "tabs" },
	{ id = "tab_8", title = "Go to Tab 8", group = "Tabs", keys = "ctrl+8", always = true, hidden = true, when = "tabs" },
	{ id = "tab_last", title = "Go to Last Tab", group = "Tabs", keys = "ctrl+9", always = true, hidden = true, when = "tabs" },

	{ id = "reload", title = "Reload", group = "Page", keys = "mod+r", always = true, when = "page" },
	{ id = "reload_hard", title = "Reload Without Cache", group = "Page", keys = "mod+shift+r", always = true, when = "page" },
	{ id = "back", title = "Back", group = "Page", keys = "mod+left", when = "back" },
	{ id = "forward", title = "Forward", group = "Page", keys = "mod+right", when = "forward" },
	{ id = "stop", title = "Stop", group = "Page", shown = "escape", when = "loading" },
	-- Zoom In is ⌘+ as a browser writes it, and every key that types + or = with ⌘: on a US keyboard ⌘= (and
	-- ⌘⇧= for +), on a Turkish Q one ⌘⇧4 for + and ⌘⇧0 for =, where neither is a key of its own.
	{ id = "zoom_in", title = "Zoom In", group = "Page", keys = { "mod++", "mod+shift++", "mod+=", "mod+shift+=" }, when = "page" },
	{ id = "zoom_out", title = "Zoom Out", group = "Page", keys = "mod+-", when = "page" },
	{ id = "zoom_reset", title = "Actual Size", group = "Page", keys = "mod+0", when = "page" },
	{ id = "devtools", title = "Developer Tools", group = "Page", shown = "alt+mod+i", when = "page" },
	-- Both are in the list only: no key is the one that empties what a person is signed in with.
	{ id = "clear_cache", title = "Clear Cache", group = "Page", when = "ready" },
	{ id = "clear_site", title = "Clear Site Data", group = "Page", when = "site" },
	-- Where the host shows no browser, the page's address opens in the person's own.
	{ id = "open_outside", title = "Open in Your Browser", group = "Page", when = "unsupported" },

	{ id = "find", title = "Find", group = "Find", keys = "mod+f", when = "page" },
	{ id = "find_next", title = "Find Next", group = "Find", keys = "mod+g", when = "finding" },
	{ id = "find_previous", title = "Find Previous", group = "Find", keys = "mod+shift+g", when = "finding" },

	-- The arrow keys while the rows of suggestions are offered: the field would have them otherwise.
	{ id = "suggest_next", title = "Next Suggestion", group = "Address", keys = "down", hidden = true, when = "navigating" },
	{ id = "suggest_previous", title = "Previous Suggestion", group = "Address", keys = "up", hidden = true, when = "navigating" },

	{ id = "commands", title = "Show All Commands", group = "Help", keys = "f1" },
	{ id = "shortcuts", title = "Keyboard Shortcuts", group = "Help", keys = "mod+?" },
	{ id = "quit", title = "Quit", group = "Help", keys = "ctrl+q", always = true },
}

-- The conditions, given by page.lua.
commands.when = {}

commands.list = TABLE

local by_id = {}
for _, command in TABLE do
	by_id[command.id] = command
end

function commands.get(id)
	return by_id[id]
end

-- Whether a command means anything now.
function commands.enabled(command)
	if command.when == nil then
		return true
	end
	local when = commands.when[command.when]
	return when == nil or when() == true
end

-- The keys the browser keeps for the page: the `always` ones.
function commands.browser_keys()
	local out = {}
	for _, command in TABLE do
		if command.always then
			out[#out + 1] = command.keys
		end
	end
	return out
end

-- The keys of a command as written for people ("⌘L"), or nil: the bound ones, else the ones it shows.
function commands.label(id)
	local command = by_id[id]
	if command == nil then
		return nil
	end
	local spec = command.keys or command.shown
	return spec and keys.label(spec) or nil
end

-- The keys of a command as the words a component takes (`u:kbd`, a tip), or nil.
function commands.keys(id)
	local command = by_id[id]
	return command and (command.keys or command.shown) or nil
end

-- The list for the library's `ui.commands`: what the host lists and does by their keys (the ones that
-- are the page's own: `shown` is not a key of the page's).
function commands.for_library()
	local out = {}
	for _, command in TABLE do
		out[#out + 1] = {
			id = command.id,
			title = command.title,
			group = command.group,
			keys = command.keys,
			when = function()
				return commands.enabled(command)
			end,
		}
	end
	return out
end

-- The commands that can be done now and are not hidden, as the items of a palette.
function commands.items()
	local out = {}
	for _, command in TABLE do
		if not command.hidden and commands.enabled(command) then
			out[#out + 1] = { key = command.id, title = command.title, hint = command.group, keys = command.keys or command.shown }
		end
	end
	return out
end

-- The groups of the shortcuts sheet: { { title =, items = { { title =, keys = } } } }. The tabs'
-- numbers are one line: ⌃1 to ⌃8 and ⌃9 for the last.
function commands.help()
	local groups, at = {}, {}
	for _, command in TABLE do
		local group = at[command.group]
		if group == nil then
			group = { title = command.group, items = {} }
			at[command.group] = group
			groups[#groups + 1] = group
		end
		if command.id == "tab_1" then
			group.items[#group.items + 1] = { title = "Go to Tab 1 to 8", keys = "ctrl+1", also = "ctrl+8" }
		elseif command.id == "tab_last" then
			group.items[#group.items + 1] = { title = "Go to Last Tab", keys = command.keys }
		elseif not (command.hidden and command.id ~= "tab_1") then
			-- A command of several keys is shown by the first, as a menu shows it.
			local first = type(command.keys) == "table" and command.keys[1] or command.keys
			group.items[#group.items + 1] = { title = command.title, keys = first or command.shown }
		end
	end
	return groups
end

return commands
