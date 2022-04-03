local Socket = {}

function Socket:new(name, from_app, to_app)
  local o = {
    name = name,
    from_app = from_app,
    to_app = to_app,
    to_app_queue = {},
  }

  setmetatable(o, self)
  self.__index = self
  return o
end

function Socket:write(data)
  if self.connected then
    self.to_app:write(data)
  else
    table.insert(self.to_app_queue, data)
  end
end

function Socket:_on_connected()
  self.connected = true
  for _, item in ipairs(self.to_app_queue) do
    self:write(item)
  end
  self.to_app_queue = {}
end

local M = {}

---@param name string|nil Preferred name of the unix domain socket; if not provided
-- or nil, `tempname()` will be used
function M.create(name)
  local path = name or vim.fn.tempname()
  local from_app = vim.loop.new_pipe(false)
  local to_app = vim.loop.new_pipe(false)

  local socket = Socket:new(path, from_app, to_app)

  from_app:bind(path)
  from_app:listen(16, function ()
    print('Received connection...')
    from_app:accept(to_app)
    socket:_on_connected()
  end)

  return socket
end

return M
