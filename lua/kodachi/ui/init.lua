local states = require 'kodachi.states'

---@return number The created window id
local function create_window()
  vim.cmd [[ vsplit | enew ]]
  return vim.fn.win_getid()
end

local function reuse_or_create_window()
  local initial_bufnr = vim.fn.bufnr('%')

  local existing = states.current { silent = true }
  if existing then
    local connected = not existing.exited and existing.connection_id
    if connected and vim.fn.bufwinnr(existing.bufnr) ~= -1 then
      print('kodachi: A connection is already live in this buffer')
      return
    elseif not existing.exited and initial_bufnr == existing.initial_bufnr and
        vim.fn.winheight(existing.initial_winid) ~= -1 then
      -- Reuse an existing (disconnected) socket
      vim.api.nvim_set_current_win(existing.initial_winid)
    elseif not connected and initial_bufnr == existing.initial_bufnr and vim.fn.winheight(existing.initial_winid) ~= -1 then
      -- Reuse an existing window
      vim.api.nvim_set_current_win(existing.initial_winid)
      vim.cmd [[ enew ]]
    elseif initial_bufnr == existing.initial_bufnr then
      -- We're in the "initial" buffer for this connection, and either the
      -- connection has closed or we've hidden the buffer.
      -- Create a new window instead of overwriting this one
      local new_winid = create_window()

      if connected then
        -- Still connected; restore the buffer!
        vim.api.nvim_win_set_buf(new_winid, existing.bufnr)
        return
      end
    else
      -- Reuse the window with a new buffer
      vim.cmd [[ enew ]]
    end
  elseif vim.bo.modified or vim.fn.bufname('%') ~= '' then
    -- No existing state in this buffer, and the buffer is modified or associated
    -- with a file on disk; go ahead and open a split
    create_window()
  end

  return initial_bufnr
end

local M = {}

---Ensures that the current window is a valid kodachi output window
---@return KodachiState|nil State associated with the buffer, or nil if a connection
--- is already live in the current window.
function M.ensure_window()
  local initial_bufnr = reuse_or_create_window()
  if not initial_bufnr then
    return
  end

  -- Reuse an existing Socket, if appropriate; else create a new one
  local existing = states.current { silent = true }
  local socket = existing.socket
  if not socket or not existing or existing.exited then
    socket = require 'kodachi.socket'.create()
  end

  local state = states.create_for_buf { socket = socket }

  -- Share the state with the source buffer, I guess? This facilitates reloading the
  -- script for a live connection to update mappings, etc.
  states[initial_bufnr] = state
  state.initial_bufnr = initial_bufnr
  state.initial_winid = vim.fn.win_getid()

  if existing and not existing.exited then
    -- Reuse the existing job
    state.job_id = existing.job_id
    state.bufnr = existing.bufnr
    return state
  end

  local job_id = require 'kodachi.ui.term'.spawn_unix {
    socket_name = socket.name,
    on_exit = function()
      if vim.api.nvim_exec_autocmds then
        vim.api.nvim_exec_autocmds('User', {
          pattern = 'KodachiDisconnect',
          modeline = false,
          data = {
            bufnr = state.bufnr,
            connection_id = state.connection_id,
          },
        })
      end

      state.exited = true
      state.connection_id = nil

      -- Clean up after ourselves
      vim.fn.delete(socket.name)
    end
  }

  state.job_id = job_id
  state.bufnr = vim.fn.bufnr('%')

  require 'kodachi.ui.window'.configure_current()

  return state
end

return M
