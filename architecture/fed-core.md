# Core (Micro-Kernel)

The core defines the editors data structures and acts as a micro kernel, that sends out core events
on state change and receives core commands to modify the editor state. Message passing is done using
asynchronous channels. Rogue protocols could modify the underlying files of documents, this is
considered an error and thus not defined as state mutation. This makes the core act as the *single*
source-of-truth for the editor state and as the *single* entity that can modify the editor's state.

## Data Structures

### Document

The document struct is the core container wrapping the state of a document. It contains the
underlying file (if applicable). Documents provide primitives to read from and write to the
document, as well as persist it as a file on permanent storage.

### Core

The core data structure holds all opened documents in memory, as well as the event mappers.

# Protocols (Functionality)

Protocols define the functionality of `fed`. The core itself only knows state, and _how_ to modify
it, but not _when_ to modify it. This is the purpose of protocols. The core provides a set of events
regarding state change and commands for changing state (messages).

## Event Mapping

Because of the asynchronous nature of `fed` and Rust, core messages must be mapped to protocol
specific messages. This is necessary to make the protocol's message handling simple to write, for 
messages from both the core (which will be wrapped), as well as messages coming from other protocols 
or the front-end. This also means complex protocol messages can be expressed by single protocol
messages and, only on demand, translated to a series of core messages. The other way around the core 
only emits core messages, making it agnostic towards installed protocols.

For this purpose there is a global list event mappers to map core events. Protocols can register
their event mapping functionality, and the core will call all registered mappers. The
*synchronous* nature of the mapper list means that event mapping should be a fast operation or may
be delegated to a separate worker to not block the following protocol mappers. Mapping protocol
commands to core commands is left to the individual protocols.

## Protocol-to-Protocol (P2P) communication

Since the core is agnostic to protocol messages, protocols should register their capability in a
global map to enable P2P communication. By registering in a global map, protocols can discover the
capabilities of the editor, as well as having the means to communicate with each other.

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
