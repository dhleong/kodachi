---@alias CompletionParams { word_to_complete: string, line_to_cursor: string, line: string, cursor: number }

local M = {}

---@param state KodachiState
---@param params CompletionParams
function M.suggest_completions(state, params)
  print('COMPLETING!', vim.inspect(params))
  local words = { 'magic', 'grayskull', 'swift' }
  return { words = words }
end

return M
