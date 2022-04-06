---@class KodachiState
---@field connection_id number|nil
---@field exited boolean|nil
---@field socket Socket
local KodachiState = {}

function KodachiState:new(o)
  setmetatable(o, self)
  self.__index = self
  return o
end

---Send some text to the connection associated with this state
---@param text string
function KodachiState:send(text)
  if not self.connection_id then
    return false
  end

  self.socket:request {
    type = "Send",
    connection = self.connection_id,
    text = text,
  }

  return true
end

return KodachiState
