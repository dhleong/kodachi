---@class KodachiState
---@field bufnr number
---@field connection_id number|nil
---@field uri string|nil
---@field exited boolean|nil
---@field socket Socket
---@field _mappings any
local KodachiState = {}

function KodachiState:new(o)
  o._mappings = o._mappings or {}
  setmetatable(o, self)
  self.__index = self
  return o
end

---Create a keymapping in normal mode for the buffer associated with this state. These mappings
-- will also be available in the composer, for convenience.
-- `rhs` may be:
-- - string: Text to be sent
-- - fn: A function to be invoked with the state
function KodachiState:map(lhs, rhs)
  self._mappings[lhs] = rhs
  vim.api.nvim_buf_set_keymap(
    self.bufnr, 'n', lhs,
    "<cmd>lua require'kodachi.states'[" .. self.bufnr .. "]:_perform_map('" .. lhs .. "')<cr>",
    {
      noremap = true,
      silent = true,
    }
  )
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

function KodachiState:_perform_map(lhs)
  local rhs = self._mappings[lhs]
  if not rhs then
    print('Nothing mapped to:', lhs)
    return
  end

  if type(rhs) == 'string' then
    self:send(rhs)
  elseif type(rhs) == 'function' then
    rhs(self)
  end
end

return KodachiState
