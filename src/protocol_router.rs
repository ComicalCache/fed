use crate::fed::FCmd;
use crate::protocols::action::ActionProtocol;
use crate::protocols::dir::DirProtocol;
use crate::protocols::doc_view::DocViewProtocol;
use crate::protocols::io::IoProtocol;
use crate::protocols::manage::ManageProtocol;
use crate::protocols::mp::MpProtocol;
use crate::protocols::picker::PickerProtocol;
use crate::protocols::quit::QuitProtocol;
use crate::protocols::screen::ScreenProtocol;
use crate::protocols::search::SearchProtocol;
use crate::protocols::view::ViewProtocol;
use crate::state::State;
use crate::types::render::Screen;

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
            FCmd::Search(cmd) => SearchProtocol::exec(s, cmd),
        }
    }
}
