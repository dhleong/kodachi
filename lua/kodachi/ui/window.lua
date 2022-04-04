local states = require'kodachi.states'

local M = {}

---Configure the current window/buffer to act as a "client." This is an error if there is no
---KodachiState associated with the current buffer
function M.configure_current()
  local state = states.current()
  if not state then
    error("Attempting to configure a non-kodachi window as a client")
  end

  -- TODO Unify and simplify this
  vim.cmd [[ nnoremap <buffer> i <cmd>lua require'kodachi.ui.composer'.enter_or_create { insert = true } <cr> ]]
  vim.cmd [[ nnoremap <buffer> I <cmd>lua require'kodachi.ui.composer'.enter_or_create{ insert = true }<cr> ]]
end

return M
