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
---@field _triggers Handlers|nil
local KodachiState = {}

function KodachiState:new(o)
  o._mappings = o._mappings or {}
  setmetatable(o, self)
  self.__index = self
  return o
end

function KodachiState:cleanup()
  if self._triggers then
    self._triggers:clear()
    if self.socket and self.connection_id then
      self.socket:notify {
        type = "Clear",
        connection_id = self.connection_id,
      }
    end
  end
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
    self.socket:listen(function(message)
      local events = self._events[string.lower(message.type)]
      if events then
        vim.schedule(function()
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
---@param handler fun(context)|nil If provided, a fn called with the same params as a trigger() handler,
---and whose return value will be used as the prompt content
function KodachiState:prompt(matcher, handler)
  matcher = matchers.inflate(matcher)

  -- TODO:
  local group_id = 0
  local prompt_index = 0

  return with_socket(self, function(socket)
    if not handler then
      socket:request {
        type = "RegisterPrompt",
        connection_id = self.connection_id,
        matcher = matcher,
        group_id = group_id,
        prompt_index = prompt_index,
      }
      return
    end

    self:trigger(matcher, function(context)
      local to_render = handler(context)
      if to_render then
        socket:request {
          type = "SetPromptContent",
          connection_id = self.connection_id,
          group_id = group_id,
          prompt_index = prompt_index,
          content = to_render,
        }
      end
    end)
  end)
end

---@param matcher MatcherSpec|string
function KodachiState:trigger(matcher, handler)
  matcher = matchers.inflate(matcher)
  return with_socket(self, function(socket)
    if not self._triggers then
      self._triggers = Handlers:new()
      socket:listen(function(message)
        if message.type == 'TriggerMatched' and message.connection_id == self.connection_id then
          local triggered_handler = self._triggers:get(message.handler_id)
          if triggered_handler then
            vim.schedule(function()
              triggered_handler(message.context)
            end)
          else
            vim.schedule(function()
              print('WARNING: Trigger handler missing...')
            end)
          end
        end
      end)
    end

    local id = self._triggers:insert(handler)
    socket:request {
      type = "RegisterTrigger",
      connection_id = self.connection_id,
      matcher = matcher,
      handler_id = id,
    }
  end)
end

---Send some text to the connection associated with this state
---@param text string
function KodachiState:send(text)
  return with_socket(self, function(socket)
    socket:request {
      type = "Send",
      connection_id = self.connection_id,
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
