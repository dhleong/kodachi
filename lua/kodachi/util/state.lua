local M = {}

---@param state KodachiState
---@param handler fun(socket:Socket)
function M.with_socket(state, handler)
  if not state.connection_id then
    return false
  end

  return handler(state.socket)
end

return M
