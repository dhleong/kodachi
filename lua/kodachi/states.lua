local KodachiState = require 'kodachi.state'

local M = {
  states = {},
}

---@param initial_state KodachiState
---@return KodachiState
function M.create_for_buf(initial_state)
  local state = KodachiState:new(initial_state or {})
  state.bufnr = vim.fn.bufnr('%')
  rawset(M.states, state.bufnr, state)
  return state
end

---@param opts { silent:boolean }|nil
function M.current(opts)
  local state = M[vim.fn.bufnr('%')]

  if not state and (not opts or not opts.silent) then
    print('Not connected to a kodachi session')
  end

  return state
end

function M.current_connected()
  local state = M.current()

  if not state or not state.connection_id then
    print('Not connected.')
    return
  end

  return state
end

setmetatable(M, {
  __index = function(_, bufnr)
    return M.states[bufnr]
  end,

  __newindex = function(_, bufnr, state)
    if M.states[bufnr] or M.states[state.bufnr] then
      M.states[bufnr] = state
    end
  end,
})

return M
