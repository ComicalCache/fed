use std::{path::PathBuf, sync::atomic::Ordering};

use crate::{
    protocols::io::{
        IoCmd,
        cmd::{RCallback, WCallback},
    },
    state::State,
};

pub struct IoProtocol {}

impl IoProtocol {
    pub fn exec(s: &State, cmd: IoCmd) {
        match cmd {
            IoCmd::Read { path, rcallback } => Self::read(path, rcallback),
            IoCmd::Write { path, data, wcallback } => Self::write(s, path, data, wcallback),
        }
    }

    pub fn read(path: PathBuf, rcallback: RCallback) {
        tokio::spawn(async move {
            rcallback(tokio::fs::read_to_string(&path).await.map_err(|err| err.to_string())).await;
        });
    }

    pub fn write(s: &State, path: PathBuf, data: String, wcallback: WCallback) {
        s.p.io.writers.fetch_add(1, Ordering::Relaxed);

        let writers = s.p.io.writers.clone();
        tokio::spawn(async move {
            let data = tokio::fs::write(&path, data).await.map_err(|err| err.to_string());
            writers.fetch_sub(1, Ordering::Relaxed);

            wcallback(data).await;
        });
    }

    pub fn can_quit(s: &State) -> Result<(), String> {
        let writers = s.p.io.writers.load(Ordering::Relaxed);
        if writers > 0 { Err(format!("{writers} ongoing writes in progress")) } else { Ok(()) }
    }
}
