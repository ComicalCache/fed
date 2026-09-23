# fed

## Data Structures

### Doc

The doc struct is the core container wrapping the state of a document. It contains the
underlying file (if applicable). Docs provide primitives to read from and write to the
document, as well as persist it as a file on permanent storage.

## Protocol-to-Protocol (P2P) communication

Protocols should register their capability in a global map to enable P2P communication. By
registering in a global map, protocols can discover the capabilities of the editor, as well as
having the means to communicate with each other. One-to-One messages should only be used if something
specific is needed from another protocol. Otherwise protocols should rely on the emitted One-to-Many
events to trigger necessary side effects like rendering.

### One-to-One

RPC-style requests where a protocol sends a message to another protocol. The capability registered
in the global map exposes a dedicated request channel, and may include a return channel to await
a response.

### One-to-Many

Event streaming where a protocol listenes to messages by another protocol. The capability registered
in the global map exposes a broadcast, allowing multiple protocols to subscribe to its continuous
message stream.

# Front-End (Rendering & Input)

The front-end is responsible for printing the editor's state to the screen and forwarding user input
into the application. It is entirely up to the implementation on how this is done.
