-- The room under the bars: the browser of the tab in front, filling what is left, or, in a blank tab, the
-- page's own beginning. The browsers of the other tabs are hidden (their pages and histories stay).
--
-- Where a browser is not ready, the card of the page stands over its room: what the host said of it is
-- the card's one sentence (the page does not say it a second time), and the button has a verb for a
-- label. While it starts there is no card: the host draws its question and its progress in the room.

local kit = require("../lib/kit")
local address = require("../lib/address")
local commands = require("./commands")
local tabs = require("./tabs")
local suggest = require("./suggest")
local browse = require("./browse")

local b, u, S, act = kit.b, kit.u, kit.S, kit.act

local room = {}

-- A button of a card: its verb, and its key soft beside it.
local function action_button(id, title, spec)
	local t = u.theme
	local over = u:hovered(id)
	local button = b.div():id(id):press(id):role("button"):label(title)
		:row():items_center():gap(8):h(t.size.control):px(t.size.pad + 2):no_shrink():rounded(t.radius.control)
		:bg(over and t.control_hover or t.control):border(1, t.line)
		:motion({ seconds = t.motion, color = true })
		:child(b.text(title):weight(500))
	if spec ~= nil then
		button:child(b.text(kit.keys_of(spec)):color(t.ink_quiet))
	end
	return button
end

-- The card's frame: over the whole room, its ground the page's (it hides what the browser shows of its own
-- failure) unless `clear`, where the host's placeholder is to be seen.
local function frame(clear)
	local view = b.div():absolute():inset(0, 0, 0, 0):col():items_center():justify_center():gap(12):p(24)
	if not clear then
		view:bg("background")
	end
	return view
end

local function sentence(words, tone)
	return b.text(words):color(tone or "dim"):wrap():max_w(420):align("center")
end

-- The card for the tab in front, or nil where the browser shows the page (or is starting).
local function card(tab)
	local state = tab.state
	if state == nil then
		return nil
	end
	local status = state.status
	if status == "ready" then
		if state.error == nil then
			return nil
		end
		local port = tab.nothing or tab.answers
		local view = frame(false):child(kit.icon(port and "plug-off" or "alert-triangle", 24, "quiet"))
		view:child(b.text("Couldn't load " .. address.display(state.error.url)):weight(600):wrap():max_w(420):align("center"))
		if tab.nothing then
			view:child(sentence("Nothing is listening on :" .. tab.nothing .. " yet"))
		elseif tab.answers then
			view:child(sentence("Something is listening on :" .. tab.answers .. " now"))
		else
			view:child(sentence(state.error.message))
		end
		return view:child(action_button("card-reload", "Reload", commands.keys("reload")))
	elseif status == "starting" then
		return nil
	elseif status == "crashed" then
		return frame(false):child(kit.icon("bug", 24, "quiet")):child(sentence(state.message or "The browser stopped."))
			:child(action_button("card-reload", "Reload", commands.keys("reload")))
	elseif status == "no_route" then
		return frame(false):child(kit.icon("plug-off", 24, "quiet")):child(sentence(state.message or "There is no way to the machine this program runs on from here."))
	elseif status == "unsupported" then
		-- The host's placeholder says it shows no browser, in the middle of the room: the card's button
		-- stands under it.
		local view = frame(true):pt(72)
		return view:child(action_button("card-outside", "Open in Your Browser"))
	end
	-- A status the page does not know is a browser that is not ready.
	return frame(false):child(sentence(state.message or "The browser is not ready."))
end

-- The room: the browser of every tab that has had an address, the one in front shown. A blank tab has no
-- browser yet: a browser made for nothing shows "about:blank" for its title and has it in its history, so
-- it is made when the tab is given its address.
function room.view()
	local view = b.div():id("room"):grow(1):min_h(0):min_w(0):col()
	local active = tabs.active()
	for _, tab in S.tabs do
		if tab.opened then
			-- The keyboard can be sent to a browser by its name, as to a field.
			u:field(tab.key)
			local browser = b.browser(tab.key, { profile = S.boot.profile, windows = "page", keys = commands.browser_keys() })
			if tab == active then
				browser:grow(1):min_h(0)
			else
				browser:hidden()
			end
			view:child(browser)
			-- The card is not the browser's child: a press on the card's room is the card's, where a press in
			-- a browser's is the browser's.
			if tab == active then
				local over = card(tab)
				if over ~= nil then
					view:child(over)
				end
			end
		end
	end
	if active ~= nil and tabs.blank(active) then
		view:child(suggest.page())
	end
	return view
end

-- A press on a card's button.
function room.pressed(name)
	local tab = tabs.active()
	if name == "card-reload" then
		browse.reload(tab, true)
		return true
	elseif name == "card-outside" then
		if tab ~= nil then
			browse.open_outside(tabs.url_of(tab))
		end
		return true
	end
	return false
end

return room
