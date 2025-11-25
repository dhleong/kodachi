local local_path = debug.getinfo(1, 'S').source:sub(2)
local kodachi_root = vim.fn.fnamemodify(local_path, ':h:h:h:h')
local kodachi_tmux = kodachi_root .. '/config/tmux.conf'
local kodachi_exe = kodachi_root .. '/target/release/kodachi'

local M = {
  debug = true,
}

---@param opts {socket_name:string, on_exit:any}
function M.spawn_unix(opts)
  -- NOTE: Neovim does not correctly persist output if the window resizes smaller
  -- than the width of the text, so we use tmux to save it (if available)
  local tmux_wrap = vim.fn.has('nvim') and vim.fn.executable('tmux')

  -- TODO If not debug, ensure the release executable is compiled/up-to-date

  local session_name = vim.fn.rand()

  local env = {
    KODACHI_DUMP = vim.g.KODACHI_DUMP or '',
    DEBUG = vim.g.KODACHI_DEBUG or '',
    RUST_BACKTRACE = vim.g.KODACHI_DEBUG and '1' or '',
  }

  local cmd = vim.tbl_flatten {
    tmux_wrap and {
      'tmux',
      '-L', 'kodachi-tmux', -- Use a kodachi-specific server to avoid option pollution
      '-f', kodachi_tmux,
      'new-session',
      '-n', 'kodachi',
      '-s', session_name,
      '-e', 'DEBUG=' .. env.DEBUG,
      '-e', 'KODACHI_DUMP=' .. env.KODACHI_DUMP,
      '-e', 'RUST_BACKTRACE=' .. env.RUST_BACKTRACE,
    } or {},
    M.debug and { 'cargo', 'run', '--' } or kodachi_exe,
    'unix', opts.socket_name,
  }

  local session_path = vim.env.HOME .. '/.config/kodachi/.sessions/'
  local output_file = session_path .. session_name
  local state = { bufnr = nil }

  -- Ensure the sessions dir exists
  vim.fn.mkdir(session_path, "p")

  local job_id = vim.fn.termopen(cmd, {
    cwd = kodachi_root,
    env = env,
    on_exit = function(_, _, _)
      opts.on_exit()

      -- Smol bit of hacks to "preserve" the window on exit.
      -- We do this *after* invoking the on_exit callback to ensure the original
      -- bufnr is still available to listeners
      if tmux_wrap and state.bufnr then
        local win = vim.fn.bufwinid(state.bufnr)
        if win ~= -1 then
          local uri = vim.uri_from_fname(output_file)
          local bufnr = vim.uri_to_bufnr(uri)
          vim.api.nvim_win_set_buf(win, bufnr)

          vim.api.nvim_win_call(win, function()
            vim.fn.termopen({ "cat", output_file }, {
              on_exit = function(_, _, _)
                vim.fn.delete(output_file)
              end
            })
          end)
        end
      end
    end,
  })

  state.bufnr = vim.fn.bufnr('%')

  return job_id
end

return M
