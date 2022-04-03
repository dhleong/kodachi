local Socket = {}

function Socket:new(name, from_app, to_app)
  local o = { name = name, from_app = from_app, to_app = to_app}
  setmetatable(o, self)
  self.__index = self
  return o
end

function Socket:write(data)
  self.to_app:write(data)
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
    from_app:accept(to_app)
    print('Received connection...')
  end)

  return socket
end

return M
