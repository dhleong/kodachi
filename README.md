kodachi
=======

*A tiny but powerful [MUD][mud] client that lives in your favorite editor*

## What?

kodachi is a terminal-based [MUD][mud] client designed to be embedded into your
preferred text editor, and includes a reference implementation for
[Neovim][nvim]. This is not merely a marriage of convenience, but is one of love!
Your entire time in a MUD is spent typing text, so why *shouldn't* you be able
to use your favorite way of doing so to play?

I've taken a [couple][judo] [stabs][iaido] at creating terminal-based modal MUD
clients from scratch in the past, but found I spent most of my time
reimplementing window layouts and different niche text editing commands; that's
fun in it's own way, but also felt like an exercise in futility. kodachi is a
new approach where the UI is handled by your already-capable editor, and the
actual capabilities are controlled from that editor with an RPC interface, so
it can be reused with whatever editors support embedding a terminal window.

### Features

- [x] Triggers
- [x] Prompts
- [x] Aliases
- [x] Intelligent auto-completion
- [x] Input history management
- [x] Common MUD protocols: [MTTS][mtts], [MCCP2][mccp2], [MSDP][msdp], [NAWS][naws], [EOR][eor]
- [x] Secure connections over TLS


## How?

The main functionality is implemented in [Rust][rust] for speed and portability. Clients
interact with this process using a JSON-based RPC protocol.

### Neovim

The easiest way to get started is to just install this as a plugin. I like [Plug][plug]:

```vim
Plug 'dhleong/kodachi'
```

You will also need to [set up Rust](https://www.rust-lang.org/learn/get-started) to build
that process; we don't currently provide pre-built binaries.

Once installed, you can use `:help kodachi` to learn more (also available online [here][help-kodachi]) or follow the quick start guide below.

#### Quick Start

From there, we provide a lua API for connecting and configuring:

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

The first time you source this script (eg: `:source %`) it will connect and open a split
window with the output. Any subsequent source while connected will update your config,
replacing triggers and mappings without disconnecting.

To open a "composer" window for sending something to the server, simply hit `i` within this
output window, as you would normally.

#### Autocomplete

Auto-complete support in Neovim is implemented as a [null-ls][null-ls] source:

```vim
Plug 'jose-elias-alvarez/null-ls.nvim'
```

```lua
require 'null-ls'.setup {
  sources = {
    -- ... Your other sources ...

    -- Kodachi completion:
    require 'kodachi.null-ls.completion',
  },
}
```


[mud]: https://en.wikipedia.org/wiki/MUD
[nvim]: https://neovim.io
[judo]: https://github.com/dhleong/judo
[iaido]: https://github.com/dhleong/iaido
[rust]: https://www.rust-lang.org
[plug]: https://github.com/junegunn/vim-plug
[null-ls]: https://github.com/jose-elias-alvarez/null-ls.nvim
[eor]: https://tintin.mudhalla.net/protocols/eor/
[mtts]: https://mudhalla.net/tintin/protocols/mtts/
[mccp2]: https://tintin.mudhalla.net/protocols/mccp/
[msdp]: https://tintin.mudhalla.net/protocols/msdp/
[naws]: https://datatracker.ietf.org/doc/html/rfc1073
[help-kodachi]: doc/kodachi.md
