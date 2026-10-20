use std::{
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use crate::{
    fed::FCmd,
    protocols::io::{
        IoCmd, ScanEntry,
        cmd::{RCallback, WCallback},
    },
    state::State,
};

pub struct IoProtocol {}

impl IoProtocol {
    pub fn exec(s: &mut State, cmd: IoCmd) {
        match cmd {
            IoCmd::Read { path, rcallback } => Self::read(path, rcallback),
            IoCmd::Write { path, data, wcallback } => Self::write(s, path, data, wcallback),
            IoCmd::Scan { path } => Self::scan(s, path),
            IoCmd::ScanCompletion { path, revision, entries } => {
                Self::scan_completion(s, path, revision, entries);
            }
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

    pub fn scan(s: &mut State, path: PathBuf) {
        s.p.io.scan_revision += 1;

        let revision = s.p.io.scan_revision;
        let tx = s.async_fcmd_tx.clone();
        tokio::spawn(async move {
            let entries = Self::scan_entries(&path).await;
            let _ = tx.send(FCmd::Io(IoCmd::ScanCompletion { path, revision, entries }));
        });
    }

    fn scan_completion(s: &mut State, path: PathBuf, revision: u64, entries: Vec<ScanEntry>) {
        if revision != s.p.io.scan_revision {
            return;
        }

        s.p.io.scan_list = entries;
        s.p.io.scan_root = Some(path);
    }

    pub fn can_quit(s: &State) -> Result<(), String> {
        let writers = s.p.io.writers.load(Ordering::Relaxed);
        if writers > 0 { Err(format!("{writers} ongoing writes in progress")) } else { Ok(()) }
    }

    async fn scan_entries(root: &Path) -> Vec<ScanEntry> {
        let mut entries = Vec::new();
        let mut directories = vec![root.to_path_buf()];
        while let Some(path) = directories.pop() {
            let Ok(metadata) = tokio::fs::metadata(&path).await else { continue };
            if !metadata.is_dir() {
                continue;
            }

            let Ok(mut directory) = tokio::fs::read_dir(&path).await else { continue };

            while let Ok(Some(entry)) = directory.next_entry().await {
                let path = entry.path();
                let Ok(file_type) = entry.file_type().await else { continue };

                if !file_type.is_dir() && !file_type.is_file() {
                    continue;
                }

                if file_type.is_dir() {
                    directories.push(path.clone());
                }

                entries.push(ScanEntry { path, dir: file_type.is_dir() });
            }
        }

        entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));

        entries
    }
}
