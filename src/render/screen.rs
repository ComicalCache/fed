use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
};
use unicode_width::UnicodeWidthStr;

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
        // Redraw everything since resize destroyes the 2D -> 1D mapping.
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

    /// Renders the dirty `Cell`s to the terminal.
    pub fn render(&mut self) {
        let mut stdout = std::io::stdout().lock();

        let mut face = Face::default();
        let mut cursor = Pos::new(usize::MAX, usize::MAX);

        queue!(stdout, SetAttribute(Attribute::Reset)).unwrap();

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (self.width * y) + x;

                if !self.dirty[idx] {
                    continue;
                }

                self.dirty[idx] = false;

                let cell = &self.grid[idx];

                if cell.wide_trailing {
                    continue;
                }

                if cursor != Pos::new(x, y) {
                    queue!(stdout, MoveTo(x as u16, y as u16)).unwrap();
                }

                Self::render_face(&mut stdout, &mut face, &cell.face);

                queue!(stdout, Print(&cell.ch)).unwrap();

                cursor.x = x + cell.ch.width();
                cursor.y = y;
            }
        }

        stdout.flush().unwrap();
    }

    fn render_face(stdout: &mut impl Write, active: &mut Face, target: &Face) {
        if active == target {
            return;
        }

        let mut reset = false;
        if (active.bold == Some(true) && target.bold != Some(true))
            || (active.italic == Some(true) && target.italic != Some(true))
            || (active.underline == Some(true) && target.underline != Some(true))
            || (active.reverse == Some(true) && target.reverse != Some(true))
        {
            reset = true;
        }

        if reset {
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

        if active.reverse != target.reverse && target.reverse == Some(true) {
            queue!(stdout, SetAttribute(Attribute::Reverse)).unwrap();

            active.reverse = target.reverse;
        }
    }
}
