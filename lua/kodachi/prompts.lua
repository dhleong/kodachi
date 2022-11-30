local Handlers = require 'kodachi.handlers'
local matchers = require 'kodachi.matchers'

local util = require 'kodachi.util.state'
local with_socket = util.with_socket

---@class PromptGroup
---@field group_id number
---@field state KodachiState
---@field _prompts Handlers
local PromptGroup = {}

function PromptGroup:new(id, state)
  local o = {
    group_id = id,
    state = state,
    _prompts = Handlers:new()
  }
  setmetatable(o, self)
  self.__index = self
  return o
end

---@param matcher MatcherSpec|string
---@param handler fun(context)|nil If provided, a fn called with the same params as a trigger() handler,
---and whose return value will be used as the prompt content
function PromptGroup:add(matcher, handler)
  matcher = matchers.inflate(matcher)

  local prompt_index = self._prompts:allocate_id()

  return with_socket(self.state, function(socket)
    if not handler then
      socket:request {
        type = "RegisterPrompt",
        connection_id = self.state.connection_id,
        matcher = matcher,
        group_id = self.group_id,
        prompt_index = prompt_index,
      }
      return
    end

    self.state:trigger(matcher, function(context)
      local to_render = handler(context)
      if to_render then
        socket:request {
          type = "SetPromptContent",
          connection_id = self.state.connection_id,
          group_id = self.group_id,
          prompt_index = prompt_index,
          content = to_render,
        }
      end
    end)
  end)
end

---@class PromptsManager
---@field _groups Handlers
---@field state KodachiState
local PromptsManager = {}

function PromptsManager:new(state)
  local o = {
    state = state,
    _groups = Handlers:new()
  }
  setmetatable(o, self)
  self.__index = self
  return o
end

function PromptsManager:clear()
  self._groups:clear()
end

function PromptsManager:create_group()
  local group_id = self._groups:allocate_id()
  return self:group(group_id)
end

---@return PromptGroup
function PromptsManager:group(id)
  local existing = self._groups:get(id)
  if existing then
    return existing
  end

  local group = PromptGroup:new(id, self.state)
  self._groups:put(id, group)
  return group
end

return PromptsManager
