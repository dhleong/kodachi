local states = require 'kodachi.states'

local MIN_HEIGHT = 2

local M = {}

local function feed_backspace()
  -- Use backspace to ensure we're starting from scratch
  local keys = vim.api.nvim_replace_termcodes('<bs>', true, false, true)
  vim.api.nvim_feedkeys(keys, 'n', true)
end

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

---@param state KodachiState
local function configure_current_as_composer(state)
  -- Enable null-ls completions
  -- NOTE: These msut be done *before* setting buftype, or else null-ls ignores!
  vim.api.nvim_buf_set_name(0, 'kodachi.composer:' .. state.connection_id)
  vim.bo.filetype = 'kodachi.composer'

  vim.bo.buftype = 'nofile'
  vim.bo.bufhidden = 'hide'
  vim.bo.swapfile = false
  vim.wo.winfixheight = true

  -- Handle submitting
  vim.cmd [[ inoremap <buffer> <cr> <cmd>lua require'kodachi.ui.composer'.submit()<cr> ]]
  vim.cmd [[ nnoremap <buffer> <cr> <cmd>lua require'kodachi.ui.composer'.submit()<cr> ]]

  -- Support inserting newlines
  vim.cmd [[ inoremap <buffer> <s-cr> <cr> ]]
  vim.cmd [[ inoremap <buffer> <a-cr> <cr> ]]

  -- Make it natural to leave
  vim.cmd [[ inoremap <buffer> <c-c> <esc>ZQ ]]
  vim.cmd [[ nnoremap <buffer> <c-c> ZQ ]]
end

local function measure_line_width(linenr)
  return vim.fn.virtcol { linenr, '$' } - 1
end

local function on_composer_buf_entered()
  vim.cmd [[
    augroup KodachiComposer
      autocmd!

      autocmd TextChanged <buffer> lua require'kodachi.ui.composer'.on_change()
      autocmd TextChangedI <buffer> lua require'kodachi.ui.composer'.on_change()

      " Hide the window on leave:
      autocmd BufLeave <buffer> lua require'kodachi.ui.composer'.hide()
    augroup KodachiComposer
  ]]

  -- Resize based on text in buffer
  M.on_change()
end

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

  if state.composer_bufnr and vim.fn.bufexists(state.composer_bufnr) ~= 0 then
    -- Reuse the existing buffer in case it had some text
    vim.api.nvim_set_current_buf(state.composer_bufnr)
  else
    -- New composer buffer
    vim.cmd [[ enew ]]
    state.composer_bufnr = vim.fn.bufnr('%')
    states[state.composer_bufnr] = state
    configure_current_as_composer(state)
  end

  on_composer_buf_entered()

  if config.insert then
    vim.cmd [[ startinsert! ]]
    feed_backspace()
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

    if vim.fn.mode() == 'i' then
      feed_backspace()
    end
  end
end

function M.compute_height()
  local win_width = vim.fn.winwidth(0)
  local height = 0
  for i = 1, vim.fn.line('$') do
    height = height + vim.fn.ceil(measure_line_width(i) / win_width)
  end
  return vim.fn.max { MIN_HEIGHT, height + 1 }
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
      -- NOTE: If leaving to a popup window, for example, we may *not* be
      -- executing in its context! So let's be more careful
      local win = vim.fn.bufwinid(state.composer_bufnr)
      if win ~= -1 then
        vim.fn.win_execute(win, [[ hide ]])
      end
    end
  end
end

function M.on_change()
  -- Ensure the current window is sized based on the height of its text
  local buf_height = M.compute_height()
  if buf_height ~= vim.fn.winheight(0) then
    vim.cmd('resize ' .. buf_height)
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

  state:send(text)
end

return M
