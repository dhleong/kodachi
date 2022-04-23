local Handlers = require 'kodachi.handlers'
local matchers = require 'kodachi.matchers'

---@alias KodachiEvent 'connected'|'disconnected'

---@param state KodachiState
---@param handler fun(socket:Socket)
local function with_socket(state, handler)
  if not state.connection_id then
    return false
  end

  return handler(state.socket)
end

---@class KodachiState
---@field bufnr number
---@field connection_id number|nil
---@field uri string|nil
---@field exited boolean|nil
---@field socket Socket
---@field _events any|nil
---@field _mappings any
---@field _triggers Handlers
local KodachiState = {}

function KodachiState:new(o)
  o._mappings = o._mappings or {}
  o._triggers = o._triggers or Handlers:new()
  setmetatable(o, self)
  self.__index = self
  return o
end

function KodachiState:cleanup()
  self._triggers:clear()
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
    self:_state_method_cmd("_perform_map('" .. lhs .. "')"),
    {
      noremap = true,
      silent = true,
    }
  )
end

---Register an event handler
---@param event KodachiEvent
function KodachiState:on(event, handler)
  if not self._events then
    self._events = {}
    self.socket:listen(function (message)
      local events = self._events[string.lower(message.type)]
      if events then
        vim.schedule(function ()
          for _, saved_handler in ipairs(events) do
            saved_handler(message)
          end
        end)
      end
    end)

    -- Special cases:
    if event == 'connected' and self.connection_id then
      handler { id = self.connection_id }
    end
  end

  self._events[event] = handler
end

---@param matcher MatcherSpec|string
function KodachiState:trigger(matcher, handler)
  matcher = matchers.inflate(matcher)
  return with_socket(self, function (socket)
    local id = self._triggers:insert(handler)
    socket:request {
      type = "Trigger",
      matcher = matcher,
      handler_id = id,
    }
  end)
end

---Send some text to the connection associated with this state
---@param text string
function KodachiState:send(text)
  return with_socket(self, function (socket)
    socket:request {
      type = "Send",
      connection = self.connection_id,
      text = text,
    }
  end)
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

function KodachiState:_state_method_call(method_call)
  return "require'kodachi.states'[" .. self.bufnr .. "]:" .. method_call
end

function KodachiState:_state_method_cmd(method_call)
  return '<cmd>lua ' .. self:_state_method_call(method_call) .. '<cr>'
end

return KodachiState
