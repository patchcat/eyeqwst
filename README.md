# eyeqwst

## eyeqwst what is?

eyeqwst is a [Quaddle](/QWD/Quaddle) client, written with [iced](https://iced.rs). It supports all 4 endpoints in Quaddle as of writing this.

eyeqwst is built using quaddlecl, a Quaddle client library, which lives under the `crates` directory of this repo. You may find it useful for developing your own bots.

This is still a fairly early version of eyeqwst, so expect bugs and a lack of polish.

## how do i get this?

You will need the latest stable version of Rust.

``` sh
git clone https://codeberg.org/Makefile_dot_in/Quaddle
cd Quaddle
cargo build --release
# the binary will be in target/release/eyeqwst
```

You may also opt to get a binary from the [releases](https://codeberg.org/Makefile_dot_in/eyeqwst/releases) page.

To run eyeqwst on Linux, you will also need various dynamic libraries. You can find a full list in `flake.nix` (but you probably already have them). `flake.nix` can also be used to install eyeqwst on NixOS from the `default` package, or from the `eyeqwst-wrapped` package on non-NixOS distros using Nix.

## usage

Most of the UI is pretty intuitive, I think, but it is worth noting that eyeqwst saves a config file at `$CONFIG_DIR/eyeqwst/config.json`, where `$CONFIG_DIR` is:

- `$XDG_CONFIG_DIR` or `$HOME/.config` on Linux
- `$HOME/Library/Application Support` on macOS
- `%USERPROFILE%\AppData\Roaming` on Windows

Currently, editing this file is the only way to do things like remove or edit added channels.

## features

- [x] logging in
- [x] signing up 
- [x] adding channels
- [x] sending messages
- [x] viewing messages and their history
- [x] error reporting for gateway errors
- [x] saving added channels between sessions
- [ ] removing/editing added channels
- [ ] logging out without restarting
- [ ] saving credentials
- [ ] error reporting for HTTP errors
- [ ] user info
- [ ] WASM support

## known bugs

- bugs that will be fixed when a custom text widget is written
  - messages are not selectable
  - word wrap doesn't work on long words
  - the editor is a bit goofy sometimes

## what's the license?

The license for eyeqwst is GPLv3, with the exception of the `assets/` directory, which contains fonts that are licensed under their respective licenses. The license for `quaddlecl` is ISC. All copyright belongs to me as of writing
