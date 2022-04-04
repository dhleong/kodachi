local states = require'kodachi.states'

---Fetch the KodachiState instance associated with the current buffer *if* the current buffer is a
--composer. Otherwise, returns nil
---@return KodachiState|nil
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

local function configure_current_as_composer()
  vim.bo.buftype = 'nofile'
  vim.bo.bufhidden = 'hide'
  vim.bo.swapfile = false

  -- Handle submitting
  vim.cmd [[ inoremap <buffer> <cr> <cmd>lua require'kodachi.ui.composer'.submit()<cr> ]]
  vim.cmd [[ nnoremap <buffer> <cr> <cmd>lua require'kodachi.ui.composer'.submit()<cr> ]]

  -- Support inserting newlines
  vim.cmd [[ inoremap <buffer> <s-cr> <cr> ]]
  vim.cmd [[ inoremap <buffer> <a-cr> <cr> ]]

  -- TODO: Auto-resize buffer as text is entered

  -- Hide the window on leave
  -- FIXME: This is not correct; with this method we will need to reconfigure this autogroup every
  -- time we enter a different kodachi composer. We *probably* want to set a filetype and have a
  -- global augroup for that filetype
  vim.cmd [[
    augroup KodachiComposers
      autocmd!
      autocmd BufLeave <buffer> hide
    augroup KodachiComposers
  ]]
end

local M = {}

---Jump to the composer window, if any is available in the current tabpage for the KodachiState
--associated with the current buffer, else create a new composer and enter that. This function is a
--nop if executed *in* a composer buffer
---@param opts { insert:boolean }|nil
function M.enter_or_create(opts)
  local state = states.current_connected()
  if not state or vim.fn.bufnr('%') == state.composer_bufnr then
    return
  end

  local config = opts or { insert = false }

  -- TODO: Look for an existing composer window in this tab for this connection

  -- No existing window; create one
  vim.cmd [[ belowright new ]]

  if state.composer_bufnr then
    -- Reuse the existing buffer in case it had some text
    vim.cmd(state.composer_bufnr .. 'buffer')
  else
    -- New composer buffer
    vim.cmd [[ enew ]]
    state.composer_bufnr = vim.fn.bufnr('%')
    states[state.composer_bufnr] = state

    configure_current_as_composer()
  end

  -- TODO: Resize based on text in buffer
  vim.cmd [[ resize 1 ]]

  if config.insert then
    vim.cmd [[ startinsert ]]
  end
end

---Clear the composer (if in one) without closing it
function M.clear()
  if state_composer() then
    -- Clear the buffer and also its undo history
    vim.cmd [[
      let old_undolevels = &undolevels
      setlocal undolevels=-1
      norm! ggdG
      let &undolevels = old_undolevels
      unlet old_undolevels
    ]]
  end
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

  M.clear()

  require'kodachi'.buf_send(text)
end

return M
