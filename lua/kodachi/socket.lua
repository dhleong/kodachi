---@class Socket
---@field name string
---@field _receivers any[]
---@field to_app any
local Socket = {}

function Socket:new(name, from_app, to_app)
  local o = {
    name = name,
    from_app = from_app,
    to_app = to_app,
    _next_request_id = 0,
    _to_app_queue = {},
    _receivers = {},
    _received_data = '',
  }

  setmetatable(o, self)
  self.__index = self
  return o
end

function Socket:next_request_id()
  local id = self._next_request_id
  self._next_request_id = id + 1
  return id
end

function Socket:listen(handler)
  table.insert(self._receivers, handler)
end

function Socket:unlisten(handler)
  local index = nil
  for i, candidate in ipairs(self._receivers) do
    if candidate == handler then
      index = i
      break
    end
  end

  if index then
    table.remove(self._receivers, index)
  end
end

function Socket:listen_matched_once(matcher, handler)
  local socket = self
  local function listener(message)
    if matcher(message) then
      handler(message)
      socket:unlisten(listener)
    end
  end

  socket:listen(listener)
end

function Socket:await_request_id(id, handler)
  local function matcher(message)
    return message.request_id == id
  end
  self:listen_matched_once(matcher, handler)
end

function Socket:write(data)
  if self.connected then
    self.to_app:write(data)
  else
    table.insert(self._to_app_queue, data)
  end
end

function Socket:_on_connected()
  self.connected = true
  for _, item in ipairs(self._to_app_queue) do
    self:write(item)
  end
  self._to_app_queue = {}
end

function Socket:_on_read(chunk)
  self._received_data = self._received_data .. chunk

  while true do
    local line_end, _ = string.find(self._received_data, '\n', 1, true)
    if not line_end then
      return
    else
      local to_parse = string.sub(self._received_data, 1, line_end)
      self._received_data = string.sub(self._received_data, line_end + 1)

      local parsed = vim.json.decode(to_parse)
      for _, receiver in ipairs(self._receivers) do
        receiver(parsed)
      end
    end
  end
end

local M = {}

---@param name string|nil Preferred name of the unix domain socket; if not provided
-- or nil, `tempname()` will be used
---@return Socket
function M.create(name)
  local path = name or vim.fn.tempname()
  local server = vim.loop.new_pipe(false)
  local client = vim.loop.new_pipe(false)

  local socket = Socket:new(path, server, client)

  server:bind(path)
  server:listen(16, function ()
    server:accept(client)
    socket:_on_connected()

    client:read_start(function (err, chunk)
      assert(not err, err) -- TODO: Handle errors better?
      if chunk then
        socket:_on_read(chunk)
      else
        -- EOF:
        socket.connected = false
        server:close()
        client:close()
      end
    end)
  end)

  return socket
end

return M
