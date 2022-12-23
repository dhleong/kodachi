---@param event_spec table
local function event_data_matches(event_spec, message)
  if event_spec[1] ~= message.ns then
    return false
  end

  if #event_spec == 2 and event_spec[2] ~= message.name then
    return false
  end

  return true
end

local M = {}

---@return string, fun(any)
function M.wrap(event, handler)
  if type(event) == "table" then
    local function wrapped(message)
      if event_data_matches(event, message) then
        handler(message.payload, message)
      end
    end

    return 'event', wrapped
  else
    return event, handler
  end
end

return M
