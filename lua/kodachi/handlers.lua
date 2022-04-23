---@class Handlers
---@field _entries any
---@field _next_id number
local Handlers = {}

function Handlers:new()
  local o = {
    _entries = {},
    _next_id = 0,
  }

  setmetatable(o, self)
  self.__index = self
  return o
end

function Handlers:clear()
  self._entries = {}
  self._next_id = 0
end

function Handlers:insert(handler)
  local id = self._next_id
  self._next_id = id + 1
  self._entries[id] = handler
  return id
end

function Handlers:remove_by_id(id)
  self._entries[id] = nil
end

function Handlers:get(id)
  return self._entries[id]
end

return Handlers
