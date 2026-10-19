use crate::{
    fed::FCmd,
    protocols::{
        action::ActionProtocol, dir::DirProtocol, doc_view::DocViewProtocol, io::IoProtocol,
        manage::ManageProtocol, mp::MpProtocol, picker::PickerProtocol, quit::QuitProtocol,
        screen::ScreenProtocol, view::ViewProtocol,
    },
    state::State,
    types::render::Screen,
};

pub struct ProtocolRouter {}

impl ProtocolRouter {
    pub fn exec(s: &mut State, screen: &mut Screen, cmd: FCmd) {
        match cmd {
            FCmd::Action(cmd) => ActionProtocol::exec(s, cmd),
            FCmd::Io(cmd) => IoProtocol::exec(s, cmd),
            FCmd::Mp(cmd) => MpProtocol::exec(s, cmd),
            FCmd::Quit => QuitProtocol::exec(s),
            FCmd::Screen(cmd) => ScreenProtocol::exec(s, screen, cmd),
            FCmd::View(cmd) => ViewProtocol::exec(s, cmd),
            FCmd::DocView(cmd) => DocViewProtocol::exec(s, cmd),
            FCmd::Dir(cmd) => DirProtocol::exec(s, cmd),
            FCmd::Manage(cmd) => ManageProtocol::exec(s, cmd),
            FCmd::Picker(cmd) => PickerProtocol::exec(s, cmd),
        }
    }
}
