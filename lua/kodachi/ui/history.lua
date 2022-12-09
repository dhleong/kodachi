local History = {
  ---@type string[]|nil
  _saved = nil,
}

function History._get_active()
  local old = {}
  local limit = vim.fn.histnr('@')
  if limit > 0 then
    for i = 1, limit do
      table.insert(old, vim.fn.histget('@', i))
    end
  end
  return old
end

function History._set_active(entries)
  vim.fn.histdel('@')

  for _, entry in ipairs(entries) do
    vim.fn.histadd('@', entry)
  end
end

function History.make_active(entries)
  History.save()
  History._set_active(entries)
end

function History.save()
  if not History._saved then
    History._saved = History._get_active()
  end
end

function History.restore()
  if History._saved then
    History._set_active(History._saved)
    History._saved = nil
  end
end

local function perform_input()
  return vim.fn.input('>')
end

local function show_history(entries)
  History.make_active(entries)

  local ok, input = pcall(perform_input)

  History.restore()

  if ok and input ~= '' then
    local state = require 'kodachi.states'.current_connected()
    if state then
      state:send(input)
    end
  end
end

local M = {}

function M._on_maybe_cmdwin_enter()
  if History._saved then
    vim.bo.filetype = 'kodachi.composer'
  end
end

function M._on_maybe_cmdwin_leave()
  if History._saved then
    vim.bo.filetype = vim.o.filetype
  end
end

function M.open()
  local bufnr = vim.fn.bufnr('%')
  local state = require 'kodachi.states'[bufnr] or require 'kodachi.ui.composer'.state_for_composer_bufnr(bufnr)
  if not state then
    print('Not connected')
    return
  end

  vim.cmd [[
    augroup KodachiHistory
      autocmd!
      autocmd CmdWinEnter @ lua require 'kodachi.ui.history'._on_maybe_cmdwin_enter()
      autocmd CmdWinLeave @ lua require 'kodachi.ui.history'._on_maybe_cmdwin_leave()
    augroup END
  ]]

  local response = state.socket:request_blocking {
    type = 'GetHistory',
    connection_id = state.connection_id,
    limit = 50,
  }

  if response.type == 'ErrorResult' then
    print('ERROR: Unable to fetch history: ', response.error)
    return
  end

  show_history(response.entries)
end

return M
