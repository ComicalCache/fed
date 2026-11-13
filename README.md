# fed

`fed` is an extendable terminal text editor. It is the spiritual successor to
[Cini](https://github.com/ComicalCache/Cini) (which itself is the spiritual successor to
[Mini](https://github.com/ComicalCache/Mini)).

> The project targets UNIX-like systems and WILL NOT compile for Windows. Even if it would,
> throughout the project a UNIX-like environment is assumed (e.g. no CRLF line endings in files) and 
> will cause undefined runtime behaviour otherwise.

## Features

`fed` is a feature rich modal editor. It features a regular viewing and editing experience with
native multi cursor support and edit history. It supports selections, search & replace, cross file
search & replace in the current `pwd`, a file browser, and fuzzy file finder. Additionally it
supports window splits and theming. All of its features and corresponding keybinds can be explored
in the keybind browser using `alt+h`.

## Building and development

The project can simply be built using `cargo build` or `cargo build -r`.

The release build will be a stripped binary with LTO and compiled with `march=native` flag. This
behaviour can be changed in `Cargo.toml` and `.cargo/config.toml`, if desired.

> Disabling LTO will drastically reduce compile times.

## Piece Table

`src/piece_table` is a fully featured implementation of a
[piece table](https://en.wikipedia.org/wiki/Piece_table) including a commit based undo/redo tree. It
is fully self-contained and can simply be extracted if so desired.
