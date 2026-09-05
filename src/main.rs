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
use futures::StreamExt;
use tokio::sync::{
    broadcast,
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};

use crate::{
    fed::Fed,
    input::InputRouter,
    modes::{
        insert::{InsertKeyInput, InsertMouseInput},
        normal::{NormalKeyInput, NormalMouseInput},
    },
    protocols::{
        action::ActionProtocol,
        io::IoProtocol,
        screen::{ScreenProtocol, ScreenResizeInput},
        view::{ViewCommand, ViewProtocol, ViewResizeInput},
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
    // Channels.
    // let (doc_event_tx, doc_event_rx) = broadcast::channel(32);

    let (view_tx, view_rx) = unbounded_channel();
    let (io_tx, io_rx) = unbounded_channel();
    let (action_tx, action_rx) = unbounded_channel();
    let (screen_tx, screen_rx) = unbounded_channel();
    let (quit_tx, quit_rx) = unbounded_channel();

    let state = State {
        workspace: Arc::new(RwLock::new(Workspace::new(Rect::new(Pos::default(), width, height)))),
        ..Default::default()
    };

    // Protocols.
    let io = IoProtocol::new(io_rx);
    let view = ViewProtocol::new(state.clone(), view_rx);
    let action = ActionProtocol::new(state.clone(), action_rx, view_tx.clone());
    let screen = ScreenProtocol::new(state.clone(), width, height, screen_rx);

    // Input handlers.
    let mut input_router = InputRouter::new(input_rx);
    input_router.add_key_handler(Box::new(NormalKeyInput::new(
        state.clone(),
        action_tx.clone(),
        quit_tx.clone(),
    )));
    input_router.add_key_handler(Box::new(InsertKeyInput::new(state.clone(), action_tx.clone())));

    input_router
        .add_mouse_handler(Box::new(NormalMouseInput::new(state.clone(), action_tx.clone())));
    input_router.add_mouse_handler(Box::new(InsertMouseInput::new(state.clone(), action_tx)));

    input_router.add_resize_handler(Box::new(ViewResizeInput::new(view_tx.clone())));
    input_router.add_resize_handler(Box::new(ScreenResizeInput::new(state.clone(), screen_tx)));

    // Initial document creation.
    tokio::spawn(async move {
        let args: Vec<String> = std::env::args().collect();
        let path = args.get(1).map(PathBuf::from);

        let Ok(doc) = state.create_document(path, io_tx).await else {
            todo!("Exit with error");
        };

        let _ = view_tx.send(ViewCommand::SpawnWindow { doc, split: RectSplit::Vertical });
    });

    (Fed::new(input_router, io, view, action, screen), quit_rx)
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
