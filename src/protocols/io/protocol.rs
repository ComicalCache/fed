use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use ignore::WalkBuilder;

use crate::fed::FCmd;
use crate::protocols::io::IoCmd;
use crate::protocols::io::IoFileEntry;
use crate::protocols::io::cmd::RCallback;
use crate::protocols::io::cmd::WCallback;
use crate::state::State;

pub struct IoProtocol {}

impl IoProtocol {
    pub fn exec(s: &mut State, cmd: IoCmd) {
        match cmd {
            IoCmd::Read { path, rcallback } => Self::read(path, rcallback),
            IoCmd::Write { path, data, wcallback } => Self::write(s, path, data, wcallback),
            IoCmd::Scan { path } => Self::scan(s, path),
            IoCmd::ScanCompletion { entries, revision } => {
                Self::scan_completion(s, entries, revision);
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
        tokio::task::spawn_blocking(move || {
            let entries = Self::scan_entries(&path);
            let _ = tx.send(FCmd::Io(IoCmd::ScanCompletion { entries, revision }));
        });
    }

    fn scan_completion(s: &mut State, entries: Vec<IoFileEntry>, revision: u64) {
        if revision != s.p.io.scan_revision {
            return;
        }

        s.p.io.scan_list = entries;
    }

    pub fn can_quit(s: &State) -> Result<(), String> {
        let writers = s.p.io.writers.load(Ordering::Relaxed);
        if writers > 0 { Err(format!("{writers} ongoing writes in progress")) } else { Ok(()) }
    }

    fn scan_entries(root: &Path) -> Vec<IoFileEntry> {
        let mut entries = Vec::new();

        let walker = WalkBuilder::new(root)
            .hidden(false)
            .require_git(false)
            .filter_entry(|e| e.file_name() != ".git")
            .build();
        for entry in walker {
            let Ok(entry) = entry else { continue };
            if entry.depth() == 0 {
                continue;
            }

            let Some(file_type) = entry.file_type() else { continue };
            if !file_type.is_dir() && !file_type.is_file() {
                continue;
            }

            let path = entry.into_path();
            let relative_path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            entries.push(IoFileEntry { path, relative_path, dir: file_type.is_dir() });
        }

        entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));

        entries
    }
}
