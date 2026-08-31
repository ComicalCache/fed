# Input

Input gets routed from the front-end receiving the input event into the application to the
asynchronous protocols.

## Input Router

The input router receives all input events from the front-end and forwards them _synchronously_ to
registered input handlers. Input handlers, which implement one of `KeyInputHandler`,
`MouseInputHandler`, `PasteInputHandler` or `ResizeInputHanlder` traits, are sorted by priority, so
that the highest priority handlers get a chance to process the input event first. If a handler
successfully processes the input, the handlers following it wont receive the input. 

## Modes

`fed` uses a modal system where each mode acts as a keybind registry. A mode implements the
input handler traits for that purpose. Since modes are persistent in memory during runtime they take
care of state tracking (e.g. multi-key command state machine). Modes have to follow a certain
protocol to ensure events are processed by the correct mode.

1. Query the currently active view
2. Query the view store to verify the appropriate mode is active
3. Process the input event and determine the command to execute
4. Execute the command

### File Structure

Each mode lives in its own directory under `src/modes/<name>/`. A mode module is composed of
specific, optionally included files based on its functionality:

- `key.rs`: Contains the key input handler
- `mouse.rs`: Contains the mouse input handler
- `paste.rs`: Contains the paste input handler
- `resize.rs`: Contains the resize input handler

### Naming Conventions

- Key event: `<Name>KeyInput`
- Mouse event: `<Name>MouseInput`
- Paste event: `<Name>PasteInput`
- Resize event: `<Name>ResizeInput`
