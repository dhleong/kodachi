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

- [x] Triggers and Prompts
- [ ] Aliases
- [.] Intelligent auto-completion (WIP)


[mud]: https://en.wikipedia.org/wiki/MUD
[nvim]: https://neovim.io
[judo]: https://github.com/dhleong/judo
[iaido]: https://github.com/dhleong/iaido
