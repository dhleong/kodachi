local M = {}

---Open a connection to the given URI in the current buffer
---@param uri string
function M.connect(uri)
  local state = require'kodachi.ui'.ensure_window()
  if not state then
    return
  end

  require'kodachi.connection'.run(state, uri)

  return state
end

return M
