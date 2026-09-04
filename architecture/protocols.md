# Protocols

Protocols are the asynchronous background jobs of `fed`. Protocols execute logic, synchronize state,
and communicated with each other.

## File Structure

Each protocol lives in its own directory under `src/protocols/<name>/`. A protocol module is
composed of specific, optionally included files based on its functionality:

- `protocol.rs`: Contains the main protocol struct, its asynchronous run loop, and the `Command`
  enum that defines its message API
- `input/<input type>.rs` (optional): Implements the input handler traits if the protocol needs to
  intercept raw input events. This is generally discouraged except for resize events since modes are
  generally resposible for input handling
- `render.rs` (optional): Implements the `Renderer` trait if the protocol has a visual component
  that needs to be drawn
- `store.rs` (optional): Contains internal caching data structures owned exclusively by the
  protocol

## Naming Conventions

- Protocol: `<Name>Protocol`
- Message API: `<Name>Command`
- Input handler: `<Name><Input Type>Input`
- Renderer: `<Name>Renderer`

## Communication

Protocols run concurrently and never share memory directly with other protocols. Instead they rely
on message passing and global state changes.
