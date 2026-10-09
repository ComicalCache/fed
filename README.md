# fed

`fed` is an extendable terminal text editor. It is the spiritual successor to
[Cini](https://github.com/ComicalCache/Cini) (which itself is the spiritual successor to
[Mini](https://github.com/ComicalCache/Mini)).

> The project targets UNIX-like systems and WILL NOT compile for Windows. Even if it would,
> throughout the project a UNIX-like environment is assumed (e.g. no CRLF line endings in files) and 
> will cause undefined runtime behaviour otherwise.

## Building and development

The project can simply be built using `cargo build` or `cargo build -r`.

The release build will be a stripped binary with LTO and compiled with `march=native` flag. This
behaviour can be changed in `Cargo.toml` and `.cargo/config.toml`, if desired.

## piece-table

`piece-table` is a fully featured implementation of a
[piece table](https://en.wikipedia.org/wiki/Piece_table) including a commit based undo/redo tree. It
can be fully extracted as a standalone crate without any modifications necessary.
