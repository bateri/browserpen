-- What every file of the page shares: the kit's builders, the library's context `u` (which remembers
-- the pointer and the keyboard and gives the components), the page's state, the reply that what the
-- page does is gathered into, and the few pieces the page is made of more than once (an icon, a
-- spinner, a toast).

local keys = require("@pencere/keys")
local lib = require("@pencere/ui")

local b = pencere
-- Compact: a browser's face is a thin strip above the page, and the page is what the pane is for.
local u = lib.new({ density = "compact" })

-- The page's state: what the program said and what the person did, in one place, so that `render` is
-- a function of it. What a web page is, is in the host's browser, and the page keeps only what the
-- browser told of it (`tab.state`).
local S = {
	now = 0,
	width = 1000,
	height = 700,
	-- "wide" or "narrow", by the page's width.
	layout = "wide",
	-- What `boot` said, and why there is none.
	boot = nil,
	boot_error = nil,
	-- The ports of the machine the program runs on and the addresses the profile has been to, as the
	-- program last said them.
	ports = {},
	recent = {},
	-- The tabs, in their order ({ key =, url =, state =, wanted =, opened = }), the key of the one in
	-- front, the number the next tab is given (a key is never used twice: the same name is the same
	-- browser and its history) and the addresses of the tabs closed lately, the latest last.
	tabs = {},
	active = nil,
	serial = 0,
	closed = {},
	-- The address field: what it holds, whether the person has typed in it since it was last filled
	-- from the browser, and whether what it is given next is to be selected.
	address = { value = "" },
	typed = false,
	select_address = false,
	-- Whether the suggestions wait, after the keyboard left the field, for the press that took it (the
	-- row it is on is gone there): see `page.lua`.
	holding = false,
	-- The suggestions of what is typed, and the row the arrow keys are on (the key of the row, nil
	-- before they are used).
	cursor = nil,
	-- The find bar: whether it is open, and what the search says (the library's input and toggle).
	find = { open = false, query = { value = "" }, case = false },
	-- What is over the page: "commands" (the palette) or "shortcuts", and the palette's state.
	overlay = nil,
	palette = { query = "", cursor = nil, field = { value = "" } },
	toasts = {},
	-- The tab whose address that did not load is on a port nothing listened on, and the port: the
	-- program is asked again until something does.
	watching = nil,
}

local SIZE = {
	-- The room around what the bars hold, points.
	pad = 8,
	-- The width below which the page is "narrow": the toolbar keeps its three buttons and the field.
	narrow = 480,
	-- The line that shows a page loading, points.
	line = 2,
}

-- --- What the page does is gathered into one reply ---------------------------------------------
--
-- What an action asks of the host (the keyboard, a browser, a link) is put here as it is decided,
-- wherever in the page that is, and every way into the page (an event, an answer of the program, a
-- timer) hands it out with its own reply.

local gathered = nil

-- Puts `reply` with what was gathered. The kit's `merge` keeps the asks of both replies' `browser`
-- (each of every tab, in the order they were decided), so two asks of a browser go once each.
local function gather(reply)
	gathered = b.merge(gathered, reply)
end

-- The reply of a way into the page: `reply` with what was gathered.
local function take(reply)
	local out = gathered
	gathered = nil
	return b.merge(out, reply)
end

-- Asks the program to do `method`; `done(result, err)` is called with its answer, and what it
-- gathered goes out with that answer's reply. `err` is { code =, message = }.
local function call(method, params, done)
	return b.call(method, params, function(result, err)
		local reply = nil
		if done then
			reply = done(result, err)
		end
		return take(reply == nil and true or reply)
	end)
end

-- --- Words and pieces ------------------------------------------------------------------------------

-- One of the pictures of src/assets/icons (Tabler's, MIT), `size` points, in a colour of the theme.
local function icon(name, size, color)
	size = size or 15
	return b.image("assets/icons/" .. name .. ".svg"):size(size, size):tint(color or "dim"):no_shrink()
end

-- A picture turning round, for what is being waited for: the host turns it, a frame at a time.
local function spinner(size, color)
	return icon("loader-2", size, color):motion({ spin = 1 })
end

-- The keys of a command as the host's keyboard writes them (`@pencere/keys`: "cmd+shift+k" is "⇧⌘K"
-- on a Mac, "Super+Shift+K" elsewhere); the first of a list.
local function keys_of(spec)
	if type(spec) == "table" then
		spec = spec[1]
	end
	if not spec then
		return nil
	end
	return keys.label(spec)
end

-- Says `words` over the page for a few seconds: a toast.
local function toast(words, tone, seconds)
	lib.toast(S.toasts, words, tone or "info", seconds)
end

-- The actions of the page, by name, filled by the parts that do them: a part calls another's action
-- through here, so that no two parts require each other.
local act = {}

return {
	b = b,
	u = u,
	lib = lib,
	S = S,
	SIZE = SIZE,
	act = act,
	gather = gather,
	take = take,
	call = call,
	icon = icon,
	spinner = spinner,
	keys_of = keys_of,
	toast = toast,
}
