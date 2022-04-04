local states = require'kodachi.states'

local local_path = debug.getinfo(1, 'S').source:sub(2)
local kodachi_root = vim.fn.fnamemodify(local_path, ':h:h:h')
local kodachi_tmux = kodachi_root .. '/config/tmux.conf'
local kodachi_exe = kodachi_root .. '/target/release/kodachi'

---@alias KodachiRequest { type: string }

local M = {
  debug = true,
  sockets = {},
}

---@param uri string
function M.buf_connect(uri)
  local existing = states.current { silent = true }
  if existing then
    if not existing.exited then
      -- TODO: Acutally, the *daemon* is live, but we could be disconnected
      print('kodachi: A connection is already live in this buffer')
      return
    end

    -- Reuse the window
    vim.cmd [[ enew ]]
  end

  -- NOTE: Neovim does not correctly persist output if the window resizes smaller
  -- than the width of the text, so we use tmux to save it
  local tmux_wrap = vim.fn.has('nvim')

  local socket = require'kodachi.socket'.create()
  local state = states.create_for_buf { socket = socket }

  local cmd = vim.tbl_flatten {
    tmux_wrap and { 'tmux', '-f', kodachi_tmux, 'new-session', '-n', 'kodachi' } or {},
    M.debug and { 'cargo', 'run', '--' } or kodachi_exe,
    'unix', socket.name,
  }

  local job_id = vim.fn.termopen(cmd, {
    cwd = kodachi_root,
    on_exit = function (_, _, _)
      state.exited = true

      -- Clean up after ourselves
      vim.fn.delete(socket.name)
    end,
  })

  state.job_id = job_id
  state.bufnr = vim.fn.bufnr('%')

  require'kodachi.ui.window'.configure_current()

  local request_id = M.buf_request { type = 'Connect', uri = uri }
  socket:await_request_id(request_id, function (response)
    state.connection_id = response.id
  end)

  return job_id
end

---@param request KodachiRequest
function M.buf_request(request)
  local state = states.current()
  if not state then
    return
  end

  request.id = state.socket:next_request_id()

  local to_write = vim.fn.json_encode(request) .. '\n'

  state.socket:write(to_write)

  return request.id
end

function M.buf_send(text)
  local state = states.current_connected()
  if not state then
    return
  end

  M.buf_request { type = "Send", connection = state.connection_id, text = text }
end

return M
