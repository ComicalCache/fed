use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
        SetUnderlineColor,
    },
    terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
};

use crate::types::{Face, Pos, render::Cell};

pub struct Screen {
    width: usize,
    height: usize,

    grid: Vec<Cell>,
    previous: Vec<Cell>,
    dirty: Vec<bool>,
    invalidated: bool,
    output: Vec<u8>,
}

impl Screen {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            grid: vec![Cell::default(); width * height],
            previous: vec![Cell::default(); width * height],
            dirty: vec![false; width * height],
            invalidated: true,
            output: Vec::new(),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        self.grid.resize(width * height, Cell::default());
        self.previous.resize(width * height, Cell::default());
        self.dirty.resize(width * height, false);
        // Redraw everything since resize destroys the 2D -> 1D mapping.
        self.invalidated = true;
    }

    pub fn begin_frame(&mut self, face: Face) {
        self.grid.fill(Cell::new(" ".to_string(), 1, face));
    }

    pub fn get(&self, pos: Pos) -> Option<&Cell> {
        let idx = (self.width * pos.y) + pos.x;

        if idx >= self.grid.len() {
            return None;
        }

        Some(&self.grid[idx])
    }

    pub fn set(&mut self, pos: Pos, cell: Cell) {
        let idx = (self.width * pos.y) + pos.x;

        if idx >= self.grid.len() {
            return;
        }

        self.grid[idx] = cell;
    }

    pub fn render(&mut self) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        self.normalize_wide_cells();

        for ((dirty, cell), previous) in self.dirty.iter_mut().zip(&self.grid).zip(&self.previous) {
            *dirty = self.invalidated || cell != previous;
        }

        if !self.dirty.iter().any(|&dirty| dirty) {
            return;
        }

        // Repaint the leading cells of wide characters, if any of the trailing
        // cells are dirty.
        for y in 0..self.height {
            let offset = self.width * y;
            for grid in [&self.previous, &self.grid] {
                for x in 0..self.width {
                    let width = grid[offset + x].width.min(self.width - x);
                    if width <= 1 {
                        continue;
                    }

                    let span = &mut self.dirty[offset + x..offset + x + width];
                    if span.iter().any(|&dirty| dirty) {
                        span.fill(true);
                    }
                }
            }
        }

        let output = &mut self.output;
        output.clear();
        queue!(output, BeginSynchronizedUpdate, SetAttribute(Attribute::Reset)).unwrap();

        let mut face = Face::default();
        for y in 0..self.height {
            let offset = self.width * y;
            let mut cursor_x = None;
            for x in 0..self.width {
                let cell = &self.grid[offset + x];
                if !self.dirty[offset + x] || cell.width == 0 {
                    continue;
                }

                if cell.width > 1 {
                    queue!(output, MoveTo((x + 1) as u16, y as u16)).unwrap();

                    for trailing in &self.grid[offset + x + 1..offset + x + cell.width] {
                        Self::render_face(output, &mut face, &trailing.face);
                        queue!(output, Print(" ")).unwrap();
                    }

                    cursor_x = None;
                }

                if cursor_x != Some(x) {
                    queue!(output, MoveTo(x as u16, y as u16)).unwrap();
                }

                Self::render_face(output, &mut face, &cell.face);
                queue!(output, Print(&cell.ch)).unwrap();

                // Some characters may not render with the same width as
                // expected by `unicode-width`, thus don't rely on the terminal
                // cursor moving as expected for non-ascii characters.
                cursor_x = cell.ch.is_ascii().then_some(x + cell.width);
            }
        }

        queue!(output, EndSynchronizedUpdate).unwrap();

        let mut stdout = std::io::stdout().lock();
        stdout.write_all(output).unwrap();
        stdout.flush().unwrap();

        self.previous.clone_from(&self.grid);
        self.invalidated = false;
    }

    fn normalize_wide_cells(&mut self) {
        for row in self.grid.chunks_mut(self.width) {
            let mut x = 0;
            while x < row.len() {
                let width = row[x].width;
                // A floating window may cover part of a wide character.
                if width == 0
                    || width > row.len() - x
                    || row[x + 1..x + width].iter().any(|c| c.width != 0)
                {
                    row[x] = Cell::new(" ".to_string(), 1, row[x].face);
                    x += 1;
                } else {
                    x += width;
                }
            }
        }
    }

    fn render_face(stdout: &mut impl Write, active: &mut Face, target: &Face) {
        if active == target {
            return;
        }

        if (active.bold == Some(true) && target.bold != Some(true))
            || (active.italic == Some(true) && target.italic != Some(true))
            || (active.underline == Some(true) && target.underline != Some(true))
            || (active.squiggly == Some(true) && target.squiggly != Some(true))
            || (active.strikethrough == Some(true) && target.strikethrough != Some(true))
            || (active.reverse == Some(true) && target.reverse != Some(true))
        {
            queue!(stdout, SetAttribute(Attribute::Reset)).unwrap();

            *active = Face::default();
        }

        if active.fg != target.fg {
            if let Some(rgb) = target.fg {
                queue!(stdout, SetForegroundColor(rgb.into())).unwrap();
            } else {
                queue!(stdout, SetForegroundColor(Color::Reset)).unwrap();
            }

            active.fg = target.fg;
        }

        if active.bg != target.bg {
            if let Some(rgb) = target.bg {
                queue!(stdout, SetBackgroundColor(rgb.into())).unwrap();
            } else {
                queue!(stdout, SetBackgroundColor(Color::Reset)).unwrap();
            }

            active.bg = target.bg;
        }

        if active.uc != target.uc {
            if let Some(rgb) = target.uc {
                queue!(stdout, SetUnderlineColor(rgb.into())).unwrap();
            } else {
                queue!(stdout, SetUnderlineColor(Color::Reset)).unwrap();
            }

            active.uc = target.uc;
        }

        if active.bold != target.bold && target.bold == Some(true) {
            queue!(stdout, SetAttribute(Attribute::Bold)).unwrap();

            active.bold = target.bold;
        }
        if active.italic != target.italic && target.italic == Some(true) {
            queue!(stdout, SetAttribute(Attribute::Italic)).unwrap();

            active.italic = target.italic;
        }
        if active.underline != target.underline && target.underline == Some(true) {
            queue!(stdout, SetAttribute(Attribute::Underlined)).unwrap();

            active.underline = target.underline;
        }
        if active.squiggly != target.squiggly && target.squiggly == Some(true) {
            queue!(stdout, SetAttribute(Attribute::Undercurled)).unwrap();

            active.squiggly = target.squiggly;
        }
        if active.strikethrough != target.strikethrough && target.strikethrough == Some(true) {
            queue!(stdout, SetAttribute(Attribute::CrossedOut)).unwrap();

            active.strikethrough = target.strikethrough;
        }

        if active.reverse != target.reverse && target.reverse == Some(true) {
            queue!(stdout, SetAttribute(Attribute::Reverse)).unwrap();

            active.reverse = target.reverse;
        }
    }
}
