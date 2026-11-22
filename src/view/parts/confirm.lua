-- The one question the page asks before it does something that cannot be undone: whether to clear what a
-- site keeps. The commands that ask it are in the list only (they have no keys, so a key is never what
-- signs a person out); the question names the site, and "Cancel" is what Enter does. Where the address
-- names a port it says what the person may not know: the cookies are the name's, so every port loses them.
-- It does not say which computers the person is signed out on: the host does not tell a page whether the
-- profile is shared.

local kit = require("../lib/kit")
local address = require("../lib/address")
local tabs = require("./tabs")
local browse = require("./browse")

local u, S = kit.u, kit.S

local confirm = {}

-- The site the tab in front is at, by the host's rule (`address.site_of`), and the port its address names;
-- nil where it is at none.
function confirm.site_of_tab(tab)
	if tab == nil or not tab.opened then
		return nil
	end
	return address.site_of(tab.state)
end

-- The question while it is still the one that was asked: the tab is there and at the site the question
-- names. What is cleared is the site the browser is at when the command arrives, so a "yes" to one site is
-- never taken for another's: a page that went on to another site of itself, or a tab that closed, ends the
-- question.
local function still()
	local asking = S.confirm
	if asking == nil then
		return nil
	end
	local tab = tabs.by_key(asking.tab)
	if tab == nil or confirm.site_of_tab(tab) ~= asking.host then
		S.confirm = nil
		return nil
	end
	return asking
end

-- Asks whether to clear what the site of `tab` keeps, at every port. Nothing is asked where the browser is at
-- no site.
function confirm.ask_site(tab)
	local host, port = confirm.site_of_tab(tab)
	if host ~= nil then
		S.confirm = { tab = tab.key, host = host, port = port ~= nil }
	end
end

-- The question, drawn into `root`.
function confirm.view(root)
	local asking = still()
	if asking == nil then
		return
	end
	local what = asking.port and ("Every port of " .. asking.host .. " shares its cookies, so this clears them all.")
		or "Cookies, storage and service workers."
	root:child(u:dialog("clear_site", {
		title = "Clear " .. asking.host .. "'s data?",
		message = what .. " This signs you out of " .. asking.host .. ".",
		actions = {
			{ key = "cancel", title = "Cancel", primary = true },
			{ key = "clear", title = "Clear Site Data", danger = true },
		},
	}))
end

-- An answer of the question. Whether it was the question's.
function confirm.action(action)
	if action.from ~= "clear_site" then
		return nil
	end
	local had = S.confirm ~= nil
	local asking = still()
	S.confirm = nil
	if action.kind == "answer" and action.key == "clear" then
		if asking ~= nil then
			browse.command(tabs.by_key(asking.tab), "clear_site_data")
		elseif had then
			kit.toast("The page is at another site now: nothing was cleared.", "warn")
		end
	end
	return true
end

return confirm
