use piece_table::Slice;
use tokio::sync::oneshot;

use crate::{
    decorators::MpDecorator,
    protocols::{
        action::ActionProtocol,
        doc_view::DocViewProtocol,
        mp::{
            MpCmd, MpKind,
            pstate::{MpId, PCallback},
        },
    },
    state::{State, ViewStoreTypes},
    types::{Pos, Rect, render::ZLayer},
    util::debug_panic,
};

pub struct MpProtocol {}

impl MpProtocol {
    pub fn exec(s: &mut State, cmd: MpCmd) {
        match cmd {
            MpCmd::Message { message, tx } => {
                Self::message(s, message, tx);
            }
            MpCmd::Prompt { prompt, initial_text, pcallback, tx } => {
                Self::prompt(s, prompt, initial_text, pcallback, tx);
            }
            MpCmd::Submit => Self::submit(s),
            MpCmd::Close { id } => Self::close(s, id),

            MpCmd::Resize { width, height } => Self::resize(s, width, height),
        }
    }

    pub fn message(
        s: &mut State, message: String, tx: Option<oneshot::Sender<MpId>>,
    ) -> Option<MpId> {
        if s.p.mp.kind == MpKind::Prompt {
            return None;
        }

        Self::close(s, s.p.mp.id);

        let (_, window) = DocViewProtocol::create_floating(
            s,
            s.p.mp.doc,
            Some(s.p.mp.view),
            Rect::new(Pos::new(0, s.p.mp.height.saturating_sub(1)), s.p.mp.width, 1),
            ZLayer::Mp,
            true,
            None,
        );

        ActionProtocol::insert_at(s, s.p.mp.view, message, 0);
        ActionProtocol::saved(s, s.p.mp.doc);

        let id = MpId(s.p.mp.next_id);
        s.p.mp.next_id += 1;

        s.p.mp.id = id;
        s.p.mp.kind = MpKind::Message;
        s.p.mp.window = Some(window);
        s.p.mp.prev_window = s.workspace.active_window;

        if let Some(tx) = tx {
            let _ = tx.send(id);
        }

        Some(id)
    }

    pub fn prompt(
        s: &mut State, prompt: String, initial_text: Option<String>, pcallback: PCallback,
        tx: Option<oneshot::Sender<MpId>>,
    ) -> Option<MpId> {
        if s.p.mp.kind == MpKind::Prompt {
            return None;
        }

        Self::close(s, s.p.mp.id);

        ActionProtocol::create_cursor_at_pos(s, s.p.mp.view, Pos::new(0, 0));
        if let Some(text) = initial_text {
            ActionProtocol::insert_at(s, s.p.mp.view, text, 0);
        }

        let (_, window) = DocViewProtocol::create_floating(
            s,
            s.p.mp.doc,
            Some(s.p.mp.view),
            Rect::new(Pos::new(0, s.p.mp.height.saturating_sub(1)), s.p.mp.width, 1),
            ZLayer::Mp,
            true,
            None,
        );

        let id = MpId(s.p.mp.next_id);
        s.p.mp.next_id += 1;

        let Some(vse) = s.view_store.get_mut(&s.p.mp.view) else {
            Self::close(s, s.p.mp.id);

            debug_panic!();

            return None;
        };

        vse.decs.decorators.insert(
            ViewStoreTypes::DecorationId::Mp,
            Box::new(MpDecorator::new(prompt, s.theme.mp)),
        );

        s.p.mp.id = id;
        s.p.mp.kind = MpKind::Prompt;
        s.p.mp.window = Some(window);
        s.p.mp.prev_window = s.workspace.active_window;
        s.p.mp.pcallback = Some(pcallback);

        s.workspace.active_window = Some(window);

        if let Some(tx) = tx {
            let _ = tx.send(id);
        }

        Some(id)
    }

    pub fn submit(s: &mut State) {
        if s.p.mp.kind != MpKind::Prompt {
            return;
        }

        let Some(dse) = s.doc_store.get(&s.p.mp.doc) else {
            Self::close(s, s.p.mp.id);

            debug_panic!();

            return;
        };

        let res = dse.doc.data.slice(0..dse.doc.data.len());

        Self::close(s, s.p.mp.id);

        if let Some(cb) = s.p.mp.pcallback.take() {
            cb(s, res);
        }
    }

    pub fn close(s: &mut State, id: MpId) {
        if s.p.mp.kind == MpKind::None {
            return;
        }
        if s.p.mp.id != id {
            return;
        }

        if let Some(window) = s.p.mp.window.take() {
            s.index.unlink_window(window);
            s.workspace.destroy_window(window);
        }

        s.workspace.active_window = s.p.mp.prev_window;

        s.p.mp.kind = MpKind::None;
        s.p.mp.prev_window = None;

        let Some(vse) = s.view_store.get_mut(&s.p.mp.view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = s.doc_store.get(&s.p.mp.doc) else {
            debug_panic!();
            return;
        };

        vse.decs.decorators.remove(&ViewStoreTypes::DecorationId::Mp);

        let len = dse.doc.data.len();

        ActionProtocol::remove(s, s.p.mp.view, 0, len);
        ActionProtocol::remove_cursors(s, s.p.mp.view, &[0]);
        ActionProtocol::saved(s, s.p.mp.doc);
    }

    pub fn resize(s: &mut State, width: usize, height: usize) {
        s.p.mp.width = width;
        s.p.mp.height = height;

        if s.p.mp.kind == MpKind::None {
            return;
        }

        let Some(window) = s.p.mp.window else {
            debug_panic!();
            return;
        };

        s.workspace.resize_floating(window, width, 1);
    }
}
