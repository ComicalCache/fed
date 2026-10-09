use std::{path::PathBuf, time::Duration};

use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    protocols::{
        action::{ActionCmd, ActionProtocol},
        dir::{DirCmd, DirRenderer},
        doc_view::DocViewCmd,
        io::{IoFuture, IoProtocol},
        mp::{MpCmd, MpProtocol},
        state::PState,
        view::{ViewCmd, ViewProtocol},
    },
    render::WindowId,
    state::{
        DirTypes::{self, Entry, PrintableEntry},
        DocId, State,
    },
    types::Pos,
    util::{self, format},
};

pub struct DirProtocol {}

impl DirProtocol {
    pub fn exec(state: &mut State, pstate: &mut PState, cmd: DirCmd) {
        match cmd {
            DirCmd::Init => Self::init(state),
            DirCmd::InitCompletion { entries, printables } => {
                Self::init_completion(state, entries, printables)
            }
            DirCmd::ReplaceWindow { window } => Self::replace_window(state, pstate, window),
            DirCmd::Create { path, dir } => Self::create(state, path, dir),
            DirCmd::Rename { old, new } => Self::rename(state, old, new),
            DirCmd::Delete { path, recursive } => Self::delete(state, path, recursive),
            DirCmd::Select => Self::select(state, pstate),
        }
    }

    fn init(state: &mut State) {
        state.dir.pwd = std::env::current_dir()
            .map(|p| p.canonicalize())
            .flatten()
            .unwrap_or_else(|_| state.dir.pwd.clone());

        tokio::spawn(Self::fetch_entries(state.dir.pwd.clone(), state.async_fcmd_tx.clone()));
    }

    fn init_completion(
        state: &mut State, mut entries: Vec<Entry>, printables: Vec<PrintableEntry>,
    ) {
        let perms = printables.iter().map(|c| c.perms.len()).max().unwrap_or(0);
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
                "{:>perms$} {:>links$} {:<owner$} {:<group$} {:>size$} {:<modified$}  ",
                entry.perms, entry.links, entry.owner, entry.group, entry.size, entry.modified,
            );

            entries[idx].path_offset = metadata.len();

            text.push_str(&metadata);
            text.push_str(&entry.name);
        }

        state.dir.entries = entries;

        let len = if let Some(dse) = state.doc_store.get_mut(&state.dir.doc) {
            // Temporarily set the mode to not read only for the modifications.
            dse.read_only = false;

            dse.doc.data.len()
        } else {
            debug_panic!();
            return;
        };

        ActionProtocol::remove(state, state.dir.view, 0, len);
        ActionProtocol::insert(state, state.dir.view, text);
        ActionProtocol::saved(state, state.dir.doc);
        ActionProtocol::move_cursor_to_pos(state, state.dir.view, Pos::new(0, 1), true);

        if let Some(dse) = state.doc_store.get_mut(&state.dir.doc) {
            // Reset the read only flag after the modifications.
            dse.read_only = true;
        } else {
            debug_panic!();
            return;
        };
    }

    fn replace_window(state: &mut State, pstate: &mut PState, window: WindowId) {
        state.workspace.replace_renderer(window, Box::new(DirRenderer::new(state.dir.view)));
        state.index.link_window_to_view(window, state.dir.view);

        ViewProtocol::update(state, pstate, state.dir.view);
    }

    fn create(state: &mut State, path: PathBuf, dir: bool) {
        let path = util::path::normalize(path);

        let async_fcmd_tx = state.async_fcmd_tx.clone();
        tokio::spawn(async move {
            let res = if dir {
                tokio::fs::create_dir_all(&path).await
            } else {
                async {
                    if let Some(parent) = path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }

                    tokio::fs::File::create(&path).await?;

                    Ok(())
                }
                .await
            };

            if let Err(err) = res {
                let (tx, rx) = oneshot::channel();
                let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Message {
                    message: format!("Failed to create path: {err}"),
                    tx: Some(tx),
                }));

                tokio::spawn(async move {
                    let Ok(id) = rx.await else {
                        debug_panic!();
                        return;
                    };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Close { id }));
                });
            } else {
                let _ = async_fcmd_tx.send(FCmd::Dir(DirCmd::Init));
            }
        });
    }

    fn rename(state: &mut State, old: PathBuf, new: PathBuf) {
        let async_fcmd_tx = state.async_fcmd_tx.clone();
        tokio::spawn(async move {
            if let Err(err) = tokio::fs::rename(&old, &new).await {
                let (tx, rx) = oneshot::channel();
                let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Message {
                    message: format!("Failed to rename path: {err}"),
                    tx: Some(tx),
                }));

                tokio::spawn(async move {
                    let Ok(id) = rx.await else {
                        debug_panic!();
                        return;
                    };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Close { id }));
                });
            } else {
                let _ = async_fcmd_tx.send(FCmd::Dir(DirCmd::Init));
            }
        });
    }

    fn delete(state: &mut State, path: PathBuf, recursive: bool) {
        let async_fcmd_tx = state.async_fcmd_tx.clone();
        tokio::spawn(async move {
            let res = if recursive {
                tokio::fs::remove_dir_all(&path).await
            } else {
                if let Ok(meta) = tokio::fs::symlink_metadata(&path).await {
                    if meta.is_dir() {
                        tokio::fs::remove_dir(&path).await
                    } else {
                        tokio::fs::remove_file(&path).await
                    }
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Path not found"))
                }
            };

            if let Err(err) = res {
                let (tx, rx) = oneshot::channel();
                let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Message {
                    message: format!("Failed to delete path: {err}"),
                    tx: Some(tx),
                }));

                tokio::spawn(async move {
                    let Ok(id) = rx.await else {
                        debug_panic!();
                        return;
                    };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Close { id }));
                });
            } else {
                let _ = async_fcmd_tx.send(FCmd::Dir(DirCmd::Init));
            }
        });
    }

    fn select(state: &mut State, pstate: &mut PState) {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, state.dir.view)
        else {
            debug_panic!();
            return;
        };

        let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
        let y = dse.doc.data.get_line_of_byte(offset);

        if y >= state.dir.entries.len() {
            debug_panic!();
            return;
        }

        let entry = state.dir.entries[y].clone();
        match entry.kind {
            DirTypes::EntryKind::File => {
                let Some(window) = state.workspace.active_window else {
                    debug_panic!();
                    return;
                };

                let doc = state.create_doc(Some(entry.path.clone()));

                let async_fcmd_tx = state.async_fcmd_tx.clone();
                IoProtocol::read(
                    entry.path,
                    Box::new(move |res| -> IoFuture {
                        Box::pin(Self::select_file(async_fcmd_tx, res, doc, window))
                    }),
                );
            }
            DirTypes::EntryKind::Dir => {
                state.dir.pwd = state.dir.entries[y].path.clone();

                let _ = std::env::set_current_dir(state.dir.entries[y].path.clone());

                Self::init(state);
            }
            DirTypes::EntryKind::Symlink => {
                let Some(id) = MpProtocol::message(
                    state,
                    pstate,
                    "Symlinks are not (yet) supported".to_string(),
                    None,
                ) else {
                    debug_panic!();
                    return;
                };

                let async_fcmd_tx = state.async_fcmd_tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Close { id }));
                });
            }
            DirTypes::EntryKind::Header => {}
        }
    }

    async fn fetch_entries(pwd: PathBuf, async_fcmd_tx: UnboundedSender<FCmd>) {
        let mut entries = Vec::new();
        let mut printables: Vec<PrintableEntry> = Vec::new();

        printables.push(PrintableEntry {
            perms: "PERMISSIONS".to_string(),
            links: "LINKS".to_string(),
            owner: "OWNER".to_string(),
            group: "GROUP".to_string(),
            size: "SIZE".to_string(),
            modified: "MODIFIED".to_string(),
            name: "NAME".to_string(),
        });
        entries.push(DirTypes::Entry {
            path: pwd.clone(),
            kind: DirTypes::EntryKind::Header,
            path_offset: 0,
        });

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
                DirTypes::EntryKind::Header => unreachable!(),
            };
            let size = format::size(metadata.len());

            let modified = if let Ok(time) = metadata.modified() {
                let datetime: chrono::DateTime<chrono::Local> = time.into();
                datetime.format("%b %e %H:%M").to_string()
            } else {
                "n/a".to_string()
            };

            let mut name = entry.file_name().display().to_string();
            if matches!(kind, DirTypes::EntryKind::Dir) {
                name.push(std::path::MAIN_SEPARATOR);
            } else if matches!(kind, DirTypes::EntryKind::Symlink) {
                if let Ok(target) = tokio::fs::read_link(entry.path()).await {
                    name.push_str(" > ");
                    name.push_str(&target.display().to_string());
                }
            }

            printables.push(PrintableEntry { perms, links, owner, group, size, modified, name });
            entries.push(DirTypes::Entry { path: entry.path(), kind, path_offset: 0 });
        }

        let _ = async_fcmd_tx.send(FCmd::Dir(DirCmd::InitCompletion { entries, printables }));
    }

    async fn select_file(
        async_fcmd_tx: UnboundedSender<FCmd>, res: Result<String, String>, doc: DocId,
        window: WindowId,
    ) {
        let text = match res {
            Ok(text) => text,
            Err(err) => {
                let (tx, rx) = oneshot::channel();
                let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Message {
                    message: format!("Error reading file: {err}"),
                    tx: Some(tx),
                }));

                tokio::spawn(async move {
                    let Ok(id) = rx.await else {
                        debug_panic!();
                        return;
                    };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Close { id }));
                });

                return;
            }
        };

        let (tx, rx) = oneshot::channel();
        let _ = async_fcmd_tx.send(FCmd::DocView(DocViewCmd::ReplaceWindow {
            doc,
            view: None,
            window,
            raw: false,
            tx: Some(tx),
        }));

        let Ok(view) = rx.await else {
            debug_panic!();
            return;
        };

        let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::Insert { view, text }));
        let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::Saved { doc }));
        let _ = async_fcmd_tx.send(FCmd::View(ViewCmd::Update { view }));
    }
}
