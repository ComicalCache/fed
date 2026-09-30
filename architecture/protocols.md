# Protocols

Protocols are the asynchronous background jobs of `fed`. Protocols execute logic, synchronize state,
and communicated with each other.

## File Structure

Each protocol lives in its own directory under `src/protocols/<name>/`. A protocol module is
composed of specific, optionally included files based on its functionality:

- `command.rs`: Contains the `Command` enum that defines its message API
- `protocol.rs`: Contains the main protocol struct and its asynchronous run loop
- `input/<input type>.rs` (optional): Implements the input handler traits if the protocol needs to
  intercept raw input events. This is generally discouraged except for resize events since modes are
  generally resposible for input handling
- `render.rs` (optional): Implements the `Renderer` trait if the protocol has a visual component
  that needs to be drawn
- `store.rs` (optional): Contains internal caching data structures owned exclusively by the
  protocol

## Naming Conventions

- Message API: `<Name>Command`
- Protocol: `<Name>Protocol`
- Input handler: `<Name><Input Type>Input`
- Renderer: `<Name>Renderer`

## Protocol-to-Protocol (P2P) communication

One-to-One messages should only be used if something specific is needed from another protocol.
Otherwise protocols should rely on the emitted One-to-Many events to trigger necessary side effects
like rendering.

### One-to-One

RPC-style requests where a protocol sends a message to another protocol. The capability registered
in the global map exposes a dedicated request channel, and may include a return channel to await
a response.

### One-to-Many

Event streaming where a protocol listenes to messages by another protocol. The capability registered
in the global map exposes a broadcast, allowing multiple protocols to subscribe to its continuous
message stream.
