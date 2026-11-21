-- The find bar: a card in the top right of the page, over the web page, with the field, how many matches
-- there are ("3 of 12"), Previous, Next, Match case and Close. The browser finds and marks what is found
-- (it is the browser's: the page keeps no text of it); the bar is the page's. Every place on it has a width
-- of its own, so that the count changing does not move anything.

local kit = require("../lib/kit")
local commands = require("./commands")
local tabs = require("./tabs")
local browse = require("./browse")

local b, u, S, act = kit.b, kit.u, kit.S, kit.act

local find = {}

local COUNT_W = 84

-- Opens the bar (or goes back into it), with what it holds selected, and finds it again.
function act.open_find()
	local tab = tabs.active()
	if tab == nil or not tab.opened then
		return
	end
	S.find.open = true
	S.find.select = true
	if S.find.query.value ~= "" then
		browse.find(tab, S.find.query.value, S.find.case)
	end
	kit.gather(u:focus("find"))
end

-- Closes the bar: the search ends in the page and its marks go. The keyboard goes back to the page unless
-- `quiet` (a tab was left: the keyboard is where that put it).
function act.close_find(quiet)
	if not S.find.open then
		return
	end
	S.find.open = false
	local tab = tabs.active()
	if tab ~= nil and tab.opened then
		browse.command(tab, "stop_finding")
		if not quiet then
			kit.gather(u:focus(tab.key))
		end
	end
end

-- What the browser says it found, in words.
local function count_words(tab)
	local found = tab and tab.state and tab.state.found
	if S.find.query.value == "" or found == nil then
		return ""
	elseif found.current == nil then
		return "No matches"
	end
	return found.current .. " of " .. found.count
end

function find.view()
	if not S.find.open then
		return nil
	end
	local t = u.theme
	local tab = tabs.active()
	local field = u:input("find", {
		state = S.find.query,
		label = "Find in page",
		hint = "Find",
		escape = true,
		select = S.find.select or nil,
		trail = { u:toggle("find-case", { on = S.find.case, glyph = "Aa", label = "Match case" }) },
	})
	S.find.select = false
	return b.div():anchor("address-box", "below", { gap = 6, align = "end" }):id("find-bar"):row():items_center():gap(4):p(6)
		:bg(t.popup):rounded(t.radius.card):border(1, t.line)
		:shadow(t.elevation.dx, 8, 24, b.alpha("shadow", 0.4))
		:child(b.div():w(200):child(field))
		:child(b.div():w(COUNT_W):no_shrink():child(b.text(count_words(tab)):color(t.ink_dim):text_size(t.text.small):truncate()))
		:child(u:icon_button("find-previous", { icon = "chevron_up", label = "Find Previous", keys = commands.keys("find_previous"), size = t.size.control }))
		:child(u:icon_button("find-next", { icon = "chevron_down", label = "Find Next", keys = commands.keys("find_next"), size = t.size.control }))
		:child(u:icon_button("find-close", { icon = "close", label = "Close Find", keys = "escape", size = t.size.control }))
end

-- An action of the bar. Whether it was its own.
function find.action(action)
	local from, kind = action.from, action.kind
	local tab = tabs.active()
	if from == "find" then
		if kind == "change" then
			if tab ~= nil then
				browse.find(tab, action.text, S.find.case)
			end
		elseif kind == "submit" then
			browse.command(tab, action.mods and action.mods.shift and "find_previous" or "find_next")
		elseif kind == "escape" then
			act.close_find()
		end
	elseif from == "find-case" and kind == "toggle" then
		S.find.case = action.on
		if tab ~= nil and S.find.query.value ~= "" then
			browse.find(tab, S.find.query.value, S.find.case)
		end
	elseif from == "find-previous" and kind == "press" then
		browse.command(tab, "find_previous")
	elseif from == "find-next" and kind == "press" then
		browse.command(tab, "find_next")
	elseif from == "find-close" and kind == "press" then
		act.close_find()
	else
		return nil
	end
	return true
end

return find
