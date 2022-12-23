local Handlers = require 'kodachi.handlers'
local matchers = require 'kodachi.matchers'
local PromptsManager = require 'kodachi.prompts'

local util = require 'kodachi.util.state'
local with_socket = util.with_socket

---@alias KodachiEvent 'connected'|'disconnected'

---@class KodachiState
---@field bufnr number
---@field connection_id number|nil
---@field uri string|nil
---@field exited boolean|nil
---@field socket Socket
---@field _events any|nil
---@field _mappings any
---@field _prompts PromptsManager|nil
---@field _aliases Handlers|nil
---@field _triggers Handlers|nil
local KodachiState = {}

function KodachiState:new(o)
  o._mappings = o._mappings or {}
  setmetatable(o, self)
  self.__index = self
  return o
end

function KodachiState:cleanup()
  local cleared_any = false
  if self._aliases then
    self._aliases:clear()
    cleared_any = true
  end

  if self._events then
    -- NOTE: We don't need to set cleared_any because the server is not tracking
    -- any state for us for events.
    self._events = nil
  end

  if self._triggers then
    self._triggers:clear()
    cleared_any = true
  end

  if self._prompts then
    self._prompts:clear()
    cleared_any = true
  end

  if cleared_any and self.socket and self.connection_id then
    self.socket:notify {
      type = "Clear",
      connection_id = self.connection_id,
    }
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

  if not self._events[event] then
    self._events[event] = {}
  end

  table.insert(self._events[event], handler)
end

---@param matcher MatcherSpec|string
function KodachiState:alias(matcher, handler)
  matcher = matchers.inflate(matcher)
  return with_socket(self, function(socket)
    if type(handler) == "string" then
      socket:request {
        type = "RegisterAlias",
        connection_id = self.connection_id,
        matcher = matcher,
        replacement_pattern = handler,
      }
    else
      local aliases = self:_alias_handlers(socket)
      local id = aliases:insert(handler)
      socket:request {
        type = "RegisterAlias",
        connection_id = self.connection_id,
        matcher = matcher,
        handler_id = id,
      }
    end
  end)
end

---@param matcher MatcherSpec|string
---@param handler fun(context)|nil If provided, a fn called with the same params as a trigger() handler,
---and whose return value will be used as the prompt content
function KodachiState:prompt(matcher, handler)
  local prompts = self._prompts
  if not prompts then
    local new_prompts = PromptsManager:new()
    self._prompts = new_prompts
    prompts = new_prompts
  end

  local group = prompts:group(0)
  return group:add(matcher, handler)
end

---@param matcher MatcherSpec|string
function KodachiState:trigger(matcher, handler)
  matcher = matchers.inflate(matcher)
  return with_socket(self, function(socket)
    local triggers = self:_trigger_handlers(socket)
    local id = triggers:insert(handler)
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

---@param socket Socket
function KodachiState:_alias_handlers(socket)
  local aliases = self._aliases
  if not aliases then
    local new_aliases = Handlers:new()
    self._aliases = new_aliases
    socket:listen(function(message)
      if message.type == 'HandleAliasMatch' and message.connection_id == self.connection_id then
        local matched_handler = self._aliases:get(message.handler_id)
        if matched_handler then
          vim.schedule(function()
            local result = matched_handler(message.context)
            if type(result) == 'string' then
              socket:notify {
                type = 'AliasMatchHandled',
                request_id = message.id,
                handler_id = message.handler_id,
                replacement = result,
              }
            end
          end)
        else
          vim.schedule(function()
            print('WARNING: Alias handler missing...')
          end)
        end
      end
    end)
    return new_aliases
  end

  return aliases
end

---@param socket Socket
function KodachiState:_trigger_handlers(socket)
  local triggers = self._triggers
  if not triggers then
    local new_triggers = Handlers:new()
    self._triggers = new_triggers
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
    return new_triggers
  end

  return triggers
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
