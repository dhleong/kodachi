---
vimdoctitle: "kodachi"
---

# Quick Start

Once installed, the quickest way to get started is with the `:KodachiConnect` command.

In general you will want to create lua script files to manage your config for a server. You may consider a naming convention to simplify auto-sourcing them, since kodachi supports hot-reloading configs for an active connection; I use `.kd.lua`.

Here's a sample script:

```lua
local kodachi = require 'kodachi'

local uri = 'myfavorite.game:1234'

kodachi.with_connection(uri, function(s)
  -- `s` is the "State" object, and has some goodies, like local mappings:
  s:map('gl', 'look')

  -- ... Triggers
  s:trigger('Hello!', function()
    s:send('say Hello yourself!')
  end

  -- ... and more
  s:prompt('> ')
end)
```

# Aliases, Triggers, and Prompts

Kodachi provides rich support for aliases, triggers, and prompts. Aliases can format the output directly using matchers, or the output can be computed by lua function.

Matchers power aliases, prompts, and triggers. Kodachi supports two variants: simple and regex. [Simple matchers](#simple-matchers) should be intuitive, with familiar syntax, while [regex](#regex-matchers) gives you the full power to match exactly what you want. Regex matchers are powered by the Rust [regex][regex] crate; in particular, be aware that this crate does not support zero-width lookaround assertions.

## Simple Matchers

Simple matchers use `$`-prefixed symbols to capture some input; aliases may also use the same syntax to reference those matches. For example:

```lua
-- This will capture (for example) "grill a burrito" and expand to "put a burrito on grill"
s:alias('grill $food', 'put $food on grill')
```

Aliases typically can be expanded into other aliases. To reduce ambiguity, you may want your alias only to match at the beginning of the line. This can be done similar to regex with the `^` symbol:

```lua
-- This will NOT match "say grill sandwich" (which *would* be matched above)
s:alias('^grill $food', 'put $food on grill')
```

Also supported is indexed symbols (eg: `$1`, `$2`, etc.) and disambiguated symbols, wrapping a name with curly braces (eg: `${food}`), which may be useful if you need to capture text that immediately preceeds other text.

## Regex Matchers

```lua
local m = require 'kodachi.matchers'

s:alias(m.regex '^grill ([a-z]+)', 'put $1 on grill')
```

## Functional match handlers

This syntax works for both Aliases and Triggers. The "context" of the match is provided as the first argument to the function. For example:

```lua
s:alias('^grill $food', function (context)
  return 'put ' .. context.named.food.plain .. ' on grill'
end)
```

Note that each "match" is an object, containing both the `plain` output (stripped of color symbols) and the `ansi` output (exactly what the server sent, including color symbols).

The `context` object also includes `indexed` symbols in eg `context.indexed[1]`.

If you don't return anything from an Alias function, nothing will be sent. If you want to handle sending yourself for whatever reason, you may use the [state:send](#state-send) method.

# Scripting

Most users will want to configure their connections using the provided Lua scripting API.

## with_connection

```lua
require 'kodachi'.with_connection(URI, handler)
```

Your primary entrypoint, `with_connection` accepts a URI (eg: `"yourmud.com:1234"`) and a handler function. The handler function is provided a [State Object](#state-object) object on connected and also if the script is sourced while connected.

## State Object

The `KodachiState` object is provided to you from the [with_connection](#with_connection) function. It is your primary means of configuring the connection, and houses all of the following methods for doing so.

#### state:alias

Create an alias for the connection. Aliases allow you to reduce repeated work by automatically expanding simple phrases into more complex ones.

```lua
s:alias(matcher, handler)
```

For most purposes, you can combine a [simple matcher](#simple-matchers) with a simple handler, like so:

```lua
s:alias('^grill $food', 'put $food on grill')
```

#### state:map

Create a normal-mode mapping. Similar to creating an nmap in vim, using this method will cause key sequences entered in normal mode in the connection buffer to trigger actions.

```lua
s:map(keys, handler)
```

If a string is provided as the handler, that string will be sent literally. More commonly, you may provide a function to be executed; that function will be provided with the state object for you to then call [state:send](#state:send) with whatever you want to send.

#### state:command

Create a command that can be executed in the connection buffer.

```lua
s:command(name, handler, *opts)
```

The command `name` must begin with a capital letter. `handler` behaves like `map`, but receives the same argument as passed to the neovim command handler (See [nvim_create_user_command]). `opts` similarly will be passed to [nvim_create_user_command].


#### state:on

Register an event handler. Most commonly, you will probably want to use these to listen to "events." To do so, pass a `{ns, name}` table as the event parameter.

```lua
s:on(event, handler)
```

For example, to listen to the `ROOM` var received over `MSDP`, use:

```lua
s:on({"MSDP", "ROOM"}, function (room)
    -- Do something with the room object
end)
```

#### state:send

Send the given String to the server.

```lua
s:send(String)
```

#### state:prompt

Register a prompt. `handler` is optional, and may be used to transform the matched line before rendering.

```lua
s:prompt(matcher, handler)
```

#### state:trigger

Register a trigger. Triggers "fire" when the `matcher` matches on a line received from the server.

```lua
s:trigger(matcher, handler)
```

The handler of a trigger *must* be a function.

[regex]: https://docs.rs/regex/latest/regex/
