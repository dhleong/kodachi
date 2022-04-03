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
  if vim.b.kodachi then
    if not vim.b.kodachi.exited then
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
  M.sockets[socket.name] = socket

  local state = { socket = socket.name }
  local cmd = vim.tbl_flatten {
    tmux_wrap and { 'tmux', '-f', kodachi_tmux, 'new-session', '-n', 'kodachi' } or {},
    M.debug and { 'cargo', 'run', '--' } or kodachi_exe,
    'unix', socket.name,
  }

  local job_id = vim.fn.termopen(cmd, {
    cwd = kodachi_root,
    on_exit = function (_, _, _)
      state.exited = true
      vim.b[state.bufnr].kodachi = state

      -- Clean up after ourselves
      vim.fn.delete(socket.name)
    end,
  })

  state.job_id = job_id
  state.bufnr = vim.fn.bufnr('%')
  vim.b.kodachi = state

  M.buf_request { type = 'Connect', uri = uri }

  return job_id
end

---@param request KodachiRequest
function M.buf_request(request)
  if not vim.b.kodachi then
    return
  end

  request.id = 1 -- TODO
  local to_write = vim.fn.json_encode(request) .. '\n'

  local socket = M.sockets[vim.b.kodachi.socket]
  socket:write(to_write)
  -- vim.fn.chansend(vim.b.kodachi.job_id, to_write)
end

return M
