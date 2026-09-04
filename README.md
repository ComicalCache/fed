# fed

`fed` is an asynchronous, RPC based, extendable terminal text editor. It is the spiritual successor
to [Cini](https://github.com/ComicalCache/Cini) (which itself is the spiritual successor to [Mini]
(https://github.com/ComicalCache/Mini)).

See `architecture` to learn more about the editor's design and architecture. Since this is my
first time designing a large asynchronous programm I'm bound to make ill-decisions in the design and
implementation. If you have any suggestions, feel free to reach out or participate.

## Building and development

The project can simply be built using `cargo build` or `cargo build -r`. It is recommended to read
`architecture` for reference.

The release build will be a stripped binary with LTO and compiled with `march=native` flag. This
behaviour can be changed in `Cargo.toml` and `.cargo/config.toml`, if desired.

## piece-table

`piece-table` is a fully featured implementation of a
[piece table](https://en.wikipedia.org/wiki/Piece_table) including a commit based undo/redo tree. It
can be fully extracted as a standalone crate without any modifications necessary.
