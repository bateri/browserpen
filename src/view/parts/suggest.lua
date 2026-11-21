-- What is offered for an address: the ports the machine the program runs on listens on, the tabs that
-- are open and the addresses the profile has been to. A blank tab shows the ports and the recent ones
-- as two lists in the room; an address being written on a tab with a page offers the same, narrowed to
-- what is typed, in a card under the field. The arrow keys go through them (one cursor over all of
-- them), Enter goes to the one the cursor is on, and a click does too. With no cursor Enter goes to
-- what was typed.

local kit = require("../lib/kit")
local address = require("../lib/address")
local tabs = require("./tabs")
local bar = require("./bar")

local b, u, S, SIZE, act = kit.b, kit.u, kit.S, kit.SIZE, kit.act

local suggest = {}

-- How many of each there are at the most: in the blank tab's lists, and in the card under the field.
local LISTED = { ports = 8, recent = 10 }
local CARD = { ports = 4, tabs = 3, recent = 6 }

local ROW = 26

local function trimmed(text)
	return string.lower(string.match(text or "", "^%s*(.-)%s*$"))
end

local function has(words, query)
	return query == "" or string.find(string.lower(words), query, 1, true) ~= nil
end

local function port_row(port)
	return {
		key = "port:" .. port.port,
		kind = "port",
		title = ":" .. port.port,
		hint = port.name,
		url = "http://localhost:" .. port.port .. "/",
	}
end

-- The rows for what is typed, in the order they are offered: ports, tabs, recent addresses. Nothing
-- typed offers the ports and the recent ones as they are.
local function compute()
	local query = trimmed(S.typed and S.address.value or "")
	local blank = tabs.active() ~= nil and tabs.blank(tabs.active())
	local rows = {}
	local counts = { ports = 0, tabs = 0, recent = 0 }
	local limit = (blank and query == "") and LISTED or CARD
	for _, port in S.ports do
		if counts.ports < limit.ports and has(":" .. port.port .. " " .. port.name .. " localhost", query) then
			counts.ports += 1
			local row = port_row(port)
			row.section = "Ports"
			rows[#rows + 1] = row
		end
	end
	if query ~= "" then
		for _, tab in S.tabs do
			local url = tabs.url_of(tab)
			if tab.key ~= S.active and tab.opened and counts.tabs < CARD.tabs and has(tabs.title_of(tab) .. " " .. url, query) then
				counts.tabs += 1
				rows[#rows + 1] = { key = "tab:" .. tab.key, kind = "tab", title = tabs.title_of(tab), hint = "Tab", tab = tab.key, section = "Tabs" }
			end
		end
	end
	for _, visit in S.recent do
		if counts.recent < limit.recent and has(visit.title .. " " .. visit.url, query) then
			counts.recent += 1
			local shown = address.display(visit.url)
			rows[#rows + 1] = {
				key = "recent:" .. visit.url,
				kind = "recent",
				title = visit.title ~= "" and visit.title or shown,
				hint = visit.title ~= "" and shown or nil,
				url = visit.url,
				section = "Recent",
			}
		end
	end
	return rows
end

-- Whether the card under the field is shown: an address is being written on a tab that has a page (the
-- keyboard is in the field, or only let go of it: `bar.writing`).
function suggest.card_shown()
	local tab = tabs.active()
	return tab ~= nil and not tabs.blank(tab) and bar.writing() and S.typed and #S.rows > 0
end

-- Whether the arrow keys are for the rows: the lists of a blank tab or the card, with the field the
-- keyboard is in.
function suggest.navigating()
	local tab = tabs.active()
	if tab == nil or not bar.focused() or #S.rows == 0 then
		return false
	end
	return tabs.blank(tab) or suggest.card_shown()
end

function suggest.refresh()
	S.rows = compute()
	if S.cursor ~= nil then
		local found = false
		for _, row in S.rows do
			found = found or row.key == S.cursor
		end
		if not found then
			S.cursor = nil
		end
	end
end

-- The row the cursor is on, where the rows are offered.
function act.picked()
	if S.cursor == nil or not suggest.navigating() then
		return nil
	end
	for _, row in S.rows do
		if row.key == S.cursor then
			return row
		end
	end
	return nil
end

-- Goes to what a row offers.
function act.pick(row)
	if row.kind == "tab" then
		S.typed = false
		S.cursor = nil
		tabs.select(row.tab)
		return
	end
	local tab = tabs.active()
	if tab ~= nil then
		act.go_to(tab, row.url)
	end
end

-- The cursor `step` rows on (round the ends), the first or the last from none.
function suggest.step(step)
	local count = #S.rows
	if count == 0 then
		return
	end
	local at = nil
	for index, row in S.rows do
		if row.key == S.cursor then
			at = index
		end
	end
	if at == nil then
		at = step > 0 and 1 or count
	else
		at = (at - 1 + step) % count + 1
	end
	S.cursor = S.rows[at].key
end

-- --- Drawing ----------------------------------------------------------------------------------------------------

local function row_view(row, width)
	local t = u.theme
	local name = "row/" .. row.key
	local chosen = S.cursor == row.key
	local over = u:hovered(name)
	local view = b.div():id(name):press(name):role("button"):label(row.title .. (row.hint and (", " .. row.hint) or ""))
		:row():items_center():gap(8):h(ROW):px(8):rounded(t.radius.control)
		:bg(chosen and t.chosen or (over and t.hover or nil))
		:motion({ seconds = t.motion, color = true })
	if width ~= nil then
		view:w(width):no_shrink()
	end
	view:child(b.text(row.title):weight(row.kind == "port" and 600 or 400):truncate():max_w(360):no_shrink(row.kind == "port"))
	if row.hint ~= nil then
		view:child(b.text(row.hint):color(t.ink_dim):truncate():min_w(0):grow(1))
	end
	return view
end

-- The card under the address field, over the page.
function suggest.card()
	if not suggest.card_shown() then
		return nil
	end
	local t = u.theme
	local width = math.min(S.width - 2 * SIZE.pad, 560)
	local card = b.div():anchor("address-box", "below", { gap = 4 }):id("suggestions"):col():gap(1):p(4):w(width)
		:bg(t.popup):rounded(t.radius.card):border(1, t.line)
		:shadow(t.elevation.dx, 8, 24, b.alpha("shadow", 0.4))
	for _, row in S.rows do
		card:child(row_view(row))
	end
	return card
end

local function heading(words)
	return b.text(words):color(u.theme.ink_quiet):text_size(u.theme.text.small):px(8):pt(8):pb(2)
end

-- What a blank tab shows in the room: the ports of the machine and the addresses the profile has been to
-- (narrowed to what is typed), as two lists the arrow keys go through.
function suggest.page()
	local t = u.theme
	local box = b.div():col():gap(0):w_full():max_w(560):px(SIZE.pad)
	local section = nil
	for _, row in S.rows do
		if row.section ~= section then
			section = row.section
			box:child(heading(section))
		end
		box:child(row_view(row))
	end
	if #S.rows == 0 then
		local words = trimmed(S.typed and S.address.value or "") == "" and "Type an address, a port or a search" or "No ports or addresses match"
		box:child(b.text(words):color(t.ink_quiet):px(8):py(8))
	end
	return b.div():size_full():col():items_center():pt(40):bg("background"):child(box)
end

-- A press on a row.
function suggest.pressed(name)
	local key = string.match(name or "", "^row/(.*)$")
	if key == nil then
		return false
	end
	for _, row in S.rows do
		if row.key == key then
			act.pick(row)
			return true
		end
	end
	return false
end

return suggest
