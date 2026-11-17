-- The app's own Lua, run once in every script's Luau before the script
-- (crates/script/src/sandbox.rs), while the globals can still be set. It's
-- handed the natives (filled in by the app afterwards: they're looked up
-- when called), the name of the app's global for Lua (`aventail`), the
-- objects' names, the things' names, and the states as { name, options }.
-- It gives back functions for the app alone.
local natives, folder, objects, things, states = ...
local made = {}

-- command("Name", { setting = value, … }) makes a command, as Lua's own way
-- of writing a LustScript component: its settings and a run function.
function command(name, settings)
  if type(name) ~= "string" then
    error("command needs a name first, such as command(\"Tidy\", { … })", 2)
  end
  if settings == nil then settings = {} end
  if type(settings) ~= "table" then
    error("command " .. name .. " wants its settings as a table after its name, such as { width = 0.2 }", 2)
  end
  local class = { __name = name, __settings = settings }
  class.__index = class
  made[#made + 1] = class
  return class
end

-- print writes in the output, like Script:say.
local tostring, select, concat = tostring, select, table.concat
function print(...)
  local n = select("#", ...)
  local parts = {}
  for i = 1, n do parts[i] = tostring((select(i, ...))) end
  natives.Script_say(nil, concat(parts, "\t"))
end

-- table.move is one loop in C that the time limit can't stop: bigger
-- ranges than any script needs are refused.
local move = table.move
table.move = function(a1, f, e, t, a2)
  if type(f) == "number" and type(e) == "number" and e - f > 1000000 then
    error("table.move can move at most a million items at once", 2)
  end
  return move(a1, f, e, t, a2)
end

-- Scripts may ask how much memory they use, but not stop the garbage
-- collector (which would make them run out bit by bit).
local gc = collectgarbage
collectgarbage = function(option)
  if option ~= nil and option ~= "count" then
    error("collectgarbage can only say how much memory is used (\"count\")", 2)
  end
  return gc("count")
end

-- A thing's calls, by name: shape:name() is natives.Shape_name(shape).
local function calls_of(owner)
  local prefix = owner .. "_"
  return function(_, key)
    if type(key) ~= "string" then return nil end
    local f = natives[prefix .. key]
    if f == nil then
      error(owner .. ":" .. key .. " isn't something scripts can call. The script reference lists every call.", 2)
    end
    return f
  end
end

-- Things the app hands scripts (shapes) are kept one per id, so the same
-- shape is always the same value (== works), and can't be changed.
local wrappers = {}
local ids = setmetatable({}, { __mode = "k" })
local kinds = setmetatable({}, { __mode = "k" })
for _, thing in ipairs(things) do
  local calls = calls_of(thing)
  local meta = {
    __index = function(t, key)
      if key == "id" then return ids[t] end
      return calls(t, key)
    end,
    __newindex = function()
      error("a " .. thing .. " from the app can't be changed this way", 2)
    end,
    __tostring = function(t) return thing .. " " .. tostring(ids[t]) end,
    __metatable = thing,
  }
  table.freeze(meta)
  local made_of_kind = {}
  wrappers[thing] = function(id)
    local w = made_of_kind[id]
    if w == nil then
      w = setmetatable({}, meta)
      ids[w] = id
      kinds[w] = thing
      made_of_kind[id] = w
    end
    return w
  end
end

-- The app's global for Lua: aventail.Scene:selected() and the rest, and the
-- states as their options' names (aventail.Mode.Polygons is "Polygons").
local global = {}
for _, object in ipairs(objects) do
  local reach = { __index = calls_of(object), __newindex = function()
    error(folder .. "." .. object .. " can't be changed", 2)
  end, __metatable = object }
  table.freeze(reach)
  global[object] = setmetatable({}, reach)
end
for _, state in ipairs(states) do
  local options = {}
  for _, option in ipairs(state[2]) do
    options[option] = option
  end
  table.freeze(options)
  global[state[1]] = options
end
table.freeze(global)
_G[folder] = global

-- For the app alone: a shape's value by its id; the id and kind of a value
-- the app made (nil for anything else); and the commands made so far.
local function wrap(kind, id) return wrappers[kind](id) end
local function unwrap(value) return ids[value], kinds[value] end
local function commands() local m = made; made = {}; return m end
return wrap, unwrap, commands
