local states = require'kodachi.states'

local M = {}

---Ensures that the current window is a valid kodachi output window
---@return KodachiState associated with the buffer
function M.ensure_window()
  local existing = states.current { silent = true }
  if existing then
    if not existing.exited and existing.connection_id then
      print('kodachi: A connection is already live in this buffer')
      return
    end

    -- Reuse the window
    vim.cmd [[ enew ]]
  end

  local socket = require'kodachi.socket'.create()
  local state = states.create_for_buf { socket = socket }

  local job_id = require'kodachi.ui.term'.spawn_unix {
    socket_name = socket.name,
    on_exit = function ()
      state.exited = true

      -- Clean up after ourselves
      vim.fn.delete(socket.name)
    end
  }

  state.job_id = job_id
  state.bufnr = vim.fn.bufnr('%')

  require'kodachi.ui.window'.configure_current()

  return state
end

return M
