#![feature(path_absolute_method)]
#![feature(option_into_flat_iter)]

mod cli;
mod decorators;
mod fed;
mod input_router;
mod layers;
mod modes;
mod protocol_router;
mod protocols;
mod state;
mod types;
mod util;

use std::{
    backtrace::{Backtrace, BacktraceStatus},
    fs::OpenOptions,
    io::{IsTerminal, Read, Write, stdout},
    os::fd::AsRawFd,
};

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
    mut cli: Cli, input_rx: UnboundedReceiver<Event>, shutdown_tx: UnboundedSender<()>,
    width: usize, height: usize,
) -> Fed {
    cli.path = cli.path.map(util::path::normalize);

    let (fed, async_fcmd_tx, doc) =
        Fed::new(width, height, cli.path.clone(), input_rx, shutdown_tx);

    tokio::spawn(async move { Fed::stage1(async_fcmd_tx, doc, cli).await });

    fed
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("fed.log") {
            let _ = writeln!(file, "{info}");
            let _ = writeln!(file, "{}", Backtrace::force_capture());
        }

        // Try to restore the terminal.
        let mut stdout = stdout();
        let _ = execute!(stdout, Show);
        let _ = execute!(stdout, DisableMouseCapture);
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = disable_raw_mode();

        let backtrace = Backtrace::capture();
        eprintln!("{info}");
        if backtrace.status() == BacktraceStatus::Captured {
            eprintln!("{backtrace}");
        }
    }));

    let mut cli = Cli::parse();

    let mut piped = None;
    if !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        let _ = std::io::stdin().read_to_string(&mut buf);
        piped = Some(buf);

        // Reconnect the terminal to stdin.
        if let Ok(tty) = std::fs::File::open("/dev/tty") {
            unsafe {
                libc::dup2(tty.as_raw_fd(), libc::STDIN_FILENO);
            }
        } else {
            eprintln!("Failed to reconnect the terminal to stdin");
            std::process::exit(1);
        }
    }
    cli.piped = piped;

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
        () = fed.run() => {}
        _ = shutdown_rx.recv() => {}
    }

    execute!(stdout, Show)?;
    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, LeaveAlternateScreen)?;

    disable_raw_mode()
}
