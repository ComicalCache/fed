#![feature(path_absolute_method)]

mod cli;
mod debug_panic;
mod decorators;
mod fed;
mod input;
mod layers;
mod modes;
mod newtype;
mod protocols;
mod render;
mod state;
mod types;
mod util;

use std::io::stdout;

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
    cli::Cli,
    fed::Fed,
    input::{
        InputRouter, KeyInputHandler, MouseInputHandler, PasteInputHandler, ResizeInputHandler,
    },
    modes::{
        dir::{DirKeyInput, DirMouseInput},
        insert::{InsertKeyInput, InsertMouseInput},
        mini_buffer::{MiniBufferKeyInput, MiniBufferMouseInput},
        normal::{NormalKeyInput, NormalMouseInput},
        search::{SearchKeyInput, SearchMouseInput},
        visual::{VisualKeyInput, VisualMouseInput},
    },
    protocols::{
        action::{ActionCommand, ActionProtocol},
        dir::{DirCommand, DirProtocol},
        doc_view::{DocViewCommand, DocViewProtocol},
        io::{IoCommand, IoProtocol},
        mini_buffer::{MiniBufferProtocol, MiniBufferResizeInput},
        quit::{QuitCallback, QuitProtocol},
        screen::{ScreenProtocol, ScreenResizeInput},
        view::{ViewProtocol, ViewResizeInput},
    },
    render::{WindowId, Workspace},
    state::{State, StateLock},
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
    cli: Cli, input_rx: UnboundedReceiver<Event>, width: usize, height: usize,
) -> (Fed, UnboundedReceiver<()>) {
    // Channels.
    let (doc_event_tx, _) = broadcast::channel(32);
    let (view_event_tx, _) = broadcast::channel(32);

    let (view_tx, view_rx) = unbounded_channel();
    let (doc_view_tx, doc_view_rx) = unbounded_channel();
    let (dir_tx, dir_rx) = unbounded_channel();
    let (io_tx, io_rx) = unbounded_channel();
    let (action_tx, action_rx) = unbounded_channel();
    let (mini_buffer_tx, mini_buffer_rx) = unbounded_channel();
    let (screen_tx, screen_rx) = unbounded_channel();
    let (quit_tx, quit_rx) = unbounded_channel();
    let (shutdown_tx, shutdown_rx) = unbounded_channel();

    // Initialize application state.
    let state = State::new(
        Workspace::new(Rect::new(Pos::default(), width, height)),
        doc_event_tx.clone(),
        view_event_tx.clone(),
    );

    let state_lock = StateLock::new(state);

    // Protocols.
    let action = ActionProtocol::new(state_lock.clone(), action_rx);
    let io = IoProtocol::new(io_rx);
    let mini_buffer = MiniBufferProtocol::new(
        width,
        height,
        state_lock.clone(),
        mini_buffer_rx,
        action_tx.clone(),
        doc_view_tx.clone(),
    );
    let screen = ScreenProtocol::new(state_lock.clone(), width, height, screen_rx);
    let view = ViewProtocol::new(
        state_lock.clone(),
        view_rx,
        doc_event_tx.subscribe(),
        view_event_tx.subscribe(),
        screen_tx.clone(),
    );
    let doc_view = DocViewProtocol::new(state_lock.clone(), doc_view_rx, view_tx.clone());
    let dir = DirProtocol::new(
        state_lock.clone(),
        dir_rx,
        action_tx.clone(),
        doc_view_tx.clone(),
        mini_buffer_tx.clone(),
        io_tx.clone(),
        view_tx.clone(),
    );

    // Quit callbacks.
    let action_tx_clone = action_tx.clone();
    let io_tx_clone = io_tx.clone();
    let callbacks: Vec<QuitCallback> = vec![
        Box::new(move || {
            let (tx, rx) = oneshot::channel();
            let _ = action_tx_clone.send(ActionCommand::CanQuit { tx });

            rx
        }),
        Box::new(move || {
            let (tx, rx) = oneshot::channel();
            let _ = io_tx_clone.send(IoCommand::CanQuit { tx });

            rx
        }),
    ];
    let quit = QuitProtocol::new(callbacks, quit_rx, mini_buffer_tx.clone(), shutdown_tx.clone());

    // Input handlers.
    let key_handlers: Vec<Box<dyn KeyInputHandler>> = vec![
        Box::new(MiniBufferKeyInput::new(
            state_lock.clone(),
            action_tx.clone(),
            mini_buffer_tx.clone(),
        )),
        Box::new(SearchKeyInput::new(
            state_lock.clone(),
            action_tx.clone(),
            mini_buffer_tx.clone(),
        )),
        Box::new(VisualKeyInput::new(
            state_lock.clone(),
            action_tx.clone(),
            mini_buffer_tx.clone(),
        )),
        Box::new(DirKeyInput::new(
            state_lock.clone(),
            action_tx.clone(),
            mini_buffer_tx.clone(),
            quit_tx.clone(),
            view_tx.clone(),
            dir_tx.clone(),
        )),
        Box::new(InsertKeyInput::new(state_lock.clone(), action_tx.clone())),
        Box::new(NormalKeyInput::new(
            state_lock.clone(),
            action_tx.clone(),
            io_tx.clone(),
            mini_buffer_tx.clone(),
            quit_tx.clone(),
            view_tx.clone(),
            dir_tx.clone(),
        )),
    ];

    let mouse_handlers: Vec<Box<dyn MouseInputHandler>> = vec![
        Box::new(MiniBufferMouseInput::new(state_lock.clone(), action_tx.clone())),
        Box::new(SearchMouseInput::new(state_lock.clone(), action_tx.clone())),
        Box::new(VisualMouseInput::new(state_lock.clone(), action_tx.clone())),
        Box::new(DirMouseInput::new(state_lock.clone(), action_tx.clone())),
        Box::new(InsertMouseInput::new(state_lock.clone(), action_tx.clone())),
        Box::new(NormalMouseInput::new(state_lock.clone(), action_tx.clone())),
    ];
    let paste_handlers: Vec<Box<dyn PasteInputHandler>> = vec![];
    let resize_handlers: Vec<Box<dyn ResizeInputHandler>> = vec![
        Box::new(ScreenResizeInput::new(state_lock.clone(), screen_tx.clone())),
        Box::new(MiniBufferResizeInput::new(mini_buffer_tx.clone())),
        Box::new(ViewResizeInput::new(view_tx.clone())),
    ];
    let input_router = InputRouter::new(
        key_handlers,
        mouse_handlers,
        paste_handlers,
        resize_handlers,
        state_lock.clone(),
        input_rx,
    );

    // Initialize state requiring protocols..
    tokio::spawn(async move {
        let path = cli.path.map(|p| util::path::normalize(p));
        let dir = path.clone().map(|p| p.is_dir()).unwrap_or_default();

        let doc = util::create_doc(state_lock.clone(), path.clone(), io_tx.clone()).await;

        let (tx, rx) = oneshot::channel();
        let _ = doc_view_tx.send(DocViewCommand::CreateTile {
            doc,
            view: None,
            // This is kind of a hack: since no other windows exist, we can pass anything
            // because a new root window will be created in any case.
            split_window: WindowId(0),
            direction: RectSplit::Vertical,
            raw: false,
            tx,
        });
        let Ok((_, window)) = rx.await else { return };

        state_lock.write().workspace.active_window = Some(window);

        if dir {
            let _ = std::env::set_current_dir(path.unwrap());

            let _ = dir_tx.send(DirCommand::Init);
            let _ = dir_tx.send(DirCommand::ReplaceWindow { window });
        } else {
            let _ = dir_tx.send(DirCommand::Init);
        }
    });

    (
        Fed::new(input_router, action, io, mini_buffer, quit, screen, view, doc_view, dir),
        shutdown_rx,
    )
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, EnableMouseCapture)?;
    execute!(stdout, Hide)?;

    // Channels to propagate input `Event`s.
    let (input_tx, input_rx) = unbounded_channel();

    let (width, height) = crossterm::terminal::size()?;
    let (mut fed, mut shutdown_rx) = setup(cli, input_rx, width as usize, height as usize);

    // Main loop.
    tokio::select! {
        _ = input_events(input_tx) => {}
        _ = fed.run() => {}
        _ = shutdown_rx.recv() => {}
    }

    execute!(stdout, Show)?;
    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, LeaveAlternateScreen)?;

    disable_raw_mode()
}
