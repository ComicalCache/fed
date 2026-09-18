use crate::types::Rgb;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Face {
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
    pub uc: Option<Rgb>,

    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub squiggly: Option<bool>,
    pub strikethrough: Option<bool>,

    pub reverse: Option<bool>,
}

impl Face {
    pub fn merge(&mut self, other: Self) {
        if let Some(fg) = other.fg {
            self.fg = Some(fg);
        }
        if let Some(bg) = other.bg {
            self.bg = Some(bg);
        }
        if let Some(uc) = other.uc {
            self.uc = Some(uc);
        }

        if let Some(bold) = other.bold {
            self.bold = Some(bold);
        }
        if let Some(italic) = other.italic {
            self.italic = Some(italic);
        }
        if let Some(underline) = other.underline {
            self.underline = Some(underline);
        }
        if let Some(squiggly) = other.squiggly {
            self.squiggly = Some(squiggly);
        }
        if let Some(strikethrough) = other.strikethrough {
            self.strikethrough = Some(strikethrough);
        }

        if let Some(reverse) = other.reverse {
            self.reverse = Some(reverse);
        }
    }
}
