use std::{path::PathBuf, time::Duration};

use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    fed::FCmd,
    protocols::{
        action::{ActionCmd, ActionProtocol},
        dir::{DirCmd, DirEntry, DirEntryKind, DirRenderer, PrintableDirEntry},
        doc_view::{DocViewCmd, DocViewProtocol},
        io::{IoFuture, IoProtocol},
        view::ViewProtocol,
    },
    state::{DocId, State, ViewId},
    types::{Pos, render::WindowId},
    util::{self, debug_panic, format},
};

pub struct DirProtocol {}

impl DirProtocol {
    pub fn exec(s: &mut State, cmd: DirCmd) {
        match cmd {
            DirCmd::Init => Self::init(s),
            DirCmd::InitCompletion { entries, printables } => {
                Self::init_completion(s, entries, printables);
            }
            DirCmd::ReplaceWindow { window } => Self::replace_window(s, window),

            DirCmd::Create { path, dir } => Self::create(s, path, dir),
            DirCmd::Rename { old, new } => Self::rename(s, old, new),
            DirCmd::Delete { path, recursive } => Self::delete(s, path, recursive),

            DirCmd::Select => Self::select(s),
        }
    }

    pub fn init(s: &mut State) {
        s.p.dir.pwd = std::env::current_dir()
            .and_then(|p| p.canonicalize())
            .unwrap_or_else(|_| s.p.dir.pwd.clone());

        tokio::spawn(Self::fetch_entries(s.p.dir.pwd.clone(), s.async_fcmd_tx.clone()));
    }

    pub fn init_completion(
        s: &mut State, mut entries: Vec<DirEntry>, printables: Vec<PrintableDirEntry>,
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

        s.p.dir.entries = entries;

        let len = if let Some(dse) = s.doc_store.get_mut(&s.p.dir.doc) {
            // Temporarily set the mode to not read only for the modifications.
            dse.read_only = false;

            dse.doc.data.len()
        } else {
            debug_panic!();
            return;
        };

        ActionProtocol::remove(s, s.p.dir.view, 0, len);
        ActionProtocol::insert(s, s.p.dir.view, text);
        ActionProtocol::saved(s, s.p.dir.doc);
        ActionProtocol::move_cursor_to_pos(s, s.p.dir.view, Pos::new(0, 1), true);

        if let Some(dse) = s.doc_store.get_mut(&s.p.dir.doc) {
            // Reset the read only flag after the modifications.
            dse.read_only = true;
        } else {
            debug_panic!();
        }
    }

    pub fn replace_window(s: &mut State, window: WindowId) {
        Self::init(s);

        s.workspace.replace_renderer(window, Box::new(DirRenderer::new()));
        s.index.link_window_to_view(window, s.p.dir.view);

        ViewProtocol::update(s, s.p.dir.view);
    }

    pub fn create(s: &State, path: PathBuf, dir: bool) {
        let path = util::path::normalize(path);

        let async_fcmd_tx = s.async_fcmd_tx.clone();
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
                util::mp::async_message(
                    async_fcmd_tx,
                    format!("Failed to create path: {err}"),
                    Duration::from_secs(3),
                );
            } else {
                let _ = async_fcmd_tx.send(FCmd::Dir(DirCmd::Init));
            }
        });
    }

    pub fn rename(s: &State, old: PathBuf, new: PathBuf) {
        let async_fcmd_tx = s.async_fcmd_tx.clone();
        tokio::spawn(async move {
            if let Err(err) = tokio::fs::rename(&old, &new).await {
                util::mp::async_message(
                    async_fcmd_tx,
                    format!("Failed to rename path: {err}"),
                    Duration::from_secs(3),
                );
            } else {
                let _ = async_fcmd_tx.send(FCmd::Dir(DirCmd::Init));
            }
        });
    }

    pub fn delete(s: &State, path: PathBuf, recursive: bool) {
        let async_fcmd_tx = s.async_fcmd_tx.clone();
        tokio::spawn(async move {
            let res = if recursive {
                tokio::fs::remove_dir_all(&path).await
            } else if let Ok(meta) = tokio::fs::symlink_metadata(&path).await {
                if meta.is_dir() {
                    tokio::fs::remove_dir(&path).await
                } else {
                    tokio::fs::remove_file(&path).await
                }
            } else {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Path not found"))
            };

            if let Err(err) = res {
                util::mp::async_message(
                    async_fcmd_tx,
                    format!("Failed to delete path: {err}"),
                    Duration::from_secs(3),
                );
            } else {
                let _ = async_fcmd_tx.send(FCmd::Dir(DirCmd::Init));
            }
        });
    }

    pub fn select(s: &mut State) {
        let Some((vse, dse)) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.dir.view)
        else {
            debug_panic!();
            return;
        };

        let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
        let y = dse.doc.data.get_line_of_byte(offset);

        if y >= s.p.dir.entries.len() {
            debug_panic!();
            return;
        }

        let entry = s.p.dir.entries[y].clone();
        match entry.kind {
            DirEntryKind::File => {
                let Some(window) = s.workspace.active_window else {
                    debug_panic!();
                    return;
                };

                let doc = match s.create_doc(Some(entry.path.clone())) {
                    Ok(doc) => doc,
                    Err(doc) => {
                        let view =
                            s.index.doc_to_views(doc).and_then(|vs| vs.iter().next()).copied();

                        if let Some(view) = view
                            && let Some(windows) = s.index.view_to_windows(view)
                            && let Some(&window) = windows.iter().next()
                        {
                            s.workspace.active_window = Some(window);
                            return;
                        }

                        DocViewProtocol::replace_window(s, doc, view, window, false, None);

                        return;
                    }
                };

                let view = s.index.doc_to_views(doc).and_then(|vs| vs.iter().next()).copied();

                let async_fcmd_tx = s.async_fcmd_tx.clone();
                IoProtocol::read(
                    entry.path,
                    Box::new(move |res| -> IoFuture {
                        Box::pin(Self::select_file(async_fcmd_tx, window, view, doc, res))
                    }),
                );
            }
            DirEntryKind::Dir => {
                s.p.dir.pwd = s.p.dir.entries[y].path.clone();

                let _ = std::env::set_current_dir(s.p.dir.entries[y].path.clone());

                Self::init(s);
            }
            DirEntryKind::Symlink => {
                util::mp::message(
                    s,
                    "Symlinks are not (yet) supported".to_string(),
                    Duration::from_secs(3),
                );
            }
            DirEntryKind::Header => {}
        }
    }

    async fn fetch_entries(pwd: PathBuf, async_fcmd_tx: UnboundedSender<FCmd>) {
        let mut entries = Vec::new();
        let mut printables: Vec<PrintableDirEntry> = Vec::new();

        printables.push(PrintableDirEntry {
            perms: "PERMISSIONS".to_string(),
            links: "LINKS".to_string(),
            owner: "OWNER".to_string(),
            group: "GROUP".to_string(),
            size: "SIZE".to_string(),
            modified: "MODIFIED".to_string(),
            name: "NAME".to_string(),
        });
        entries.push(DirEntry { path: pwd.clone(), kind: DirEntryKind::Header, path_offset: 0 });

        if let Some(parent) = pwd.parent()
            && let Ok(metadata) = tokio::fs::metadata(parent).await
        {
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

            printables.push(PrintableDirEntry {
                perms: format!("d{perms}"),
                links,
                owner,
                group,
                size: format::size(metadata.len()),
                modified: metadata.modified().map_or_else(
                    |_| "n/a".to_string(),
                    |time| {
                        let datetime: chrono::DateTime<chrono::Local> = time.into();
                        datetime.format("%b %e %H:%M").to_string()
                    },
                ),
                name: "../".to_string(),
            });
            entries.push(DirEntry {
                path: parent.to_path_buf(),
                kind: DirEntryKind::Dir,
                path_offset: 0,
            });
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
                DirEntryKind::Dir
            } else if metadata.is_symlink() {
                DirEntryKind::Symlink
            } else {
                DirEntryKind::File
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
                DirEntryKind::File => format!("-{perms}"),
                DirEntryKind::Dir => format!("d{perms}"),
                DirEntryKind::Symlink => format!("l{perms}"),
                DirEntryKind::Header => unreachable!(),
            };
            let size = format::size(metadata.len());

            let modified = metadata.modified().map_or_else(
                |_| "n/a".to_string(),
                |time| {
                    let datetime: chrono::DateTime<chrono::Local> = time.into();
                    datetime.format("%b %e %H:%M").to_string()
                },
            );

            let mut name = entry.file_name().display().to_string();
            if matches!(kind, DirEntryKind::Dir) {
                name.push(std::path::MAIN_SEPARATOR);
            } else if matches!(kind, DirEntryKind::Symlink)
                && let Ok(target) = tokio::fs::read_link(entry.path()).await
            {
                name.push_str(" > ");
                name.push_str(&target.display().to_string());
            }

            printables.push(PrintableDirEntry { perms, links, owner, group, size, modified, name });
            entries.push(DirEntry { path: entry.path(), kind, path_offset: 0 });
        }

        let _ = async_fcmd_tx.send(FCmd::Dir(DirCmd::InitCompletion { entries, printables }));
    }

    async fn select_file(
        async_fcmd_tx: UnboundedSender<FCmd>, window: WindowId, view: Option<ViewId>, doc: DocId,
        res: Result<String, String>,
    ) {
        let text = match res {
            Ok(text) => text,
            Err(err) => {
                util::mp::async_message(
                    async_fcmd_tx,
                    format!("Error reading file: {err}"),
                    Duration::from_secs(3),
                );

                return;
            }
        };

        let (tx, rx) = oneshot::channel();
        let _ = async_fcmd_tx.send(FCmd::DocView(DocViewCmd::ReplaceWindow {
            doc,
            view,
            window,
            raw: false,
            tx: Some(tx),
        }));

        let Ok(view) = rx.await else {
            debug_panic!();
            return;
        };

        let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::Insert { view, text }));
        let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::MoveCursorToPos {
            view,
            pos: Pos::new(0, 0),
            move_anchor: true,
        }));
        let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::Saved { doc }));
    }
}
