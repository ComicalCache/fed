use crate::{
    fed::FCmd,
    protocols::{
        action::ActionProtocol, dir::DirProtocol, doc_view::DocViewProtocol, io::IoProtocol,
        mp::MpProtocol, quit::QuitProtocol, screen::ScreenProtocol, state::PState,
        view::ViewProtocol,
    },
    state::State,
};

pub struct ProtocolRouter {}

impl ProtocolRouter {
    pub fn exec(state: &mut State, pstate: &mut PState, cmd: FCmd) {
        match cmd {
            FCmd::Action(cmd) => ActionProtocol::exec(state, cmd),
            FCmd::Io(cmd) => IoProtocol::exec(pstate, cmd),
            FCmd::Mp(cmd) => MpProtocol::exec(state, pstate, cmd),
            FCmd::Quit => QuitProtocol::exec(state, pstate),
            FCmd::Screen(cmd) => ScreenProtocol::exec(state, pstate, cmd),
            FCmd::View(cmd) => ViewProtocol::exec(state, pstate, cmd),
            FCmd::DocView(cmd) => DocViewProtocol::exec(state, pstate, cmd),
            FCmd::Dir(cmd) => DirProtocol::exec(state, pstate, cmd),
        }
    }
}
