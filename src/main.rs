#![feature(path_absolute_method)]

mod cli;
mod debug_panic;
mod decorators;
mod fed;
mod input_handler;
mod input_router;
mod layers;
mod modes;
mod newtype;
mod protocol_router;
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
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::{cli::Cli, fed::Fed};

async fn input(tx: UnboundedSender<Event>) -> std::io::Result<()> {
    let mut reader = EventStream::new();

    while let Some(Ok(event)) = reader.next().await {
        if tx.send(event).is_err() {
            break;
        }
    }

    Ok(())
}

fn setup(
    cli: Cli, input_rx: UnboundedReceiver<Event>, shutdown_tx: UnboundedSender<()>, width: usize,
    height: usize,
) -> Fed {
    let path = cli.path.map(|p| util::path::normalize(p));

    let (fed, async_fcmd_tx, doc) = Fed::new(width, height, path.clone(), input_rx, shutdown_tx);

    let dir = path.clone().map(|p| p.is_dir()).unwrap_or_default();
    tokio::spawn(async move { Fed::stage1(async_fcmd_tx, doc, path, dir).await });

    fed
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, EnableMouseCapture)?;
    execute!(stdout, Hide)?;

    let (input_tx, input_rx) = unbounded_channel();
    let (shutdown_tx, mut shutdown_rx) = unbounded_channel();

    let (width, height) = crossterm::terminal::size()?;
    let mut fed = setup(cli, input_rx, shutdown_tx, width as usize, height as usize);

    // Main loop.
    tokio::select! {
        _ = input(input_tx) => {}
        _ = fed.run() => {}
        _ = shutdown_rx.recv() => {}
    }

    execute!(stdout, Show)?;
    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, LeaveAlternateScreen)?;

    disable_raw_mode()
}
