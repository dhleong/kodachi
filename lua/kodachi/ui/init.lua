local states = require'kodachi.states'

local M = {}

---Ensures that the current window is a valid kodachi output window
---@return KodachiState associated with the buffer
function M.ensure_window()
  local initial_bufnr = vim.fn.bufnr('%')
  local existing = states.current { silent = true }
  if existing then
    if not existing.exited and existing.connection_id then
      print('kodachi: A connection is already live in this buffer')
      return
    end

    -- Reuse the window with a new buffer
    vim.cmd [[ enew ]]
  elseif vim.bo.modified or vim.fn.bufname('%') ~= '' or initial_bufnr == existing.initial_bufnr then
    -- No existing state in this buffer, and the buffer is modified or associated
    -- with a file on disk; go ahead and open a split
    vim.cmd [[ vsplit | enew ]]
  end

  local socket = require'kodachi.socket'.create()
  local state = states.create_for_buf { socket = socket }

  -- Share the state with the source buffer, I guess? This facilitates reloading the
  -- script for a live connection to update mappings, etc.
  states[initial_bufnr] = state
  state.initial_bufnr = initial_bufnr

  local job_id = require'kodachi.ui.term'.spawn_unix {
    socket_name = socket.name,
    on_exit = function ()
      state.exited = true
      state.connection_id = nil

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
