-- The bar under the tabs: Back, Forward and Reload, the address field, the zoom when it is not 100%, and
-- the line that shows a page loading. Every place on it keeps its own width whatever the page is doing:
-- a button the page cannot go to is faint, not gone, and Reload does not turn into Stop (Esc stops).

local kit = require("../lib/kit")
local address = require("../lib/address")
local commands = require("./commands")
local tabs = require("./tabs")
local browse = require("./browse")

local b, u, S, SIZE, act = kit.b, kit.u, kit.S, kit.SIZE, kit.act

local bar = {}

-- The field's name, as the library gives it.
local function field_name()
	return u:name("address", "field")
end

-- Whether the keyboard is in the address field.
function bar.focused()
	return u:keyboard() == field_name()
end

-- Whether the keyboard has let go of the field with what was written still there. A press on the page takes
-- the keyboard out of the host's field before the page is told of the press, which may be on a row of the
-- suggestions: what was written waits for the press, as a picker's card does (a press elsewhere, Esc or
-- Enter, a change of tab, or the keyboard going to something that has a name, ends it).
function bar.let_go()
	return S.typed and u:keyboard() == "page"
end

-- Whether an address is being written: the keyboard is in the field, or has let go of it (`bar.let_go`).
function bar.writing()
	return bar.focused() or bar.let_go()
end

-- What the field says when the person is not writing in it: the address of the tab in front, with no
-- scheme. Where the person is writing, it is left to them.
function act.fill_address()
	if bar.writing() then
		return
	end
	local tab = tabs.active()
	S.address.value = tab and address.display(tabs.url_of(tab)) or ""
	S.typed = false
end

-- Puts the keyboard in the address field with the whole address in it (with its scheme), selected, for the
-- person to write another over it or copy it: ⌘L, a click in the field, and a new tab.
function act.focus_address()
	local tab = tabs.active()
	S.address.value = tab and tabs.url_of(tab) or ""
	S.select_address = true
	S.typed = false
	S.cursor = nil
	kit.gather(u:focus("address"))
end

-- The person is done with the address field: what it holds is the tab's address again, and the keyboard
-- is the page's (the browser's, where the tab has a page).
function act.leave_address()
	S.typed = false
	S.cursor = nil
	local tab = tabs.active()
	S.address.value = tab and address.display(tabs.url_of(tab)) or ""
	if tab ~= nil and tab.opened then
		kit.gather(u:focus(tab.key))
	end
end

-- Goes to what was typed: an address, or a search.
function act.submit_address(text)
	local url = address.resolve(text)
	local tab = tabs.active()
	if url == nil or tab == nil then
		act.leave_address()
		return
	end
	act.go_to(tab, url)
end

-- Takes `tab` to `url` and leaves the address field for the page. A blank tab becomes the page.
function act.go_to(tab, url)
	S.typed = false
	S.cursor = nil
	S.address.value = address.display(url)
	browse.go(tab, url)
	kit.gather(u:focus(tab.key))
end

-- The zoom chip: how large the page is shown, where it is not its own size.
local function zoom_chip(zoom)
	local t = u.theme
	local over = u:hovered("zoom-reset")
	return b.div():id("zoom-reset"):press("zoom-reset"):role("button"):label("Actual Size")
		:row():items_center():h(16):px(6):no_shrink():rounded(t.radius.chip)
		:bg(over and t.control_hover or t.chip):color(t.ink_dim):text_size(t.text.tiny)
		:tip("Actual Size", commands.label("zoom_reset"))
		:child(b.text(string.format("%d%%", math.floor(zoom * 100 + 0.5))))
end

local function button(id, glyph, label, command, enabled)
	local t = u.theme
	return u:icon_button(id, {
		icon = "assets/icons/" .. glyph .. ".svg",
		label = label,
		keys = commands.keys(command),
		disabled = not enabled,
		size = t.size.control,
	})
end

function bar.view()
	local t = u.theme
	local tab = tabs.active()
	local state = tab and tab.state
	local ready = tab ~= nil and tab.opened and state ~= nil and state.status == "ready"
	local trail = {}
	if ready and state.zoom ~= nil and math.abs(state.zoom - 1) > 0.001 then
		trail[1] = zoom_chip(state.zoom)
	end
	local field = u:input("address", {
		state = S.address,
		label = "Address",
		hint = "Search or enter address",
		escape = true,
		select = S.select_address or nil,
		trail = trail,
	})
	S.select_address = false
	return b.div():id("toolbar"):row():items_center():gap(4):px(SIZE.pad):py(4):bg(t.surface)
		:child(button("back", "arrow-left", "Back", "back", ready and state.back))
		:child(button("forward", "arrow-right", "Forward", "forward", ready and state.forward))
		:child(button("reload", "refresh", "Reload", "reload", ready))
		:child(b.div():id("address-box"):grow(1):min_w(0):child(field))
end

-- The line under the bar while the page loads: two points over the top of the room, taking no room of its
-- own. It comes and goes by fading, which Reduce Motion leaves out.
function bar.line()
	local tab = tabs.active()
	if tab == nil or not tab.opened or tab.state == nil or not tab.state.loading then
		return nil
	end
	return b.div():anchor("toolbar", "below", { gap = 0 }):id("loading"):w(S.width):h(SIZE.line):bg("accent")
		:motion({ seconds = 0.24, enter = { opacity = 0, seconds = 0.24 }, exit = { opacity = 0, seconds = 0.24 } })
end

-- An action of the buttons and the field. Whether it was theirs.
function bar.action(action)
	local from, kind = action.from, action.kind
	if from == "back" and kind == "press" then
		act.run("back")
	elseif from == "forward" and kind == "press" then
		act.run("forward")
	elseif from == "reload" and kind == "press" then
		act.run("reload")
	elseif from == "address" then
		if kind == "change" then
			S.typed = true
			S.cursor = nil
		elseif kind == "submit" then
			local picked = act.picked()
			if picked ~= nil then
				act.pick(picked)
			else
				act.submit_address(action.text)
			end
		elseif kind == "escape" then
			act.leave_address()
		end
	else
		return nil
	end
	return true
end

-- A press on the zoom chip: the page's own size.
function bar.pressed(name)
	if name == "zoom-reset" then
		browse.zoom(tabs.active(), 0)
		return true
	end
	return false
end

return bar
