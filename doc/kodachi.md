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

Kodachi provides rich support for aliases, triggers, and prompts. Aliases can format the the output directly using [matchers](#matchers), or the output can be computed by lua function.

## Matchers

Matchers power alias, prompts, and triggers. Kodachi supports two variants: simple and regex. Simple matchers should be intuitive, with familiar syntax, while regex gives you the full power to match exactly what you want. Regex matchers are powered by the Rust [regex][regex] crate; in particular, be aware that this crate does not support zero-width lookaround assertions.

### Simple Matchers

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

### Functional matches

This syntax works for both Aliases and Triggers. The "context" of the match is provided as the first argument to the function. For example:

```lua
s:alias('^grill $food', function (context)
  return 'put ' .. context.named.food.plain .. ' on grill'
end)
```

Note that each "match" is an object, containing both the `plain` output (stripped of color symbols) and the `ansi` output (exactly what the server sent, including color symbols).

The `context` object also includes `indexed` symbols in eg `context.indexed[1]`.

If you don't return anything from an Alias function, nothing will be sent. If you want to handle sending yourself for whatever reason, you may use the `s:send()` function.

# Scripting

Most users will want to configure their connections using the provided Lua scripting API.

## with_connection

```lua
require 'kodachi'.with_connection(URI, handler)
```

Your primary entrypoint, `with_connection` accepts a URI (eg: `"yourmud.com:1234"`) and a handler function. The handler function is provided a [KodachiState](#kodachistate) object on connected and also if the script is sourced while connected.

## KodachiState

The `KodachiState` object is provided to you from the [with_connection](#with_connection)

#### alias{(matcher, handler)}

TK

#### map{(keys, handler)}

TK

#### on{(event, handler)}

TK

#### send{(String)}

Send the given String to the server

#### prompt{(matcher, handler)}

TK

#### trigger{(matcher, handler)}

TK

[regex]: https://docs.rs/regex/latest/regex/
