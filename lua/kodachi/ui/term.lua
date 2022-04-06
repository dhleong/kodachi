local local_path = debug.getinfo(1, 'S').source:sub(2)
local kodachi_root = vim.fn.fnamemodify(local_path, ':h:h:h')
local kodachi_tmux = kodachi_root .. '/config/tmux.conf'
local kodachi_exe = kodachi_root .. '/target/release/kodachi'

local M = {}

---@param opts {socket_name:string, on_exit:any}
function M.spawn_unix(opts)
  local socket_name = opts.socket_name

  -- NOTE: Neovim does not correctly persist output if the window resizes smaller
  -- than the width of the text, so we use tmux to save it
  local tmux_wrap = vim.fn.has('nvim')

  local cmd = vim.tbl_flatten {
    tmux_wrap and { 'tmux', '-f', kodachi_tmux, 'new-session', '-n', 'kodachi' } or {},
    M.debug and { 'cargo', 'run', '--' } or kodachi_exe,
    'unix', socket_name,
  }

  local job_id = vim.fn.termopen(cmd, {
    cwd = kodachi_root,
    on_exit = function (_, _, _)
      opts.on_exit()
    end,
  })

  return job_id
end

return M
