# Renderer

The renderer doesn't fetch for changes, instead it gets notified by other protocols to re-render the
terminal. Upon waking up through an event, the entire event-queue is being drained and the most
up-to-date state is being fetched and drawn.

## Terminal

The terminal is abstracted into cells and the screen buffer.

### Cells

Cells contain the content of one cell on the terminal (one (wide) "character"). Characters are
defined by unicode grapheme clusters (this avoids trailing zero-width character decorations
occupying a cell causing rendering artifacts). A wide character is considered to only be in *one*
cell. The following cell must be marked as trailing. Cells furter hold styling information like
foreground and background colors, italics, bold, underline, etc...

### Screen Buffer

The screen buffer is a grid of cells that can be updated. The screen buffer is the internal
representation in the renderer.

### Workspace

The screen buffer is sectioned as a workspace, dividing it into individual rectangles (windows)
which correspond to a bounded viewport. The workspace manages creation/destruction of, resizing,
tiling and floating windows.

## Rendering

The renderer calls all the workspace to populate the screen buffer. Then the renderer reads the
screen buffer, and using diffed rendering, renders all changes to the terminal, using a double
buffer to avoid flickering.

# Render Protocols

Protocols can implement the `Renderer` trait. The workspace is populated using render protocols.
During rendering it calls the protocols in the workspace to populate the screen buffer handling
tiling and the z-Index of floating windows. Render protocols receive a viewport which is a bounded
proxy on the screen buffer, to avoid protocols drawing to parts on the screen they don't own.
