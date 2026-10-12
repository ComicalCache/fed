use crate::{
    fed::FCmd,
    protocols::{
        action::ActionProtocol, dir::DirProtocol, doc_view::DocViewProtocol, io::IoProtocol,
        manage::ManageProtocol, mp::MpProtocol, quit::QuitProtocol, screen::ScreenProtocol,
        pstate::PState, view::ViewProtocol,
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
            FCmd::View(cmd) => ViewProtocol::exec(state, cmd),
            FCmd::DocView(cmd) => DocViewProtocol::exec(state, cmd),
            FCmd::Dir(cmd) => DirProtocol::exec(state, pstate, cmd),
            FCmd::Manage(cmd) => ManageProtocol::exec(state, pstate, cmd),
        }
    }
}
