use crate::{
    layers::{CurrLineLayer, CursorLayer},
    protocols::view::ViewRenderer,
    state::State,
    types::{
        Pos,
        render::{Cell, Renderer, Viewport, WindowId},
    },
};

pub struct PickerRenderer {
    input: ViewRenderer,
    list: ViewRenderer,
}

impl PickerRenderer {
    pub fn new() -> Self {
        Self {
            input: ViewRenderer::new(vec![Box::new(CursorLayer {})]),
            list: ViewRenderer::new(vec![Box::new(CurrLineLayer {})]),
        }
    }
}

impl Renderer for PickerRenderer {
    fn render(&self, s: &State, viewport: &mut Viewport, _: WindowId) {
        if viewport.width() < 3 || viewport.height() < 5 {
            return;
        }

        let (input_vse, input_dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.picker.input_view)
                .unwrap();
        let mut input = viewport.sub_view(input_vse.layout(input_dse).content);
        self.input.render(s, &mut input, s.p.picker.input_view);

        let (list_vse, list_dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.picker.list_view)
                .unwrap();
        let mut list = viewport.sub_view(list_vse.layout(list_dse).content);
        if s.p.picker.filtered.is_empty() {
            for y in 0..list.height() {
                for x in 0..list.width() {
                    list.set(Pos::new(x, y), Cell::new(" ".to_string(), 1, s.theme.default));
                }
            }

            for (x, ch) in "No matches".chars().enumerate() {
                list.set(Pos::new(x, 0), Cell::new(ch.to_string(), 1, s.theme.default));
            }
        } else {
            self.list.render(s, &mut list, s.p.picker.list_view);
        }

        self.render_border(s, viewport);
    }
}

impl PickerRenderer {
    fn render_border(&self, s: &State, viewport: &mut Viewport) {
        let width = viewport.width();
        let height = viewport.height();
        let face = s.theme.default;

        for (y, left, right) in [(0, "┏", "┓"), (2, "┣", "┫"), (height - 1, "┗", "┛")] {
            viewport.set(Pos::new(0, y), Cell::new(left.to_string(), 1, face));
            for x in 1..width - 1 {
                viewport.set(Pos::new(x, y), Cell::new("━".to_string(), 1, face));
            }
            viewport.set(Pos::new(width - 1, y), Cell::new(right.to_string(), 1, face));
        }

        for y in 1..height - 1 {
            if y != 2 {
                viewport.set(Pos::new(0, y), Cell::new("┃".to_string(), 1, face));
                viewport.set(Pos::new(width - 1, y), Cell::new("┃".to_string(), 1, face));
            }
        }
    }
}
