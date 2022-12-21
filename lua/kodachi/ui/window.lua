local states = require 'kodachi.states'

local M = {}

---Configure the current window/buffer to act as a "client." This is an error if there is no
---KodachiState associated with the current buffer
function M.configure_current()
  local state = states.current()
  if not state then
    error("Attempting to configure a non-kodachi window as a client")
  end

  -- Ensure closing the window(s) doesn't kill the client accidentally
  vim.bo.bufhidden = 'hide'

  -- TODO Unify and simplify this
  vim.cmd [[ nnoremap <buffer> i <cmd>lua require'kodachi.ui.composer'.enter_or_create { insert = true } <cr> ]]
  vim.cmd [[ nnoremap <buffer> I <cmd>lua require'kodachi.ui.composer'.enter_or_create { insert = true }<cr> ]]

  vim.cmd [[ nnoremap <Plug>KodachiPrompt <cmd>lua require'kodachi.ui.history'.open()<cr>]]
  vim.cmd([[ nnoremap <buffer> qi <Plug>KodachiPrompt]] .. vim.o.cedit)

  -- Ensure the cursor is at the bottom of the window so it scrolls with output initially
  vim.cmd [[ normal! G ]]
end

return M
