mod fed;
mod input;
mod modes;
mod newtype;
mod protocols;
mod render;
mod state;
mod types;

use std::{io::stdout, path::PathBuf};

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
    oneshot,
};

use crate::{
    fed::Fed,
    input::InputRouter,
    modes::{
        insert::{InsertKeyInput, InsertMouseInput},
        mini_buffer::{MiniBufferKeyInput, MiniBufferMouseInput},
        normal::{NormalKeyInput, NormalMouseInput},
    },
    protocols::{
        action::ActionProtocol,
        io::{IoCommand, IoProtocol},
        mini_buffer::{MiniBufferProtocol, MiniBufferResizeInput},
        screen::{ScreenProtocol, ScreenResizeInput},
        view::{ViewCommand, ViewProtocol, ViewResizeInput},
    },
    render::Workspace,
    state::{State, StateLock, ViewStoreTypes::Layout},
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

fn setup(
    input_rx: UnboundedReceiver<Event>, width: usize, height: usize,
) -> (Fed, UnboundedReceiver<()>) {
    // Channels.
    // let (doc_event_tx, doc_event_rx) = broadcast::channel(32);

    let (view_tx, view_rx) = unbounded_channel();
    let (io_tx, io_rx) = unbounded_channel();
    let (action_tx, action_rx) = unbounded_channel();
    let (mini_buffer_tx, mini_buffer_rx) = unbounded_channel();
    let (screen_tx, screen_rx) = unbounded_channel();
    let (quit_tx, quit_rx) = unbounded_channel();

    // Initialize application state.
    let mut state = State {
        workspace: Workspace::new(Rect::new(Pos::default(), width, height)),
        ..Default::default()
    };
    state.mini_buffer_store.doc = state.create_document(None, String::new());
    state.mini_buffer_store.view = state.create_view(state.mini_buffer_store.doc);

    let mini_buffer_view = state.mini_buffer_store.view;
    let mini_buffer_vse =
        state.view_store.get_mut(&mini_buffer_view).expect("Mini buffer view must exist");
    mini_buffer_vse.layout = Layout { gutter: false, mode_line: 0 };
    mini_buffer_vse.cursors.list.clear();

    let state_lock = StateLock::new(state);

    // Protocols.
    let action =
        ActionProtocol::new(state_lock.clone(), action_rx, view_tx.clone(), screen_tx.clone());
    let io = IoProtocol::new(io_rx);
    let mini_buffer = MiniBufferProtocol::new(
        width,
        height,
        state_lock.clone(),
        mini_buffer_rx,
        action_tx.clone(),
        view_tx.clone(),
        screen_tx.clone(),
    );
    let screen = ScreenProtocol::new(state_lock.clone(), width, height, screen_rx);
    let view = ViewProtocol::new(state_lock.clone(), view_rx, screen_tx.clone());

    // Input handlers.
    let mut input_router = InputRouter::new(state_lock.clone(), input_rx);
    input_router.add_key_handler(Box::new(NormalKeyInput::new(
        state_lock.clone(),
        action_tx.clone(),
        mini_buffer_tx.clone(),
        quit_tx.clone(),
    )));
    input_router
        .add_key_handler(Box::new(InsertKeyInput::new(state_lock.clone(), action_tx.clone())));
    input_router.add_key_handler(Box::new(MiniBufferKeyInput::new(
        state_lock.clone(),
        action_tx.clone(),
        mini_buffer_tx.clone(),
    )));

    input_router
        .add_mouse_handler(Box::new(NormalMouseInput::new(state_lock.clone(), action_tx.clone())));
    input_router
        .add_mouse_handler(Box::new(InsertMouseInput::new(state_lock.clone(), action_tx.clone())));
    input_router.add_mouse_handler(Box::new(MiniBufferMouseInput::new(
        state_lock.clone(),
        action_tx.clone(),
    )));

    input_router.add_resize_handler(Box::new(ViewResizeInput::new(view_tx.clone())));
    input_router.add_resize_handler(Box::new(MiniBufferResizeInput::new(mini_buffer_tx.clone())));
    input_router.add_resize_handler(Box::new(ScreenResizeInput::new(
        state_lock.clone(),
        screen_tx.clone(),
    )));

    // Initial document creation.
    tokio::spawn(async move {
        let args: Vec<String> = std::env::args().collect();
        let path = args.get(1).map(PathBuf::from);

        let data = if let Some(path) = path.clone() {
            let (tx, rx) = oneshot::channel();
            if io_tx.send(IoCommand::Read { path, tx }).is_err() {
                todo!("Exit with error");
            }

            rx.await.map_err(|err| err.to_string()).flatten().unwrap_or_else(|_| String::new())
        } else {
            String::new()
        };

        let doc = state_lock.write().create_document(path, data);

        let (tx, rx) = oneshot::channel();
        let _ = view_tx.send(ViewCommand::CreateTile {
            doc,
            view: None,
            split: RectSplit::Vertical,
            tx,
        });

        let Ok((_, window)) = rx.await else { return };
        state_lock.write().workspace.active_window = Some(window);
    });

    (Fed::new(input_router, action, io, mini_buffer, screen, view), quit_rx)
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
    let (mut fed, mut quit_rx) = setup(input_rx, width as usize, height as usize);

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
