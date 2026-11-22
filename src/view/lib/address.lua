-- What an address is to the page, in plain functions: the words a person types made into one a
-- browser is given (`resolve`), the address as it is read in the bar and in a tab (`display`,
-- `title_of`), and which local port an address is of (`local_port`).
--
-- A browser is given only printable ASCII with no space (`browser_refused`, "address"), so what is
-- written in another way is written in percent bytes here, and the person's Turkish search is a search.

local address = {}

-- Where words that are not an address are searched. The one place that names the search engine.
address.SEARCH = "https://www.google.com/search?q="

local LOCAL_HOSTS = { ["localhost"] = true, ["127.0.0.1"] = true, ["[::1]"] = true, ["0.0.0.0"] = true }

-- `text` with each byte that is not printable ASCII (and the space, and what a link would break on)
-- written as %XX. `keep` is what stays as it is besides the letters and digits.
local function percent(text, keep)
	return (string.gsub(text, "[^%w" .. keep .. "]", function(char)
		return string.format("%%%02X", string.byte(char))
	end))
end

-- A web address as it was written, with only what a browser is not given put right.
local function encoded(url)
	return (string.gsub(url, '[^\33-\126]', function(char)
		return string.format("%%%02X", string.byte(char))
	end))
end

-- Words as a search's query: a space is a `+`, the rest of what is not a letter or a digit is %XX.
local function query(words)
	return (string.gsub(percent(words, "%-%._~ "), " ", "+"))
end
address.query = query

-- Whether `host` is a host a developer is at: this machine's names, or an IPv4 address (a LAN's
-- dev server is not served over https).
local function is_local(host)
	local lower = string.lower(host)
	if LOCAL_HOSTS[lower] or string.sub(lower, -10) == ".localhost" then
		return true
	end
	return string.match(lower, "^%d+%.%d+%.%d+%.%d+$") ~= nil
end

-- The host and the port of `authority` (`localhost:3000`, `[::1]:3000`, `example.com`), or nil.
local function split(authority)
	local host, port = string.match(authority, "^(%[[%x:.]+%]):(%d+)$")
	if host then
		return host, tonumber(port)
	end
	if string.match(authority, "^%[[%x:.]+%]$") then
		return authority, nil
	end
	host, port = string.match(authority, "^([^:%[%]]+):(%d+)$")
	if host then
		return host, tonumber(port)
	end
	if string.match(authority, "^[^:%[%]]+$") then
		return authority, nil
	end
	return nil, nil
end

-- The address the words a person typed come to, or nil where they said nothing:
--   `3000`, `:3000`                   http://localhost:3000
--   `localhost`, `127.0.0.1`, `[::1]`, `0.0.0.0`, a name with no dot and a port (`myhost:8080`),
--   an IPv4 address                   http://, as written
--   an address with a scheme          as it is written
--   a dotted name with no space       https://
--   anything else                     a search
function address.resolve(words)
	words = string.match(words or "", "^%s*(.-)%s*$")
	if words == "" then
		return nil
	end
	if string.match(words, "^%a[%w+.-]*://") or string.lower(words) == "about:blank" or string.lower(string.sub(words, 1, 5)) == "data:" then
		return encoded(words)
	end
	if not string.find(words, "%s") then
		local port, rest = string.match(words, "^:?(%d+)$"), ""
		if port == nil then
			port, rest = string.match(words, "^:?(%d+)([/?#].*)$")
		end
		if port and tonumber(port) >= 1 and tonumber(port) <= 65535 then
			return encoded("http://localhost:" .. tonumber(port) .. rest)
		end
		local authority, rest_of = string.match(words, "^([^/?#]+)([/?#].*)$")
		if authority == nil then
			authority, rest_of = words, ""
		end
		local host, number = split(authority)
		if host ~= nil then
			if is_local(host) then
				return encoded("http://" .. authority .. rest_of)
			elseif number ~= nil and not string.find(host, ".", 1, true) then
				return encoded("http://" .. authority .. rest_of)
			elseif string.find(host, ".", 1, true) and not string.match(host, "^%.") and not string.match(host, "%.$") then
				return encoded("https://" .. authority .. rest_of)
			end
		end
	end
	return address.SEARCH .. query(words)
end

-- An address as it is read in the bar when the person is not writing in it: no scheme, and no slash
-- after a host alone (`localhost:3000/about`), where what is written comes back to the same address
-- (`example.com` is https, so an http address of it keeps its scheme).
function address.display(url)
	if url == nil or url == "" or string.lower(url) == "about:blank" then
		return ""
	end
	local shown = string.gsub(url, "^%a[%w+.-]*://", "")
	shown = string.gsub(shown, "^([^/?#]*)/$", "%1")
	local back = address.resolve(shown)
	if back ~= nil and string.gsub(back, "/$", "") == string.gsub(url, "/$", "") then
		return shown
	end
	return url
end

-- The host, the port and the rest of an http(s) address, or nil.
local function parts(url)
	local rest = string.match(url or "", "^%a[%w+.-]*://(.*)$")
	if rest == nil then
		return nil
	end
	local authority = string.match(rest, "^([^/?#]*)")
	authority = string.gsub(authority, "^.*@", "")
	return split(authority)
end

-- The port of an address that is this machine's (`localhost`, `127.0.0.1`, `[::1]`, `0.0.0.0`), or nil.
function address.local_port(url)
	local host, port = parts(url)
	if host ~= nil and LOCAL_HOSTS[string.lower(host)] and port ~= nil then
		return port
	end
	return nil
end

-- Whether `url` is one a page of the web is at, as a recent address: not a blank page.
function address.is_page(url)
	return url ~= nil and string.match(url, "^https?://") ~= nil
end

-- The name of the site an http(s) address is at, in lower case (no scheme or user), and the port the address
-- names (nil where it names none), or nil.
local function site_name(url)
	url = string.lower(url or "")
	if not address.is_page(url) then
		return nil
	end
	return parts(url)
end

-- The site a browser is at, for what is done to "this site": the name of its page's address, or, where
-- that is none (a load that failed leaves the browser at about:blank), of the address that failed; nil where
-- neither is a page of the web; and the port that address names, if any (what is cleared is the name's, at every
-- port). It is the host's rule too, so that what a person is asked about is what the host deletes.
function address.site_of(state)
	if state == nil then
		return nil
	end
	local name, port = site_name(state.url)
	if name == nil and state.error ~= nil then
		name, port = site_name(state.error.url)
	end
	return name, port
end

-- What a tab is called: the page's title, else its address as it is read, else what a blank tab is.
function address.title_of(url, title)
	if title ~= nil and title ~= "" and string.lower(title) ~= "about:blank" then
		return title
	end
	local shown = address.display(url)
	if shown ~= "" then
		return shown
	end
	return "New Tab"
end

return address
