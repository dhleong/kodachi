local M = {}

---Open a connection to the given URI in the current buffer
---@param uri string
function M.connect(uri)
  local state = require 'kodachi.ui'.ensure_window()
  if not state then
    return
  end

  state.uri = uri
  require 'kodachi.connection'.run(state, uri)

  return state
end

---Ensure that a connection exists in the current tabpage for the given URI,
-- calling the given callback when it's ready (and again if the script is
-- sourced while the connection is active).
-- If called when a connection exists for another URI, this function is a nop
-- (and the callback will not be called).
function M.with_connection(uri, on_connection)
  local state = require 'kodachi.states'.current { silent = true }

  if state and state.connection_id and state.uri and state.uri ~= uri then
    -- Already connected, but to another URI
    return
  elseif not (state and state.connection_id) then
    -- Not connected yet
    state = M.connect(uri)
    if state then
      state.socket:listen_matched_once(
        function(message)
          return message.type == 'Connected'
        end,
        vim.schedule_wrap(function()
          state.just_connected = true
          on_connection(state)
          state.just_connected = nil
        end)
      )
    end
  else
    -- Already connected to this URI; first, cleanup state
    state:cleanup()

    -- Next, ensure we have an opened window
    require 'kodachi.ui'.ensure_window()

    -- Finally, trigger the callback
    on_connection(state)
  end
end

return M
