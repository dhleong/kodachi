local function swap_active(new_list)
  local old = {}
  local limit = vim.fn.histnr('@')
  if limit > 0 then
    for i = 1, limit do
      table.insert(old, vim.fn.histget('@', i))
    end
  end

  vim.fn.histdel('@')

  for _, entry in ipairs(new_list) do
    vim.fn.histadd('@', entry)
  end

  return old
end

local function perform_input()
  return vim.fn.input('>')
end

local M = {}

function M.open()
  -- TODO: Setup autocmd to use the right filetype in the cmdlinewin
  -- TODO: Query for history
  local old_history = swap_active({ 'for', 'honor', 'grayskull' })

  local ok, input = pcall(perform_input)

  swap_active(old_history)

  if ok then
    local state = require 'kodachi.states'.current_connected()
    if state then
      state:send(input)
    end
  end
end

return M
