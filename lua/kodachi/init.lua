local M = {}

---Open a connection to the given URI in the current buffer
---@param uri string
function M.connect(uri)
  local state = require'kodachi.ui'.ensure_window()
  if not state then
    return
  end

  state.socket:request(
    { type = 'Connect', uri = uri },
    function (response)
      state.connection_id = response.id
    end
  )

  state.socket:listen_matched_once(
    function (event)
      return event.type == 'Disconnected' and event.connection_id == state.connection_id
    end,
    function ()
      state.connection_id = nil
    end
  )

  return state
end

return M
