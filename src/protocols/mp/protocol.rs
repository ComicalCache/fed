use piece_table::Slice;
use tokio::sync::oneshot;

use crate::{
    debug_panic::debug_panic,
    decorators::MpDecorator,
    protocols::{
        action::ActionProtocol,
        doc_view::DocViewProtocol,
        mp::{MpCmd, state::PCallback},
        state::PState,
    },
    render::ZLayer,
    state::{MpId, MpTypes, State, ViewStoreTypes},
    types::{Pos, Rect},
};

pub struct MpProtocol {}

impl MpProtocol {
    pub fn exec(state: &mut State, pstate: &mut PState, cmd: MpCmd) {
        match cmd {
            MpCmd::Message { message, tx } => {
                Self::message(state, pstate, message, tx);
            }
            MpCmd::Prompt { prompt, initial_text, pcallback, tx } => {
                Self::prompt(state, pstate, prompt, initial_text, pcallback, tx);
            }
            MpCmd::Submit => Self::submit(state, pstate),
            MpCmd::Close { id } => Self::close(state, id),
            MpCmd::Resize { width, height } => Self::resize(state, pstate, width, height),
        }
    }

    pub fn message(
        state: &mut State, pstate: &mut PState, message: String, tx: Option<oneshot::Sender<MpId>>,
    ) -> Option<MpId> {
        if state.mp.kind == MpTypes::Kind::Prompt {
            return None;
        }

        Self::close(state, state.mp.id);

        let (_, window) = DocViewProtocol::create_floating(
            state,
            pstate,
            state.mp.doc,
            Some(state.mp.view),
            Rect::new(Pos::new(0, pstate.mp.height.saturating_sub(1)), pstate.mp.width, 1),
            ZLayer::MiniBuffer,
            true,
            None,
        );

        ActionProtocol::insert_at(state, state.mp.view, message, 0);
        ActionProtocol::saved(state, state.mp.doc);

        let id = MpId(pstate.mp.next_id);
        pstate.mp.next_id += 1;

        state.mp.id = id;
        state.mp.kind = MpTypes::Kind::Message;
        state.mp.window = Some(window);
        state.mp.prev_window = state.workspace.active_window;

        if let Some(tx) = tx {
            let _ = tx.send(id);
        }

        Some(id)
    }

    pub fn prompt(
        state: &mut State, pstate: &mut PState, prompt: String, initial_text: Option<String>,
        pcallback: PCallback, tx: Option<oneshot::Sender<MpId>>,
    ) -> Option<MpId> {
        if state.mp.kind == MpTypes::Kind::Prompt {
            return None;
        }

        Self::close(state, state.mp.id);

        ActionProtocol::create_cursor_at_pos(state, state.mp.view, Pos::new(0, 0));
        if let Some(text) = initial_text {
            ActionProtocol::insert_at(state, state.mp.view, text, 0);
        }

        let (_, window) = DocViewProtocol::create_floating(
            state,
            pstate,
            state.mp.doc,
            Some(state.mp.view),
            Rect::new(Pos::new(0, pstate.mp.height.saturating_sub(1)), pstate.mp.width, 1),
            ZLayer::MiniBuffer,
            true,
            None,
        );

        let id = MpId(pstate.mp.next_id);
        pstate.mp.next_id += 1;

        let Some(vse) = state.view_store.get_mut(&state.mp.view) else {
            state.workspace.destroy_window(window);

            debug_panic!();

            return None;
        };

        vse.decs.decorators.insert(
            ViewStoreTypes::DecorationId::MiniBuffer,
            Box::new(MpDecorator::new(prompt, state.theme.mp)),
        );

        state.mp.id = id;
        state.mp.kind = MpTypes::Kind::Prompt;
        state.mp.window = Some(window);
        state.mp.prev_window = state.workspace.active_window;

        state.workspace.active_window = Some(window);

        pstate.mp.pcallback = Some(pcallback);

        if let Some(tx) = tx {
            let _ = tx.send(id);
        }

        Some(id)
    }

    pub fn submit(state: &mut State, pstate: &mut PState) {
        if state.mp.kind != MpTypes::Kind::Prompt {
            return;
        }

        let Some(dse) = state.doc_store.get(&state.mp.doc) else {
            debug_panic!();
            return;
        };

        let res = dse.doc.data.slice(0..dse.doc.data.len());

        Self::close(state, state.mp.id);

        if let Some(cb) = pstate.mp.pcallback.take() {
            cb(state, res);
        }
    }

    pub fn close(state: &mut State, id: MpId) {
        if state.mp.kind == MpTypes::Kind::None {
            return;
        }
        if state.mp.id != id {
            return;
        }

        if let Some(window) = state.mp.window.take() {
            state.index.unlink_window(window);
            state.workspace.destroy_window(window);
        };

        state.workspace.active_window = state.mp.prev_window;

        state.mp.kind = MpTypes::Kind::None;
        state.mp.prev_window = None;

        let Some(vse) = state.view_store.get_mut(&state.mp.view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&state.mp.doc) else {
            debug_panic!();
            return;
        };

        vse.decs.decorators.remove(&ViewStoreTypes::DecorationId::MiniBuffer);

        let len = dse.doc.data.len();
        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();

        ActionProtocol::remove_cursors(state, state.mp.view, offsets);
        ActionProtocol::remove(state, state.mp.view, 0, len);
        ActionProtocol::saved(state, state.mp.doc);
    }

    pub fn resize(state: &mut State, pstate: &mut PState, width: usize, height: usize) {
        pstate.mp.width = width;
        pstate.mp.height = height;

        if state.mp.kind == MpTypes::Kind::None {
            return;
        }

        let Some(window) = state.mp.window else {
            debug_panic!();
            return;
        };

        state.workspace.resize_floating(window, width, 1);
    }
}
