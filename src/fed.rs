use std::{path::PathBuf, time::Duration};

use anstyle_parse::{DefaultCharAccumulator, Parser};
use crossterm::event::Event;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    oneshot,
};

use crate::{
    cli::Cli,
    input_router::InputRouter,
    modes,
    protocol_router::ProtocolRouter,
    protocols::{
        action::ActionCmd,
        dir::DirCmd,
        doc_view::DocViewCmd,
        io::{IoCmd, IoProtocol},
        manage::ManageCmd,
        mp::MpCmd,
        pstate::PState,
        quit::QuitProtocol,
        screen::{ScreenCmd, ScreenProtocol},
        view::{ViewCmd, ViewProtocol},
    },
    state::{DocId, DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{
        Pos, Rect, RectSplit,
        render::{WindowId, Workspace},
    },
    util::{self, debug_panic},
};

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
}

#[allow(unused)]
pub enum FEvent {
    Doc(DocStoreTypes::Event),
    View(ViewStoreTypes::Event),
}

pub struct Fed {
    input_router: InputRouter,

    state: State,
    pstate: PState,

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
        let mut state = State::new(
            Workspace::new(Rect::new(Pos::default(), width, height)),
            fevent_tx,
            async_fcmd_tx.clone(),
        );
        let pstate = PState::new(width, height);

        let doc = state.create_doc(path).unwrap();

        let fed =
            Self { input_router, state, pstate, input_rx, shutdown_tx, fevent_rx, async_fcmd_rx };

        (fed, async_fcmd_tx, doc)
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(event) = self.input_rx.recv() => {
                    for cmd in self.input_router.exec(&mut self.state, event) {
                        ProtocolRouter::exec(&mut self.state, &mut self.pstate, cmd);
                    }
                }
                Some(cmd) = self.async_fcmd_rx.recv() => {
                    ProtocolRouter::exec(&mut self.state, &mut self.pstate, cmd);

                    while let Ok(cmd) = self.async_fcmd_rx.try_recv() {
                        ProtocolRouter::exec(&mut self.state, &mut self.pstate, cmd);
                    }
                }
                else => break,
            }

            while let Ok(event) = self.fevent_rx.try_recv() {
                match event {
                    FEvent::Doc(event) => {
                        modes::mp::setup::exec_doc_event(&mut self.state, &event);
                        modes::dir::setup::exec_doc_event(&mut self.state, &event);
                        modes::manage::setup::exec_doc_event(&mut self.state, &event);
                        modes::read::setup::exec_doc_event(&mut self.state, &event);
                        ViewProtocol::exec_doc_event(&mut self.state, &event);
                    }
                    FEvent::View(event) => {
                        modes::mp::setup::exec_view_event(&mut self.state, &event);
                        modes::dir::setup::exec_view_event(&mut self.state, &event);
                        modes::manage::setup::exec_view_event(&mut self.state, &event);
                        modes::read::setup::exec_view_event(&mut self.state, &event);
                        ViewProtocol::exec_view_event(&mut self.state, &event);
                    }
                }
            }

            if QuitProtocol::can_quit(&self.pstate) {
                let _ = self.shutdown_tx.send(());
                break;
            }

            ScreenProtocol::render(&self.state, &mut self.pstate);
        }
    }

    pub async fn stage1(tx: UnboundedSender<FCmd>, doc: DocId, cli: Cli) {
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
        let Ok(Some((view, window))) = vw_rx.await else {
            debug_panic!();
            return;
        };

        let _ = tx.send(FCmd::Action(ActionCmd::SetActiveWindow { window: Some(window) }));

        if dir {
            let _ = std::env::set_current_dir(cli.path.clone().unwrap());

            let _ = tx.send(FCmd::Dir(DirCmd::Init));
            let _ = tx.send(FCmd::Dir(DirCmd::ReplaceWindow { window }));
        }
        if let Some(text) = cli.piped.clone() {
            let mut decoration_parser = util::ansi::DecoratioParser::default();
            let mut parser = Parser::<DefaultCharAccumulator>::new();

            for byte in text.as_bytes() {
                parser.advance(&mut decoration_parser, *byte);
            }
            decoration_parser.commit();

            let text = decoration_parser.parsed.clone();
            let dec = Box::new(decoration_parser.decorator());

            let _ = tx.send(FCmd::Action(ActionCmd::SetDocDecorator {
                doc,
                id: DocStoreTypes::DecorationId::Span,
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
