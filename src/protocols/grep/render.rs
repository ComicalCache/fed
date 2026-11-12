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

pub struct GrepRenderer {
    inner: ViewRenderer,
}

impl GrepRenderer {
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

impl Renderer for GrepRenderer {
    fn render(&self, s: &State, viewport: &mut Viewport, _: WindowId) {
        if viewport.width() == 0 || viewport.height() == 0 {
            return;
        }

        let vse = s.view_store.get(&s.p.grep.view).unwrap();
        let layout =
            vse.layout(s.p.grep.rows.iter().flatten().map(|row| row.line + 1).max().unwrap_or(1));

        let mut gutter = viewport.sub_view(layout.gutter);
        let lines =
            s.p.grep.rows.iter().skip(vse.scroll.y).map(|row| row.as_ref().map(|row| row.line + 1));
        util::render::render_gutter(s, &mut gutter, lines);

        let mut mode_line = viewport.sub_view(layout.mode_line);
        util::render::render_mode_line(s, &mut mode_line, s.p.grep.view);

        let mut content = viewport.sub_view(layout.content);
        self.inner.render(s, &mut content, s.p.grep.view);
    }
}
