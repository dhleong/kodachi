local states = require'kodachi.states'

local function state_composer()
  local state = states.current_connected()
  if not state then
    return
  end

  if vim.fn.bufnr('%') ~= state.composer_bufnr then
    return
  end

  return state
end

local M = {}

function M.create_or_enter()
  local state = states.current_connected()
  if not state then
    return
  end

  -- TODO: Look for an existing composer window in this tab for this connection

  -- No existing window; create one
  vim.cmd [[ belowright new ]]

  if state.composer_bufnr then
    -- Reuse the existing buffer in case it had some text
    vim.cmd(state.composer_bufnr .. 'buffer')
  else
    vim.cmd [[ enew ]]
    state.composer_bufnr = vim.fn.bufnr('%')
    states[state.composer_bufnr] = state

    vim.bo.buftype = 'nofile'
    vim.bo.bufhidden = 'hide'
    vim.bo.swapfile = false

    vim.cmd('inoremap <buffer> <cr> <cmd>lua require"kodachi.composer".submit()<cr>')
  end

  -- TODO: Resize based on text in buffer
  -- TODO: Auto-resize buffer as text is entered
  vim.cmd [[ resize 1 ]]
end

---@param opts { clear:boolean }|nil
function M.hide(opts)
  local state = state_composer()
  if state then
    vim.cmd [[ stopinsert ]]

    if opts and opts.clear then
      vim.cmd [[ bwipeout! ]]
      state.composer_bufnr = nil
    else
      vim.cmd [[ hide ]]
    end
  end
end

function M.submit()
  local state = state_composer()
  if not state then
    return
  end

  local lines = vim.fn.getline(1, '$')
  local text = table.concat(lines, '\n')

  M.hide { clear = true }

  require'kodachi'.buf_send(text)
end

return M
