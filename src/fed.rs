use std::path::PathBuf;
use std::time::Duration;

use anstyle_parse::DefaultCharAccumulator;
use anstyle_parse::Parser;
use crossterm::event::Event;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::oneshot;

use crate::cli::Cli;
use crate::input_router::InputRouter;
use crate::protocol_router::ProtocolRouter;
use crate::protocols::action::ActionCmd;
use crate::protocols::dir::DirCmd;
use crate::protocols::doc_view::DocViewCmd;
use crate::protocols::io::IoCmd;
use crate::protocols::io::IoProtocol;
use crate::protocols::manage::ManageCmd;
use crate::protocols::manage::ManageProtocol;
use crate::protocols::mp::MpCmd;
use crate::protocols::picker::PickerCmd;
use crate::protocols::screen::ScreenCmd;
use crate::protocols::screen::ScreenProtocol;
use crate::protocols::search::SearchCmd;
use crate::protocols::search::SearchProtocol;
use crate::protocols::view::ViewCmd;
use crate::protocols::view::ViewProtocol;
use crate::state::DocId;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::Pos;
use crate::types::RectSplit;
use crate::types::render::Screen;
use crate::types::render::WindowId;
use crate::util;

#[allow(unused)]
pub enum FCmd {
    Action(ActionCmd),
    Io(IoCmd),
    Mp(MpCmd),
    Quit,
    Screen(ScreenCmd),
    View(ViewCmd),
    DocView(DocViewCmd),
    Dir(DirCmd),
    Manage(ManageCmd),
    Picker(PickerCmd),
    Search(SearchCmd),
}

/// FEvent's are deferred and may only update component owned state. They should
/// NOT do initialization. The FEvent targets may be destroyed before the event
/// has been delivered.
#[allow(unused)]
pub enum FEvent {
    Doc(DocStoreTypes::Event),
    View(ViewStoreTypes::Event),
}

pub struct Fed {
    input_router: InputRouter,

    screen: Screen,
    s: State,

    input_rx: UnboundedReceiver<Event>,
    shutdown_tx: UnboundedSender<()>,

    fevent_rx: UnboundedReceiver<FEvent>,
    async_fcmd_rx: UnboundedReceiver<FCmd>,
}

impl Fed {
    pub fn new(
        width: usize, height: usize, path: Option<PathBuf>, input_rx: UnboundedReceiver<Event>,
        shutdown_tx: UnboundedSender<()>,
    ) -> (Self, UnboundedSender<FCmd>, DocId) {
        let (fevent_tx, fevent_rx) = unbounded_channel();
        let (async_fcmd_tx, async_fcmd_rx) = unbounded_channel();

        let input_router = InputRouter::default();
        let screen = Screen::new(width, height);
        let mut s = State::new(width, height, fevent_tx, async_fcmd_tx.clone());

        let doc = s.create_doc(path.filter(|p| !p.is_dir())).unwrap();

        let fed = Self { input_router, screen, s, input_rx, shutdown_tx, fevent_rx, async_fcmd_rx };

        (fed, async_fcmd_tx, doc)
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(event) = self.input_rx.recv() => {
                    let cmds = self.input_router.exec(&self.s, event);
                    if cmds.is_empty() {
                        continue;
                    }

                    for cmd in cmds {
                        ProtocolRouter::exec(&mut self.s, &mut self.screen, cmd);
                        self.process_events();
                    }
                }
                Some(cmd) = self.async_fcmd_rx.recv() => {
                    ProtocolRouter::exec(&mut self.s, &mut self.screen, cmd);
                    self.process_events();

                    while let Ok(cmd) = self.async_fcmd_rx.try_recv() {
                        ProtocolRouter::exec(&mut self.s, &mut self.screen, cmd);
                        self.process_events();
                    }
                }
                else => break,
            }

            if self.s.p.quit.quit {
                let _ = self.shutdown_tx.send(());
                break;
            }

            ScreenProtocol::render(&self.s, &mut self.screen);
        }
    }

    fn process_events(&mut self) {
        while let Ok(event) = self.fevent_rx.try_recv() {
            match event {
                FEvent::Doc(event) => {
                    ManageProtocol::exec_doc_event(&mut self.s, &event);
                    ViewProtocol::exec_doc_event(&mut self.s, &event);
                }
                FEvent::View(event) => {
                    SearchProtocol::exec_view_event(&mut self.s, &event);
                    ViewProtocol::exec_view_event(&mut self.s, &event);
                }
            }
        }
    }

    pub async fn stage1(tx: UnboundedSender<FCmd>, doc: DocId, cli: Cli) {
        if let Ok(path) = std::env::current_dir() {
            let _ = tx.send(FCmd::Io(IoCmd::Scan { path }));
        }

        let dir = cli.path.clone().is_some_and(|p| p.is_dir());

        let (vw_tx, vw_rx) = oneshot::channel();
        let _ = tx.send(FCmd::DocView(DocViewCmd::CreateTile {
            doc,
            view: None,
            // This is kind of a hack: since no other windows exist, we can pass anything
            // because a new root window will be created in any case.
            split_window: WindowId(0),
            direction: RectSplit::Vertical,
            raw: false,
            tx: Some(vw_tx),
        }));
        let (view, window) = vw_rx.await.unwrap();

        let _ = tx.send(FCmd::Action(ActionCmd::SetActiveWindow { window: Some(window) }));

        if dir {
            let _ = tx.send(FCmd::Dir(DirCmd::Show { window, path: cli.path.clone() }));
        } else if let Some(text) = cli.piped.clone() {
            let mut decoration_parser = util::ansi::DecorationParser::default();
            let mut parser = Parser::<DefaultCharAccumulator>::new();

            for byte in text.as_bytes() {
                parser.advance(&mut decoration_parser, *byte);
            }
            decoration_parser.commit();

            let text = decoration_parser.parsed.clone();
            let dec = Box::new(decoration_parser.decorator());

            let _ = tx.send(FCmd::Action(ActionCmd::SetDocDecorator {
                doc,
                decoration: DocStoreTypes::DecorationId::Span,
                dec,
            }));
            let _ = tx.send(FCmd::Action(ActionCmd::Insert { view, text }));
            let _ = tx.send(FCmd::Action(ActionCmd::Saved { doc }));
            let _ = tx.send(FCmd::Action(ActionCmd::MoveCursorToPos {
                view,
                pos: Pos::new(0, 0),
                move_anchor: true,
            }));

            Self::apply_modes(tx, doc, view, cli);
        } else {
            let _ = tx.send(FCmd::Dir(DirCmd::Init));

            if let Some(path) = cli.path.clone() {
                IoProtocol::read(
                    path,
                    Box::new(move |res| {
                        Box::pin(async move { Self::stage2(tx, view, doc, cli, res) })
                    }),
                );
            } else {
                Self::apply_modes(tx, doc, view, cli);
            }
        }
    }

    fn stage2(
        tx: UnboundedSender<FCmd>, view: ViewId, doc: DocId, cli: Cli, res: Result<String, String>,
    ) {
        let text = match res {
            Ok(text) => text,
            Err(err) => {
                util::mp::async_message(
                    tx,
                    format!("Error reading file: {err}"),
                    Duration::from_secs(3),
                );

                return;
            }
        };

        let _ = tx.send(FCmd::Action(ActionCmd::Insert { view, text }));
        let _ = tx.send(FCmd::Action(ActionCmd::Saved { doc }));
        let _ = tx.send(FCmd::Action(ActionCmd::MoveCursorToPos {
            view,
            pos: Pos::new(0, 0),
            move_anchor: true,
        }));

        Self::apply_modes(tx, doc, view, cli);
    }

    fn apply_modes(tx: UnboundedSender<FCmd>, doc: DocId, view: ViewId, cli: Cli) {
        if let Some(mode) = &cli.doc_mode {
            let mode = match mode.to_lowercase().as_str() {
                "read" => DocStoreTypes::Mode::Read,
                "mp" => DocStoreTypes::Mode::Mp,
                "dir" => DocStoreTypes::Mode::Dir,
                "manage" => DocStoreTypes::Mode::Manage,
                _ => DocStoreTypes::Mode::Text,
            };

            let _ = tx.send(FCmd::Action(ActionCmd::SetDocMode { doc, mode }));
        }

        if let Some(mode) = &cli.view_mode {
            let mode = match mode.to_lowercase().as_str() {
                "insert" => ViewStoreTypes::Mode::Insert,
                "visual" => ViewStoreTypes::Mode::Visual,
                "search" => ViewStoreTypes::Mode::Search,
                _ => ViewStoreTypes::Mode::Normal,
            };

            let _ = tx.send(FCmd::Action(ActionCmd::PushViewMode { view, mode }));
        }
    }
}
