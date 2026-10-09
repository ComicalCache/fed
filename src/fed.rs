use std::{path::PathBuf, time::Duration};

use crossterm::event::Event;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    oneshot,
};

use crate::{
    debug_panic::debug_panic,
    input_router::InputRouter,
    protocol_router::ProtocolRouter,
    protocols::{
        action::ActionCmd,
        dir::DirCmd,
        doc_view::DocViewCmd,
        io::{IoCmd, IoProtocol, IoState},
        mp::{MpCmd, MpState},
        quit::{QuitProtocol, QuitState},
        screen::{ScreenCmd, ScreenProtocol, ScreenState},
        state::PState,
        view::{ViewCmd, ViewProtocol},
    },
    render::{WindowId, Workspace},
    state::{DocId, DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{Pos, Rect, RectSplit},
};

pub enum FCmd {
    Action(ActionCmd),
    Io(IoCmd),
    Mp(MpCmd),
    Quit,
    Screen(ScreenCmd),
    View(ViewCmd),
    DocView(DocViewCmd),
    Dir(DirCmd),
}

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
    fcmd_rx: UnboundedReceiver<FCmd>,
    async_fcmd_rx: UnboundedReceiver<FCmd>,
}

impl Fed {
    pub fn new(
        width: usize, height: usize, path: Option<PathBuf>, input_rx: UnboundedReceiver<Event>,
        shutdown_tx: UnboundedSender<()>,
    ) -> (Self, UnboundedSender<FCmd>, DocId) {
        let (fevent_tx, fevent_rx) = unbounded_channel();
        let (fcmd_tx, fcmd_rx) = unbounded_channel();
        let (async_fcmd_tx, async_fcmd_rx) = unbounded_channel();

        let mut state = State::new(
            Workspace::new(Rect::new(Pos::default(), width, height)),
            fevent_tx.clone(),
            fcmd_tx.clone(),
            async_fcmd_tx.clone(),
        );

        let pstate = PState {
            io: IoState::default(),
            mp: MpState::new(width, height),
            quit: QuitState::default(),
            screen: ScreenState::new(width, height),
        };

        let doc = state.create_doc(path);

        let fed = Self {
            input_router: InputRouter::default(),
            state,
            pstate: pstate,
            input_rx,
            shutdown_tx,
            fevent_rx: fevent_rx,
            fcmd_rx: fcmd_rx,
            async_fcmd_rx: async_fcmd_rx,
        };

        (fed, async_fcmd_tx, doc)
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(event) = self.input_rx.recv() => {
                    for cmd in self.input_router.exec(&mut self.state, event) {
                        let _ = self.state.fcmd_tx.send(cmd);
                    }
                }
                Some(cmd) = self.async_fcmd_rx.recv() => {
                    let _ = self.state.fcmd_tx.send(cmd);

                    while let Ok(cmd) = self.async_fcmd_rx.try_recv() {
                        let _ = self.state.fcmd_tx.send(cmd);
                    }
                }
                else => break,
            }

            while let Ok(cmd) = self.fcmd_rx.try_recv() {
                ProtocolRouter::exec(&mut self.state, &mut self.pstate, cmd);
            }

            while let Ok(event) = self.fevent_rx.try_recv() {
                match event {
                    FEvent::Doc(event) => {
                        ViewProtocol::exec_doc_event(&mut self.state, &mut self.pstate, event);
                    }
                    FEvent::View(event) => {
                        ViewProtocol::exec_view_event(&mut self.state, &mut self.pstate, event);
                    }
                }
            }

            if QuitProtocol::can_quit(&self.pstate) {
                let _ = self.shutdown_tx.send(());
                break;
            }

            ScreenProtocol::do_render(&mut self.state, &mut self.pstate);
        }
    }

    pub async fn stage1(tx: UnboundedSender<FCmd>, doc: DocId, path: Option<PathBuf>, dir: bool) {
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
            let _ = std::env::set_current_dir(path.unwrap());

            let _ = tx.send(FCmd::Dir(DirCmd::Init));
            let _ = tx.send(FCmd::Dir(DirCmd::ReplaceWindow { window }));
        } else {
            let _ = tx.send(FCmd::Dir(DirCmd::Init));

            let Some(path) = path.clone() else {
                return;
            };

            IoProtocol::read(path, Box::new(move |res| Box::pin(Self::stage2(tx, view, doc, res))));
        }
    }

    async fn stage2(
        tx: UnboundedSender<FCmd>, view: ViewId, doc: DocId, res: Result<String, String>,
    ) {
        let text = match res {
            Ok(text) => text,
            Err(err) => {
                let (id_tx, id_rx) = oneshot::channel();
                let _ = tx.send(FCmd::Mp(MpCmd::Message {
                    message: format!("Error reading file: {err}"),
                    tx: Some(id_tx),
                }));

                tokio::spawn(async move {
                    let Ok(id) = id_rx.await else {
                        debug_panic!();
                        return;
                    };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = tx.send(FCmd::Mp(MpCmd::Close { id }));
                });

                return;
            }
        };

        let _ = tx.send(FCmd::Action(ActionCmd::Insert { view, text }));
        let _ = tx.send(FCmd::Action(ActionCmd::MoveCursorToPos {
            view,
            pos: Pos::new(0, 0),
            move_anchor: true,
        }));
        let _ = tx.send(FCmd::Action(ActionCmd::Saved { doc }));
    }
}
