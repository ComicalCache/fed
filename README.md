# fed

`fed` is an asynchronous, RPC based, extendable terminal text editor powered by the `fed-core`
micro-kernel. It is the spiritual successor to [Cini](https://github.com/ComicalCache/Cini) (which
itself is the spiritual successor to [Mini](https://github.com/ComicalCache/Mini)).

See `architecture` to learn more about the editor's design and architecture. Since this is my
first time designing a RPC based micro-kernel I'm bound to make ill-decisions in the design and
implementation. If you have any suggestions, feel free to reach out or participate.

## Building and development

The project can simply be built using `cargo build` or `cargo build -r`. It is recommended to read
`architecture` and check out `fed-core/src/messages.rs` and `fed-core/src/event_mapper.rs`, as
well as the `fed` implementation for reference.

The release build will be a stripped binary with LTO and compiled with `march=native` flag. This
behaviour can be changed in `Cargo.toml` and `.cargo/config.toml`, if desired.

## fed-core

`fed-core` is technically an independent micro-kernel used by `fed`. For ease of usage within `fed`,
I chose to use cargo's workspace feature. It can be fully extracted and decoupled by modifying its
`Cargo.toml` and replacing all `workspace` references and relative paths by the actual values.

## piece-table

`piece-table` is a fully featured implementation of a
[piece table](https://en.wikipedia.org/wiki/Piece_table) including a commit based undo/redo tree. It
can be fully extracted as a standalone crate without any modifications necessary.
