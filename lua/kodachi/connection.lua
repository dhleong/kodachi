local M = {}

--Start up and manage a connection to the given URI on the provided
--state object.
---@param state KodachiState
---@param uri string
function M.run(state, uri)
  state.socket:request(
    { type = 'Connect', uri = uri },
    function (response)
      state.connection_id = response.id
    end
  )

  state.socket:listen_matched_once(
    function (event)
      return event.type == 'Disconnected' and event.connection_id == state.connection_id
    end,
    function ()
      state.connection_id = nil
    end
  )
end

return M
