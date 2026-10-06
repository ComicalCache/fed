use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{
    debug_panic::debug_panic,
    protocols::{
        action::ActionCommand,
        dir::{DirCommand, DirRenderer},
        doc_view::DocViewCommand,
        io::IoCommand,
        mini_buffer::MiniBufferCommand,
        view::ViewCommand,
    },
    render::WindowId,
    state::{DirTypes, State, StateLock},
    types::Pos,
    util::{self, format},
};

struct PrintableEntry {
    perms: String,
    links: String,
    owner: String,
    group: String,
    size: String,
    modified: String,
    name: String,
}

pub struct DirProtocol {
    state_lock: StateLock,

    rx: UnboundedReceiver<DirCommand>,
    action_tx: UnboundedSender<ActionCommand>,
    doc_view_tx: UnboundedSender<DocViewCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    io_tx: UnboundedSender<IoCommand>,
    view_tx: UnboundedSender<ViewCommand>,
}

impl DirProtocol {
    pub fn new(
        state_lock: StateLock, rx: UnboundedReceiver<DirCommand>,
        action_tx: UnboundedSender<ActionCommand>, doc_view_tx: UnboundedSender<DocViewCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>, io_tx: UnboundedSender<IoCommand>,
        view_tx: UnboundedSender<ViewCommand>,
    ) -> Self {
        Self { state_lock, rx, action_tx, doc_view_tx, mini_buffer_tx, io_tx, view_tx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                DirCommand::Init => self.init().await,
                DirCommand::ReplaceWindow { window } => self.replace_window(window),
                DirCommand::Select => self.select().await,
            }
        }
    }

    async fn init(&self) {
        let state = self.state_lock.read();
        let pwd = state.dir.pwd.clone();
        drop(state);

        let mut entries = Vec::new();
        let mut printables: Vec<PrintableEntry> = Vec::new();

        if let Some(parent) = pwd.parent() {
            if let Ok(metadata) = tokio::fs::metadata(parent).await {
                let (perms, links, owner, group) = {
                    // This breaks Windows compatability (lol).
                    use std::os::unix::fs::MetadataExt;

                    (
                        format::permissions(metadata.mode()),
                        metadata.nlink().to_string(),
                        format::user_name(metadata.uid()),
                        format::group_name(metadata.gid()),
                    )
                };

                printables.push(PrintableEntry {
                    perms: format!("d{}", perms),
                    links,
                    owner,
                    group,
                    size: format::size(metadata.len()),
                    modified: if let Ok(time) = metadata.modified() {
                        let datetime: chrono::DateTime<chrono::Local> = time.into();
                        datetime.format("%b %e %H:%M").to_string()
                    } else {
                        "n/a".to_string()
                    },
                    name: "../".to_string(),
                });
                entries.push(DirTypes::Entry {
                    path: parent.to_path_buf(),
                    kind: DirTypes::EntryKind::Dir,
                    path_offset: 0,
                });
            }
        }

        let mut dir_entries = Vec::new();
        let Ok(mut read_dir) = tokio::fs::read_dir(pwd).await else {
            // TODO: print error and clear document and state.
            return;
        };
        loop {
            let Ok(Some(entry)) = read_dir.next_entry().await else { break };
            dir_entries.push(entry);
        }
        dir_entries.sort_by(|a, b| {
            a.path()
                .file_name()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .cmp(&b.path().file_name().unwrap_or_default().to_ascii_lowercase())
        });

        for entry in dir_entries {
            let Ok(metadata) = entry.metadata().await else {
                // TODO: print error, should this just be silently
                //       dropped?
                continue;
            };

            let kind = if metadata.is_dir() {
                DirTypes::EntryKind::Dir
            } else if metadata.is_symlink() {
                DirTypes::EntryKind::Symlink
            } else {
                DirTypes::EntryKind::File
            };

            let (perms, links, owner, group) = {
                // This breaks Windows compatability (lol).
                use std::os::unix::fs::MetadataExt;

                (
                    format::permissions(metadata.mode()),
                    metadata.nlink().to_string(),
                    format::user_name(metadata.uid()),
                    format::group_name(metadata.gid()),
                )
            };

            let perms = match kind {
                DirTypes::EntryKind::File => format!("-{}", perms),
                DirTypes::EntryKind::Dir => format!("d{}", perms),
                DirTypes::EntryKind::Symlink => format!("l{}", perms),
            };
            let size = format::size(metadata.len());

            let modified = if let Ok(time) = metadata.modified() {
                let datetime: chrono::DateTime<chrono::Local> = time.into();
                datetime.format("%b %e %H:%M").to_string()
            } else {
                "n/a".to_string()
            };

            let mut name = entry.file_name().to_string_lossy().to_string();
            if matches!(kind, DirTypes::EntryKind::Dir) {
                name.push('/');
            } else if matches!(kind, DirTypes::EntryKind::Symlink) {
                if let Ok(target) = tokio::fs::read_link(entry.path()).await {
                    name.push_str(" -> ");
                    name.push_str(&target.to_string_lossy());
                }
            }

            printables.push(PrintableEntry { perms, links, owner, group, size, modified, name });
            entries.push(DirTypes::Entry { path: entry.path(), kind, path_offset: 0 });
        }

        let links = printables.iter().map(|c| c.links.len()).max().unwrap_or(0);
        let owner = printables.iter().map(|c| c.owner.len()).max().unwrap_or(0);
        let group = printables.iter().map(|c| c.group.len()).max().unwrap_or(0);
        let size = printables.iter().map(|c| c.size.len()).max().unwrap_or(0);
        let modified = printables.iter().map(|c| c.modified.len()).max().unwrap_or(0);
        let mut text = String::new();
        for (idx, entry) in printables.iter().enumerate() {
            if idx > 0 {
                text.push('\n');
            }

            let metadata = format!(
                "{} {:>links$} {:<owner$} {:<group$} {:>size$} {:>modified$} ",
                entry.perms, entry.links, entry.owner, entry.group, entry.size, entry.modified,
            );

            entries[idx].path_offset = metadata.len();

            text.push_str(&metadata);
            text.push_str(&entry.name);
        }

        let mut guard = self.state_lock.write();
        let state = &mut *guard;
        let Some(dse) = state.doc_store.get_mut(&state.dir.doc) else {
            debug_panic!();
            return;
        };

        // Temporarily set the mode to not read only for the modifications.
        dse.read_only = false;
        state.dir.entries = entries;

        let view = state.dir.view;
        let doc = state.dir.doc;
        let len = dse.doc.data.len();
        drop(guard);

        let _ = self.action_tx.send(ActionCommand::Remove { view, offset: 0, len });
        let _ = self.action_tx.send(ActionCommand::Insert { view, text });
        let _ = self.action_tx.send(ActionCommand::Saved { doc });
        let _ = self.action_tx.send(ActionCommand::MoveCursorToPos {
            view,
            pos: Pos::new(0, 0),
            move_anchor: true,
        });

        // This is kind of a hack: ideally I wouldn't need to toggle
        // dse.read_only and then need to sync back with the action
        // command. This is just needed to prevent the search mode from
        // replacing the dir's doc content.
        let (tx, rx) = oneshot::channel();
        let _ = self.action_tx.send(ActionCommand::Sync { tx });
        let _ = rx.await;

        let mut guard = self.state_lock.write();
        let state = &mut *guard;
        let Some(dse) = state.doc_store.get_mut(&state.dir.doc) else {
            debug_panic!();
            return;
        };

        // Reset the read only flag after the modifications.
        dse.read_only = true;
        drop(guard);
    }

    fn replace_window(&self, window: WindowId) {
        let mut guard = self.state_lock.write();
        let state = &mut *guard;

        state.workspace.replace_renderer(window, Box::new(DirRenderer::new(state.dir.view)));

        state.index.link_window_to_view(window, state.dir.view);

        let view = state.dir.view;
        drop(guard);

        let _ = self.view_tx.send(ViewCommand::Update { view });
    }

    async fn select(&self) {
        let state = self.state_lock.read();
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, state.dir.view)
        else {
            debug_panic!();
            return;
        };

        let active_window = state.workspace.active_window;

        let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
        let y = dse.doc.data.get_line_of_byte(offset);
        let entries = state.dir.entries.clone();
        drop(state);

        if y >= entries.len() {
            debug_panic!();
            return;
        }

        match entries[y].kind {
            DirTypes::EntryKind::File => {
                let path = entries[y].path.clone();
                let state_lock = self.state_lock.clone();
                let doc_view_tx = self.doc_view_tx.clone();
                let io_tx = self.io_tx.clone();
                let view_tx = self.view_tx.clone();
                tokio::spawn(async move {
                    let doc = util::create_doc(state_lock, Some(path), io_tx).await;
                    let Some(window) = active_window else {
                        debug_panic!();
                        return;
                    };

                    let (tx, rx) = oneshot::channel();
                    let _ = doc_view_tx.send(DocViewCommand::ReplaceWindow {
                        doc,
                        view: None,
                        window,
                        raw: false,
                        tx,
                    });

                    let Ok(view) = rx.await else {
                        debug_panic!();
                        return;
                    };
                    let _ = view_tx.send(ViewCommand::Update { view });
                });
            }
            DirTypes::EntryKind::Dir => {
                let mut state = self.state_lock.write();
                state.dir.pwd = entries[y].path.clone();
                drop(state);

                let _ = std::env::set_current_dir(entries[y].path.clone());

                self.init().await;
            }
            DirTypes::EntryKind::Symlink => {
                let (tx, rx) = oneshot::channel();
                let _ = self.mini_buffer_tx.send(MiniBufferCommand::Message {
                    message: "Symlinks are not (yet) supported".to_string(),
                    tx,
                });

                let mini_buffer_tx = self.mini_buffer_tx.clone();
                tokio::spawn(async move {
                    let Ok(id) = rx.await else { return };

                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                    let _ = mini_buffer_tx.send(MiniBufferCommand::Close { id });
                });
            }
        }
    }
}
