-- The two things that say what can be done: the list of all commands (F1: a palette of the commands that
-- mean something now, each with its keys) and the sheet of shortcuts (⌘?: every command with its keys, in
-- its group).

local kit = require("../lib/kit")
local commands = require("./commands")

local b, u, S, act = kit.b, kit.u, kit.S, kit.act

local help = {}

-- Opens the palette of the commands, with nothing written in it.
function help.show_commands()
	S.palette.query = ""
	S.palette.field.value = ""
	S.palette.cursor = nil
	S.overlay = "commands"
end

function help.show_shortcuts()
	S.overlay = "shortcuts"
end

function help.hide()
	S.overlay = nil
end

local function caps(spec, also)
	local row = b.div():row():items_center():gap(4):no_shrink():child(u:kbd(spec))
	if also ~= nil then
		row:child(b.text("to"):color(u.theme.ink_quiet):text_size(u.theme.text.small)):child(u:kbd(also))
	end
	return row
end

local function sheet_body()
	local t = u.theme
	local body = b.div():col():gap(14):p(14):scroll():min_h(0)
	for _, group in commands.help() do
		local section = b.div():col():gap(2)
		section:child(b.text(group.title):color(t.ink_quiet):text_size(t.text.small):pb(2))
		for _, item in group.items do
			section:child(b.div():row():items_center():gap(12):h(t.size.row)
				:child(b.text(item.title):grow(1):min_w(0):truncate())
				:child(item.keys and caps(item.keys, item.also) or b.div()))
		end
		body:child(section)
	end
	return b.div():col():min_h(0)
		:child(b.div():row():items_center():px(14):pt(12):child(b.text("Keyboard Shortcuts"):weight(600):grow(1)))
		:child(body)
end

-- What is over the page, drawn into `root`.
function help.view(root)
	if S.overlay == "commands" then
		root:child(u:palette("palette", {
			items = commands.items(),
			state = S.palette,
			label = "Commands",
			hint = "Find a command",
			rows = 8,
			footer = { { keys = "enter", title = "Run" }, { keys = "escape", title = "Close" } },
		}))
	elseif S.overlay == "shortcuts" then
		root:child(u:modal("shortcuts", { width = 460, label = "Keyboard Shortcuts" }, sheet_body()))
	end
end

-- An action of the palette or the sheet. Whether it was theirs.
function help.action(action)
	local from, kind = action.from, action.kind
	if from == "palette" then
		if kind == "dismiss" then
			help.hide()
		elseif kind == "pick" then
			help.hide()
			act.run(action.item.key)
		end
		return true
	elseif from == "shortcuts" then
		if kind == "dismiss" then
			help.hide()
		end
		return true
	end
	return nil
end

return help
