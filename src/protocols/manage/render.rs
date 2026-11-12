use crate::layers::CurrLineLayer;
use crate::layers::CursorLayer;
use crate::layers::SearchLayer;
use crate::layers::SelectionLayer;
use crate::protocols::view::ViewRenderer;
use crate::state::State;
use crate::types::render::Renderer;
use crate::types::render::Viewport;
use crate::types::render::WindowId;
use crate::util;

pub struct ManageRenderer {
    inner: ViewRenderer,
}

impl ManageRenderer {
    pub fn new() -> Self {
        Self {
            inner: ViewRenderer::new(vec![
                Box::new(CurrLineLayer {}),
                Box::new(SearchLayer {}),
                Box::new(SelectionLayer {}),
                Box::new(CursorLayer {}),
            ]),
        }
    }
}

impl Renderer for ManageRenderer {
    fn render(&self, s: &State, viewport: &mut Viewport, _: WindowId) {
        let width = viewport.width();
        let height = viewport.height();

        if height == 0 || width == 0 {
            return;
        }

        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.manage.view).unwrap();

        let layout = vse.layout(dse.doc.data.line_count());

        let mut gutter = viewport.sub_view(layout.gutter);
        let lines = (vse.scroll.y..dse.doc.data.line_count()).map(|line| Some(line + 1));
        util::render::render_gutter(s, &mut gutter, lines);

        let mut mode_line = viewport.sub_view(layout.mode_line);
        util::render::render_mode_line(s, &mut mode_line, s.p.manage.view);

        let mut content = viewport.sub_view(layout.content);
        self.inner.render(s, &mut content, s.p.manage.view);
    }
}
