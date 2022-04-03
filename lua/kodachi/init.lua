local local_path = debug.getinfo(1, 'S').source:sub(2)
local kodachi_root = vim.fn.fnamemodify(local_path, ':h:h:h')
local kodachi_exe = kodachi_root .. '/target/release/kodachi'

---@alias KodachiRequest { type: string }

local M = {}

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

  local state = {}
  -- local cmd = { 'tmux', 'new-session', '-n', 'kodachi', kodachi_exe }
  local cmd = { kodachi_exe }
  local job_id = vim.fn.termopen(cmd, {
    on_stderr = function (_, data)
      print(data)
    end,
    on_exit = function (_, _, _)
      state.exited = true
      vim.b[state.bufnr].kodachi = state
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
  vim.fn.chansend(vim.b.kodachi.job_id, vim.fn.json_encode(request) .. '\n')
end

return M
