use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{KeyInputHandler, priorities::KeyInputPriority},
    protocols::action::ActionCommand,
    state::{DocumentId, DocumentStoreTypes::Document, State, ViewStoreTypes},
    types::Direction,
};

pub struct NormalKeyInput {
    state: State,

    action_tx: UnboundedSender<ActionCommand>,
    quit_tx: UnboundedSender<()>,
}

impl NormalKeyInput {
    pub fn new(
        state: State, action_tx: UnboundedSender<ActionCommand>, quit_tx: UnboundedSender<()>,
    ) -> Self {
        Self { state, action_tx, quit_tx }
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let Some(view) = self
            .state
            .with_workspace(|w| w.active_window)
            .and_then(|window| self.state.with_window_view_map(|wv| wv.get(&window).cloned())?)
        else {
            return false;
        };

        if self.state.with_view(view, |vm| vm.get::<ViewStoreTypes::Mode>().cloned()).flatten()
            != Some(ViewStoreTypes::Mode::Normal)
        {
            return false;
        }

        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('q') {
            let _ = self.quit_tx.send(());

            return true;
        }

        match event.code {
            KeyCode::Char('h') => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Left });
            }
            KeyCode::Char('j') => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Down });
            }
            KeyCode::Char('k') => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Up });
            }
            KeyCode::Char('l') => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Right });
            }
            KeyCode::Char('i') => {
                self.state
                    .with_view(view, |vm| Some(vm.get::<DocumentId>().cloned()?))
                    .flatten()
                    .and_then(|d| {
                        self.state.with_doc_mut(d, |d| {
                            d.get_mut::<Document>().and_then(|d| {
                                d.data.start_commit();

                                Some(())
                            })
                        })
                    });

                self.state.with_view_mut(view, |vm| {
                    if let Some(m) = vm.get_mut::<ViewStoreTypes::Mode>() {
                        *m = ViewStoreTypes::Mode::Insert;
                    }
                });
            }
            _ => return false,
        };

        true
    }
}
