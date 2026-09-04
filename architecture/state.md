# Data Layers

The state of `fed` is distributed across distinct layers. This separation ensures rendering doesn't
have to wait on background I/O and prevents deadlocks in asynchronous protocols.

## 1. Runtime State

The global `State` acts as a global application data cache. It's based on read-write locked data to
dynamically set and modify state at runtime. It is important that protocols accessing the global
state release all locks before making asynchronous calls to avoid deadlocks!

## 2. Protocol State

Protocols maintain their own, protocol-specific state that is not used/needed by other components.
Protocol-specific data doesn't necessarily have to be locked.
