local states = require'kodachi.states'

---@alias KodachiRequest { type: string }

local M = {}

---@param uri string
function M.buf_connect(uri)
  local state = require'kodachi.ui'.ensure_window()

  M.buf_request(
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

---@param request KodachiRequest
function M.buf_request(request, cb)
  local state = states.current()
  if not state then
    return
  end

  state.socket:request(request, cb)
end

function M.buf_send(text)
  local state = states.current_connected()
  if not state then
    return
  end

  M.buf_request { type = "Send", connection = state.connection_id, text = text }
end

return M
