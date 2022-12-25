# kodachi

# Quick Start

Once installed, the quickest way to get started is with the `:KodachiConnect` command.

## Scripting

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

# Matchers

Matchers power alias, prompts, and triggers. Kodachi supports two variants: simple and regex. Simple matchers should be intuitive, with familiar syntax, while regex gives you the full power to match exactly what you want. Regex matchers are powered by the Rust [regex][regex] crate; in particular, be aware that this crate does not support zero-width lookaround assertions.

[regex]: https://docs.rs/regex/latest/regex/
