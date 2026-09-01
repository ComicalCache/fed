mod fed;
mod input;
mod modes;
mod newtype;
mod protocols;
mod render;
mod state;
mod type_map;
mod types;

use std::{
    io::stdout,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use fed_core::Core;
use futures::StreamExt;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::{
    fed::Fed,
    input::InputRouter,
    modes::normal::{NormalKeyInput, NormalMouseInput},
    protocols::{
        buffer::{BufferCommand, BufferProtocol, BufferRenderer, BufferResizeInput},
        cursor::CursorProtocol,
        screen::{ScreenProtocol, ScreenResizeInput},
    },
    render::Workspace,
    state::State,
    types::{Pos, Rect, RectSplit},
};

/// Broadcasts input `Event`s into the application.
async fn input_events(tx: UnboundedSender<Event>) -> std::io::Result<()> {
    let mut reader = EventStream::new();

    while let Some(Ok(event)) = reader.next().await {
        if tx.send(event).is_err() {
            break;
        }
    }

    Ok(())
}

async fn setup(
    input_rx: UnboundedReceiver<Event>, width: usize, height: usize,
) -> (Fed, UnboundedReceiver<()>) {
    let mut core = Core::new();
    let core_tx = core.tx();

    // The core lives in the background since it is not "owned" by any part of the
    // editor and needed for setup.
    tokio::spawn(async move {
        core.run().await;
    });

    let state = State {
        workspace: Arc::new(RwLock::new(Workspace::new(Rect::new(Pos::default(), width, height)))),
        ..Default::default()
    };

    // Get path of initial document.
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(PathBuf::from);

    // Initial document.
    let doc = state::create_document(&state, core_tx.clone(), path)
        .await
        .expect("Failed to create document");
    let view = state::create_view(&state, doc);

    // Channels.
    let (buffer_tx, buffer_rx) = unbounded_channel();
    let (cursor_tx, cursor_rx) = unbounded_channel();
    let (screen_tx, screen_rx) = unbounded_channel();
    let (quit_tx, quit_rx) = unbounded_channel();

    // Protocols.
    let buffer = BufferProtocol::new(state.clone(), buffer_rx, core_tx.clone());
    let cursor = CursorProtocol::new(state.clone(), cursor_rx, buffer_tx.clone(), core_tx);
    let screen = ScreenProtocol::new(state.clone(), width, height, screen_rx);

    // Initial renderer.
    let renderer = Box::new(BufferRenderer::new(doc, view, buffer.store(), state.clone()));
    let window = state.workspace.write().unwrap().create_tile(renderer, RectSplit::Vertical);

    state.window_view_map.write().unwrap().insert(window, view);

    let _ = buffer_tx.send(BufferCommand::Init { window, view, doc });

    // Setup input handlers.
    let mut input_router = InputRouter::new(input_rx);
    input_router.add_key_handler(Box::new(NormalKeyInput::new(
        state.clone(),
        cursor_tx.clone(),
        quit_tx.clone(),
    )));

    input_router.add_mouse_handler(Box::new(NormalMouseInput::new(state.clone(), cursor_tx)));

    input_router.add_resize_handler(Box::new(BufferResizeInput::new(buffer_tx)));
    input_router.add_resize_handler(Box::new(ScreenResizeInput::new(state, screen_tx)));

    (Fed::new(input_router, buffer, cursor, screen), quit_rx)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, EnableMouseCapture)?;
    execute!(stdout, Hide)?;

    // Channels to propagate input `Event`s.
    let (input_tx, input_rx) = unbounded_channel();

    let (width, height) = crossterm::terminal::size()?;
    let (mut fed, mut quit_rx) = setup(input_rx, width as usize, height as usize).await;

    // Main loop.
    tokio::select! {
        _ = input_events(input_tx) => {}
        _ = fed.run() => {}
        _ = quit_rx.recv() => {} // TODO: proper quit protocol.
    }

    execute!(stdout, Show)?;
    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, LeaveAlternateScreen)?;

    disable_raw_mode()
}
