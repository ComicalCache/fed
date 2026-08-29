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

use crate::{
    fed::Fed,
    input::{InputHandler, InputRouter},
    modes::normal::NormalInput,
    protocols::{
        buffer::{BufferCommand, BufferInput, BufferProtocol, BufferRenderer},
        cursor::CursorProtocol,
        screen::{ScreenInput, ScreenProtocol},
    },
    render::Workspace,
    state::State,
    types::{Rect, RectSplit},
};

/// Broadcasts input `Event`s into the application.
async fn input_events(tx: flume::Sender<Event>) -> std::io::Result<()> {
    let mut reader = EventStream::new();

    while let Some(Ok(event)) = reader.next().await {
        if tx.send_async(event).await.is_err() {
            break;
        }
    }

    Ok(())
}

async fn setup(
    input_rx: flume::Receiver<Event>, width: usize, height: usize,
) -> (Fed, flume::Receiver<()>) {
    let mut core = Core::new();
    let core_tx = core.tx();

    // The core lives in the background since it is not "owned" by any part of the
    // editor and needed for setup.
    tokio::spawn(async move {
        core.run().await;
    });

    let state = State {
        workspace: Arc::new(RwLock::new(Workspace::new(Rect::new(0, 0, width, height)))),
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
    let (buffer_tx, buffer_rx) = flume::unbounded();
    let (cursor_tx, cursor_rx) = flume::unbounded();
    let (screen_tx, screen_rx) = flume::unbounded();
    let (quit_tx, quit_rx) = flume::unbounded();

    // Protocols.
    let buffer = BufferProtocol::new(state.clone(), buffer_rx.clone(), core_tx.clone());
    let cursor = CursorProtocol::new(state.clone(), cursor_rx, buffer_tx.clone(), core_tx);
    let screen = ScreenProtocol::new(state.clone(), width, height, screen_rx);

    // Initial renderer.
    let renderer = Box::new(BufferRenderer::new(doc, view, buffer.store(), state.clone()));
    let window = state.workspace.write().unwrap().create_tile(renderer, RectSplit::Vertical);

    state.window_view_map.write().unwrap().insert(window, view);

    let _ = buffer_tx.send(BufferCommand::Init { window, view, doc });

    // Setup input handlers.
    let handlers: Vec<Box<dyn InputHandler>> = vec![
        Box::new(ScreenInput::new(state.clone(), screen_tx)),
        Box::new(NormalInput::new(state, cursor_tx, quit_tx)),
        Box::new(BufferInput::new(buffer_tx)),
    ];
    let input_router = InputRouter::new(handlers, input_rx);

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
    let (input_tx, input_rx) = flume::unbounded();

    let (width, height) = crossterm::terminal::size()?;
    let (mut fed, quit_rx) = setup(input_rx, width as usize, height as usize).await;

    // Main loop.
    tokio::select! {
        _ = input_events(input_tx) => {}
        _ = fed.run() => {}
        _ = quit_rx.recv_async() => {} // TODO: proper quit protocol.
    }

    execute!(stdout, Show)?;
    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, LeaveAlternateScreen)?;

    disable_raw_mode()
}
