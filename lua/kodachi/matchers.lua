---@alias MatcherSpec { type:'regex', source:string }

local M = {}

---@param matcher MatcherSpec|string
function M.inflate(matcher)
  local matcher_type = type(matcher)
  if matcher_type == 'table' then
    return matcher
  elseif matcher_type == 'string' then
    return M.simple(matcher)
  else
    error("Invalid matcher type: " .. matcher_type)
  end
end

---Create a matcher using the regex syntax of the Rust lang `regex` library
---@param pattern string A perl-like regex pattern.
---@return MatcherSpec
function M.regex(pattern, options)
  return {
    type = 'Regex',
    source = pattern,
    consume = options and options.consume,
  }
end

function M.re_consume(pattern)
  return M.regex(pattern, { consume = true })
end

---Create a matcher using "simple" syntax
---@param pattern string A "simple" matcher pattern
---@return MatcherSpec
function M.simple(pattern)
  return {
    type = 'Simple',
    source = pattern,
  }
end

return M
