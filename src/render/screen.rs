use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
        SetUnderlineColor,
    },
};

use crate::{
    render::Cell,
    types::{Face, Pos},
};

pub struct Screen {
    width: usize,
    height: usize,

    grid: Vec<Cell>,
    /// `Cell`s that need to be redrawn.
    dirty: Vec<bool>,
}

impl Screen {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            grid: vec![Cell::default(); width * height],
            dirty: vec![true; width * height],
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        self.grid.resize(width * height, Cell::default());
        self.dirty.resize(width * height, true);
        // Redraw everything since resize destroys the 2D -> 1D mapping.
        self.dirty.fill(true);
    }

    pub fn get(&mut self, pos: Pos) -> Option<&Cell> {
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

        if self.grid[idx] == cell {
            return;
        }

        self.grid[idx] = cell;
        self.dirty[idx] = true;
    }

    pub fn render(&mut self) {
        // Mark the leading cells of wide characters as dirty, if the trailing
        // cell is dirty.
        for y in 0..self.height {
            for x in (1..self.width).rev() {
                let idx = (self.width * y) + x;
                if self.dirty[idx] && self.grid[idx].width == 0 {
                    self.dirty[idx - 1] = true;
                }
            }
        }

        let mut stdout = std::io::stdout().lock();
        queue!(stdout, SetAttribute(Attribute::Reset)).unwrap();

        let mut face = Face::default();
        for y in 0..self.height {
            let offset = self.width * y;

            let line_dirty = self.dirty[offset..offset + self.width].iter().any(|&d| d);
            if !line_dirty {
                continue;
            }

            queue!(stdout, MoveTo(0, y as u16)).unwrap();
            for x in 0..self.width {
                let cell = &self.grid[offset + x];

                Self::render_face(&mut stdout, &mut face, &cell.face);
                queue!(stdout, Print(" ")).unwrap();
            }

            let mut cursor_x = self.width;
            for x in 0..self.width {
                self.dirty[offset + x] = false;
                let cell = &self.grid[offset + x];

                if cell.width == 0 || cell.ch == " " {
                    continue;
                }

                if cursor_x != x {
                    queue!(stdout, MoveTo(x as u16, y as u16)).unwrap();
                }

                Self::render_face(&mut stdout, &mut face, &cell.face);
                queue!(stdout, Print(&cell.ch)).unwrap();

                // Some characters may not render with the same width as
                // expected by `unicode-width`, thus don't rely on the terminal
                // cursor moving as expected for non-ascii characters.
                if cell.ch.is_ascii() {
                    cursor_x = x + cell.width;
                }
            }
        }

        stdout.flush().unwrap();
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
