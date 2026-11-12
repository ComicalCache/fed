use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;

use crate::fed::FCmd;
use crate::protocols::io::IoCmd;
use crate::protocols::io::IoFileEntry;
use crate::protocols::io::cmd::RCallback;
use crate::protocols::io::cmd::SCallback;
use crate::protocols::io::cmd::WCallback;
use crate::protocols::io::pstate::IoRequest;
use crate::state::State;
use crate::util;

pub struct IoProtocol {}

impl IoProtocol {
    pub fn exec(s: &mut State, cmd: IoCmd) {
        match cmd {
            IoCmd::Read { path, rcallback } => Self::read(s, path, rcallback),
            IoCmd::Write { path, data, wcallback, revision } => {
                Self::write(s, path, data, wcallback, revision);
            }
            IoCmd::Completion { path } => Self::completion(s, path),
            IoCmd::Scan { path, scallback } => Self::scan(s, path, scallback),
            IoCmd::ScanCompletion { entries, scallback, revision } => {
                Self::scan_completion(s, entries, scallback, revision);
            }
        }
    }

    pub fn read(s: &mut State, path: PathBuf, rcallback: RCallback) {
        Self::enqueue(s, path, IoRequest::Read { rcallback });
    }

    pub fn write(s: &mut State, path: PathBuf, data: String, wcallback: WCallback, revision: u64) {
        Self::enqueue(s, path, IoRequest::Write { data, wcallback, revision });
    }

    fn completion(s: &mut State, path: PathBuf) {
        let queue = s.p.io.queues.get_mut(&path).unwrap();

        if let Some(request) = queue.pop_front() {
            Self::start(s, path, request);
        } else {
            s.p.io.queues.remove(&path);
        }
    }

    pub fn scan(s: &mut State, path: PathBuf, scallback: SCallback) {
        s.p.io.scan_revision += 1;

        let revision = s.p.io.scan_revision;
        let tx = s.async_fcmd_tx.clone();
        tokio::task::spawn_blocking(move || {
            let entries = Self::scan_entries(&path);
            let _ = tx.send(FCmd::Io(IoCmd::ScanCompletion { entries, scallback, revision }));
        });
    }

    fn scan_completion(
        s: &mut State, entries: Vec<IoFileEntry>, scallback: SCallback, revision: u64,
    ) {
        if revision != s.p.io.scan_revision {
            return;
        }

        scallback(s, entries);
    }

    pub fn can_quit(s: &State) -> Result<(), String> {
        let requests: usize = s.p.io.queues.values().map(|queue| queue.len() + 1).sum();
        if requests > 0 { Err(format!("{requests} IO requests in progress")) } else { Ok(()) }
    }

    fn enqueue(s: &mut State, path: PathBuf, request: IoRequest) {
        let mut path = util::path::normalize(path);

        if let (Some(parent), Some(name)) = (path.parent(), path.file_name())
            && let Ok(parent) = parent.canonicalize()
        {
            path = parent.join(name);
        }

        match s.p.io.queues.entry(path.clone()) {
            Entry::Occupied(mut entry) => entry.get_mut().push_back(request),
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::new());
                Self::start(s, path, request);
            }
        }
    }

    fn start(s: &State, path: PathBuf, request: IoRequest) {
        let tx = s.async_fcmd_tx.clone();
        tokio::spawn(async move {
            match request {
                IoRequest::Read { rcallback } => {
                    let result =
                        tokio::fs::read_to_string(&path).await.map_err(|err| err.to_string());
                    rcallback(result).await;
                }
                IoRequest::Write { data, wcallback, revision } => {
                    let result = tokio::fs::write(&path, data).await.map_err(|err| err.to_string());
                    wcallback(result, revision).await;
                }
            }

            let _ = tx.send(FCmd::Io(IoCmd::Completion { path }));
        });
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
