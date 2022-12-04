---@alias CompletionParams { word_to_complete: string, line_to_cursor: string, line: string, cursor: number }

local M = {}

---@param state KodachiState
---@param params CompletionParams
function M.suggest_completions(state, params, cb)
  state.socket:request(
    {
      type = 'CompleteComposer',
      connection_id = state.connection_id,
      word_to_complete = params.word_to_complete,
      line_to_cursor = params.line_to_cursor,
      line = params.line,
    },
    function(response)
      if response.type == 'ErrorResult' then
        cb(response.error)
      else
        cb(nil, response)
      end
    end
  )
end

return M
