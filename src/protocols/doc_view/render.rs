use crate::layers::CursorLayer;
use crate::layers::RulerLayer;
use crate::layers::SearchLayer;
use crate::layers::SelectionLayer;
use crate::protocols::view::ViewRenderer;
use crate::state::State;
use crate::state::ViewId;
use crate::types::render::Renderer;
use crate::types::render::Viewport;
use crate::types::render::WindowId;
use crate::util;

pub struct DocViewRenderer {
    inner: ViewRenderer,

    view: ViewId,
    raw: bool,
}

impl DocViewRenderer {
    pub fn new(view: ViewId, raw: bool) -> Self {
        if raw {
            Self {
                inner: ViewRenderer::new(vec![Box::new(SearchLayer {}), Box::new(CursorLayer {})]),
                view,
                raw,
            }
        } else {
            Self {
                inner: ViewRenderer::new(vec![
                    Box::new(RulerLayer {}),
                    Box::new(SearchLayer {}),
                    Box::new(SelectionLayer {}),
                    Box::new(CursorLayer {}),
                ]),
                view,
                raw,
            }
        }
    }
}

impl Renderer for DocViewRenderer {
    fn render(&self, s: &State, viewport: &mut Viewport, _: WindowId) {
        let width = viewport.width();
        let height = viewport.height();

        if height == 0 || width == 0 {
            return;
        }

        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, self.view).unwrap();

        let layout = vse.layout(dse.doc.data.line_count());

        if !self.raw {
            let mut gutter = viewport.sub_view(layout.gutter);
            let lines = (vse.scroll.y..dse.doc.data.line_count()).map(|line| Some(line + 1));
            util::render::render_gutter(s, &mut gutter, lines);

            let mut mode_line = viewport.sub_view(layout.mode_line);
            util::render::render_mode_line(s, &mut mode_line, self.view);
        }

        let mut content = viewport.sub_view(layout.content);
        self.inner.render(s, &mut content, self.view);
    }
}
