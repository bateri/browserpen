-- What is asked of a tab's browser, and what its browser tells back. The browser is the host's: the
-- page asks it to go, go back, reload, find and zoom in a reply, and is told all of what it is and
-- does each time any of it changes (`browser` events). An address is asked once the browser says it
-- is ready, and a host that shows no browser has the address open in the person's own.

local kit = require("../lib/kit")
local address = require("../lib/address")
local tabs = require("./tabs")

local b, S, act = kit.b, kit.S, kit.act

local browse = {}

-- The sizes a page is shown at, one step to the next.
local STEPS = { 0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4, 5 }

-- How often a port that nothing listened on is looked at again, seconds.
local WATCH = 1.5

-- The most recent addresses the page keeps (the program keeps the same).
local MOST_RECENT = 200

-- An ask of the browser of `tab`, after the ones asked before it.
local function ask(tab, request)
	kit.gather({ redraw = true, browser = { [tab.key] = { request } } })
end

-- The step after `zoom` (`direction` 1) or before it (-1), and the last one there is at the end.
local function stepped(zoom, direction)
	local chosen = zoom
	for _, step in STEPS do
		if direction > 0 and step > zoom + 0.001 then
			return step
		elseif direction < 0 and step < zoom - 0.001 then
			chosen = step
		end
	end
	return chosen
end

-- Whether the host shows no browser: it said so of this one.
function browse.unsupported(tab)
	return tab.state ~= nil and tab.state.status == "unsupported"
end

-- Opens `url` in the person's own browser, where the host shows none.
function browse.open_outside(url)
	if address.is_page(url) then
		kit.gather({ link = url })
	end
end

-- Goes to `url` in `tab`: now where its browser is ready, else when it says so. Where the host shows no
-- browser the address is kept for the tab and opens in the person's own.
function browse.go(tab, url)
	tab.url, tab.opened = url, true
	if browse.unsupported(tab) then
		tab.wanted = nil
		browse.open_outside(url)
		return
	end
	if tab.state ~= nil and tab.state.status == "ready" then
		tab.wanted = nil
		ask(tab, { go = url })
	else
		tab.wanted = url
	end
end

function browse.command(tab, name)
	if tab ~= nil and tab.opened then
		ask(tab, { command = name })
	end
end

-- Loads the page again (`cache` false: not from what the browser keeps of it). A browser that crashed
-- is made again by it.
function browse.reload(tab, cache)
	if tab == nil or not tab.opened then
		return
	end
	tab.nothing, tab.answers = nil, nil
	if browse.unsupported(tab) then
		browse.open_outside(tabs.url_of(tab))
		return
	end
	ask(tab, { command = "reload", cache = cache })
end

function browse.zoom(tab, direction)
	if tab == nil or not tab.opened then
		return
	end
	local zoom = 1
	if direction ~= 0 then
		zoom = stepped(tab.state and tab.state.zoom or 1, direction)
	end
	ask(tab, { zoom = zoom })
end

-- Looks for `pattern` in the page of `tab`; an empty one ends the search.
function browse.find(tab, pattern, case)
	S.find.asked = pattern
	ask(tab, { command = "find", pattern = pattern, case = case })
end

-- --- Recent addresses -----------------------------------------------------------------------------------------

-- The page is at `url`, loaded: the program keeps it (the profile's recent addresses, on the machine whose
-- `localhost` it is) and the page's list has it first.
local function visited(tab, state)
	local mark = state.url .. "\0" .. state.title
	if tab.recorded == mark then
		return
	end
	tab.recorded = mark
	for index, visit in S.recent do
		if visit.url == state.url then
			table.remove(S.recent, index)
			break
		end
	end
	table.insert(S.recent, 1, { url = state.url, title = state.title })
	while #S.recent > MOST_RECENT do
		table.remove(S.recent)
	end
	kit.call("visit", { url = state.url, title = state.title })
end

-- --- Nothing is listening ---------------------------------------------------------------------------------------

-- Looks at the port of a `localhost` address that did not load, and again while it stays so: the card says
-- that nothing is listening on it (`tab.nothing`, the port) until something answers, and then the page asks the
-- browser to go to the address again, as it would for a person who pressed Reload: the card says that
-- something is listening (`tab.answers`) until the browser reports the new load, and Reload is the person's
-- where it fails again. Whether anything answers is the program's to say: the list of ports is the person's own
-- processes', and a server that root or a container holds is there all the same.
local function watch(tab, failed)
	local port = address.local_port(failed.url)
	if port == nil then
		tab.nothing, tab.answers = nil, nil
		return
	end
	local function again()
		local state = tab.state
		if not tab.probing or tabs.by_key(tab.key) == nil or state == nil or state.error == nil or state.error.url ~= failed.url then
			tab.probing = false
			return nil
		end
		kit.call("probe", { port = port }, function(result, err)
			if err or result == nil then
				return nil
			end
			local nothing, answers = nil, nil
			if result.listening then
				answers = port
			else
				nothing = port
			end
			if nothing == tab.nothing and answers == tab.answers then
				return nil
			end
			local started = tab.nothing ~= nil and answers ~= nil
			tab.nothing, tab.answers = nothing, answers
			if started then
				ask(tab, { go = failed.url })
			end
			return true
		end)
		pencere.time.after(WATCH, again)
		return nil
	end
	if not tab.probing then
		tab.probing = true
		again()
	end
end

-- --- What the browser tells ---------------------------------------------------------------------------------------

-- The browser of `tab` says what it is and does (all of it, each time any of it changes).
function browse.told(tab, state)
	local before = tab.state
	tab.state = state
	-- From the first address it reports, the browser's about:blank is the page's own.
	if state.url ~= "" and string.lower(state.url) ~= "about:blank" then
		tab.reported = true
	end
	if state.status == "ready" and tab.wanted ~= nil then
		local url = tab.wanted
		tab.wanted = nil
		ask(tab, { go = url })
	elseif state.status == "unsupported" and tab.wanted ~= nil then
		-- The browser of a tab is made when the tab gets its address, so that a host that shows none says
		-- so after the address was asked for: it opens in the person's own browser then.
		local url = tab.wanted
		tab.wanted = nil
		browse.open_outside(url)
	end
	if state.error == nil then
		tab.nothing, tab.answers = nil, nil
	else
		watch(tab, state.error)
	end
	if not state.loading and state.error == nil and address.is_page(state.url) and (before == nil or before.url ~= state.url or before.title ~= state.title or before.loading) then
		visited(tab, state)
	end
	if S.active == tab.key then
		act.fill_address()
	end
end

browse.stepped = stepped

return browse
