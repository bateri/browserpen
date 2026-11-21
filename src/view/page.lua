-- The page of browserpen: the face of a web browser, drawn by the terminal in its own font and theme.
--
-- Each tab is a browser of the terminal's (`pencere.browser`), which loads and draws web pages in its
-- room; around it the tabs, the bar with the address field, the find bar and the cards that say what a
-- browser that is not ready cannot. The program lists the ports of the machine it runs on, keeps the
-- addresses the profile has been to and says whether a port answers; the page talks to it only by the
-- calls of docs/calls.md, and draws with the kit's library (`@pencere/ui`).

local kit = require("./lib/kit")
local address = require("./lib/address")
local commands = require("./parts/commands")
local tabs = require("./parts/tabs")
local browse = require("./parts/browse")
local bar = require("./parts/bar")
local suggest = require("./parts/suggest")
local room = require("./parts/room")
local find = require("./parts/find")
local help = require("./parts/help")

local b, u, S, act, lib = kit.b, kit.u, kit.S, kit.act, kit.lib

-- --- The commands -------------------------------------------------------------------------------------------
--
-- What can be done, once (parts/commands.lua): the keys, the host's list, the palette, the shortcuts sheet and
-- the keys the browser keeps for the page are made of it. The conditions are named in the table and given
-- here; they say what the page is showing, never where the keyboard is.

local function active_state()
	local tab = tabs.active()
	return tab ~= nil and tab.opened, tab and tab.state
end

commands.when = {
	tabs = function()
		return #S.tabs > 1
	end,
	closed = function()
		return #S.closed > 0
	end,
	page = function()
		return (active_state())
	end,
	back = function()
		local opened, state = active_state()
		return opened and state ~= nil and state.status == "ready" and state.back
	end,
	forward = function()
		local opened, state = active_state()
		return opened and state ~= nil and state.status == "ready" and state.forward
	end,
	loading = function()
		local opened, state = active_state()
		return opened and state ~= nil and state.loading
	end,
	finding = function()
		return S.find.open
	end,
	navigating = function()
		return suggest.navigating()
	end,
}

local cmds = lib.commands(commands.for_library())

-- Copies the address of `tab` to the pasteboard.
function act.copy_address(tab)
	if tab ~= nil and tab.opened then
		kit.gather({ copy = tabs.url_of(tab) })
	end
end

function act.reload(tab, cache)
	browse.reload(tab, cache)
end

-- Does the command `id`.
local function run(id)
	local tab = tabs.active()
	if id == "address" then
		act.focus_address()
	elseif id == "copy_address" then
		act.copy_address(tab)
	elseif id == "new_tab" then
		tabs.open(nil)
	elseif id == "close_tab" then
		tabs.close(S.active)
	elseif id == "reopen_tab" then
		tabs.reopen()
	elseif id == "next_tab" then
		tabs.step(1)
	elseif id == "previous_tab" then
		tabs.step(-1)
	elseif id == "tab_last" then
		tabs.go_to(nil)
	elseif string.match(id, "^tab_%d$") then
		tabs.go_to(tonumber(string.sub(id, 5)))
	elseif id == "reload" then
		browse.reload(tab, true)
	elseif id == "reload_hard" then
		browse.reload(tab, false)
	elseif id == "back" or id == "forward" or id == "stop" or id == "devtools" then
		browse.command(tab, id)
	elseif id == "zoom_in" then
		browse.zoom(tab, 1)
	elseif id == "zoom_out" then
		browse.zoom(tab, -1)
	elseif id == "zoom_reset" then
		browse.zoom(tab, 0)
	elseif id == "find" then
		act.open_find()
	elseif id == "find_next" or id == "find_previous" then
		browse.command(tab, id)
	elseif id == "commands" then
		help.show_commands()
	elseif id == "shortcuts" then
		help.show_shortcuts()
	elseif id == "suggest_next" then
		suggest.step(1)
	elseif id == "suggest_previous" then
		suggest.step(-1)
	elseif id == "quit" then
		kit.call("quit", {}, function(result)
			if result ~= nil and result.unsupported then
				kit.toast(result.unsupported, "warn", 6)
				return true
			end
			return nil
		end)
	end
end

act.run = run

-- --- The page -----------------------------------------------------------------------------------------------------

local function booting()
	local box = b.div():size_full():col():items_center():justify_center():gap(8):bg("background"):family("terminal")
	if S.boot_error then
		box:child(b.text("The program did not answer"):weight(650))
		box:child(b.text(S.boot_error):color("dim"):wrap():max_w(520):align("center"))
	else
		box:child(kit.spinner(20, "quiet"))
	end
	return box
end

-- The keyboard went into or out of the address field of itself (a click, a tab): what the field holds is
-- the tab's whole address, selected, while the person is in it, and its address as it is read when they are
-- not.
local was_in_address = false
local function address_follows_keyboard()
	local inside = bar.focused()
	if inside and not was_in_address and not S.typed then
		local tab = tabs.active()
		S.address.value = tab and tabs.url_of(tab) or ""
		S.select_address = true
	elseif was_in_address and not inside then
		S.typed = false
		S.cursor = nil
		local tab = tabs.active()
		S.address.value = tab and address.display(tabs.url_of(tab)) or ""
	end
	was_in_address = inside
end

local function render(size, now, reduce_motion)
	u:begin(size, now, reduce_motion)
	S.now, S.width, S.height = now, size.w, size.h
	if not S.boot then
		return u:finish(booting())
	end
	lib.toasts_prune(S.toasts, now)
	address_follows_keyboard()
	suggest.refresh()
	local root = b.div():size_full():col():bg("background"):color("foreground"):family("terminal")
	root:child(tabs.view())
	root:child(bar.view())
	root:child(room.view())
	root:child(bar.line())
	root:child(find.view())
	root:child(suggest.card())
	help.view(root)
	if #S.toasts > 0 then
		root:child(u:toasts("toasts", { items = S.toasts }))
	end
	cmds:write(u, root)
	return u:finish(root)
end

-- --- What the person does ---------------------------------------------------------------------------------------

-- What an action of a component comes to.
local function on_action(action)
	local reply = tabs.action(action)
	if reply == nil then
		reply = bar.action(action)
	end
	if reply == nil then
		reply = find.action(action)
	end
	if reply == nil then
		reply = help.action(action)
	end
	if reply == nil and action.from == "commands" and action.kind == "command" then
		run(action.id)
		reply = true
	elseif reply == nil and action.from == "toasts" and action.kind == "close" then
		for index, item in S.toasts do
			if item.key == action.key then
				table.remove(S.toasts, index)
				break
			end
		end
		reply = true
	end
	if reply == nil then
		reply = true
	end
	return reply
end

-- A key no component took: Esc, which stops what the page is doing, from the narrowest to the widest.
local function key(event)
	if event.key == "escape" and (event.mods == nil or not (event.mods.command or event.mods.control or event.mods.alt or event.mods.shift)) then
		if S.overlay ~= nil then
			help.hide()
			return true
		elseif S.find.open then
			act.close_find()
			return true
		end
		local tab = tabs.active()
		local opened, state = active_state()
		if opened and state ~= nil and state.loading then
			browse.command(tab, "stop")
			return true
		end
	end
	return nil
end

-- The pointer is a hand over what answers a press and the arrow elsewhere; a shape the library asks for
-- stays its own.
local POINTER_INPUTS = {
	move = true,
	leave = true,
	press = true,
	release = true,
	wheel = true,
}
local pointer_at = nil
local pressed_on = nil

local function hand(reply, at)
	local shape = type(reply) == "table" and reply.cursor or nil
	if (shape ~= nil and shape ~= "arrow") or u.cursor ~= nil or u.capture ~= nil then
		if shape == nil and u.cursor ~= nil then
			return b.merge(reply, { cursor = u.cursor })
		end
		return reply
	end
	local name = at ~= nil and pencere.layout.hit(at) or nil
	if name ~= nil and (name == "zoom-reset" or string.match(name, "^row/") or string.match(name, "^card%-")) then
		return b.merge(reply, { cursor = "pointer" })
	end
	return reply
end

local function handle(event)
	local kind = event.type
	if kind == "browser" then
		local tab = tabs.by_key(event.browser)
		if tab ~= nil then
			browse.told(tab, event)
		end
		return kit.take(true)
	elseif kind == "browser_refused" then
		if event.reason == "ask" then
			kit.toast("This terminal does not know that yet.", "warn")
		else
			kit.toast("That is not an address a browser can open.", "warn")
		end
		return kit.take(true)
	elseif kind == "browser_window" then
		-- A link that opens a window: a tab of its own, behind this one where the person asked for that.
		tabs.open(event.url, { after = event.browser, background = event.background })
		if event.background then
			kit.toast("Opened behind this tab", "info", 2)
		end
		return kit.take(true)
	end
	local reply, actions, taken = u:handle(event)
	for _, action in actions do
		reply = b.merge(reply, on_action(action))
	end
	if kind == "leave" then
		pointer_at = nil
	elseif POINTER_INPUTS[kind] and event.x ~= nil and event.y ~= nil then
		pointer_at = { x = event.x, y = event.y }
	end
	if not taken then
		if kind == "key" then
			reply = b.merge(reply, key(event))
		elseif kind == "press" and (event.button or "left") == "left" then
			pressed_on = pencere.layout.hit({ x = event.x, y = event.y })
			-- A row of suggestions is chosen as the press goes down, as a menu's line is: the press takes the
			-- keyboard out of the address field, and the card under it is gone before the button is let go.
			if suggest.pressed(pressed_on) then
				pressed_on = nil
				reply = b.merge(reply, true)
			end
		elseif kind == "release" and (event.button or "left") == "left" then
			-- A click is a press and a release on the same element: the page's own parts (a row, a
			-- card's button, the zoom) are asked by their names; the library's components say theirs.
			local name = pencere.layout.hit({ x = event.x, y = event.y })
			if name ~= nil and name == pressed_on and (bar.pressed(name) or room.pressed(name)) then
				reply = b.merge(reply, true)
			end
			pressed_on = nil
		end
	end
	if POINTER_INPUTS[kind] then
		reply = hand(reply, pointer_at)
	end
	return kit.take(reply)
end

-- --- Starting ----------------------------------------------------------------------------------------------------------

-- How often the ports are asked for while they are shown (a blank tab, or an address being written).
local PORTS_EVERY = 2

local function ports_shown()
	local tab = tabs.active()
	return S.boot ~= nil and tab ~= nil and (tabs.blank(tab) or suggest.card_shown())
end

local function same_ports(a, c)
	if #a ~= #c then
		return false
	end
	for index, port in a do
		if port.port ~= c[index].port or port.name ~= c[index].name then
			return false
		end
	end
	return true
end

local function poll()
	if ports_shown() then
		kit.call("ports", {}, function(result)
			if result == nil or same_ports(S.ports, result.ports) then
				return nil
			end
			S.ports = result.ports
			return true
		end)
	end
	pencere.time.after(PORTS_EVERY, poll)
	return nil
end

kit.call("boot", {}, function(boot, err)
	if err then
		S.boot_error = err.message or err.code
		return true
	end
	S.boot = boot
	S.ports = boot.ports or {}
	S.recent = boot.recent or {}
	local start = boot.start ~= nil and address.resolve(boot.start) or nil
	tabs.open(start)
	pencere.time.after(PORTS_EVERY, poll)
	return true
end)

return {
	render = render,
	settle = function()
		return u:settle()
	end,
	handle = handle,
}
