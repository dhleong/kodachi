local h = require 'null-ls.helpers'
local methods = require 'null-ls.methods'

local composer = require 'kodachi.ui.composer'

local COMPLETION = methods.internal.COMPLETION

return h.make_builtin({
  name = 'kodachi.composer',
  meta = {
    description = 'Completions for kodachi composer',
  },
  method = COMPLETION,
  filetypes = {
    'kodachi.composer',
  },
  generator = {
    fn = function(params, done)
      local get_candidates = function(entries)
        local items = {}
        for k, v in ipairs(entries) do
          items[k] = {
            label = v,
            kind = vim.lsp.protocol.CompletionItemKind.Text,
          }
        end

        return items
      end

      local state = composer.state_for_composer_bufnr(params.bufnr)
      if not (state and state.connection_id) then
        return done {
          {
            items = {},
            isIncomplete = false,
          },
        }
      end

      local line = params.content[params.row]
      local results = require 'kodachi.completion'.suggest_completions(state, {
        word_to_complete = params.word_to_complete,
        line = line,
        line_to_cursor = line:sub(1, params.col)
      })

      local candidates = results and get_candidates(results.words) or {}
      done {
        {
          items = candidates,
          isIncomplete = #candidates > 0
        }
      }
    end,
    async = true,
  },
})
