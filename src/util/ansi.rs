use anstyle_parse::{Params, Perform};

use crate::{
    decorators::SpanDecorator,
    types::{Decoration, Face, Rgb, Span},
};

#[derive(Default)]
pub struct DecoratioParser {
    pub parsed: String,
    spans: Vec<Span<Decoration>>,

    face: Face,
    face_offset: usize,
}

impl DecoratioParser {
    pub fn commit(&mut self) {
        let pos = self.parsed.len();

        if self.face_offset < pos {
            self.spans.push(Span {
                start: self.face_offset,
                end: pos,
                data: Decoration::Style { face: self.face },
            });
        }

        self.face_offset = pos;
    }

    pub fn decorator(self) -> SpanDecorator { SpanDecorator::new(self.spans) }
}

impl Perform for DecoratioParser {
    fn print(&mut self, ch: char) { self.parsed.push(ch); }

    fn execute(&mut self, byte: u8) {
        let ch = byte as char;
        // FIXME: ignores everything but \n.
        if ch == '\n' {
            self.parsed.push(ch);
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _: &[u8], _: bool, action: u8) {
        let action = action as char;

        if action == 'm' {
            self.commit();

            let mut iter = params.iter();
            while let Some(param) = iter.next() {
                let code = param.first().copied().unwrap_or(0);

                match code {
                    0 => self.face = Face::default(),

                    1 => self.face.bold = Some(true),
                    3 => self.face.italic = Some(true),
                    4 => {
                        if param.get(1) == Some(&3) {
                            self.face.squiggly = Some(true);
                        } else {
                            self.face.underline = Some(true);
                        }
                    }
                    7 => self.face.reverse = Some(true),
                    9 => self.face.strikethrough = Some(true),

                    22 => self.face.bold = Some(false),
                    23 => self.face.italic = Some(false),
                    24 => {
                        self.face.underline = Some(false);
                        self.face.squiggly = Some(false);
                    }
                    27 => self.face.reverse = Some(false),
                    29 => self.face.strikethrough = Some(false),

                    30..=37 => self.face.fg = Some(ansi_to_rgb(code - 30)),
                    90..=97 => self.face.fg = Some(ansi_to_rgb(code - 90 + 8)),

                    38 => {
                        if let Some(mode) = iter.next().and_then(|p| p.first().copied())
                            && mode == 2
                        {
                            let r = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            let g = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            let b = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            self.face.fg = Some(Rgb { r, g, b });
                        }
                    }
                    39 => self.face.fg = None,

                    40..=47 => self.face.bg = Some(ansi_to_rgb(code - 40)),
                    100..=107 => self.face.bg = Some(ansi_to_rgb(code - 100 + 8)),

                    48 => {
                        if let Some(mode) = iter.next().and_then(|p| p.first().copied())
                            && mode == 2
                        {
                            let r = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            let g = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            let b = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            self.face.bg = Some(Rgb { r, g, b });
                        }
                    }
                    49 => self.face.bg = None,

                    58 => {
                        if let Some(mode) = iter.next().and_then(|p| p.first().copied())
                            && mode == 2
                        {
                            let r = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            let g = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            let b = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                            self.face.uc = Some(Rgb { r, g, b });
                        }
                    }
                    59 => self.face.uc = None,

                    _ => {}
                }
            }
        }
    }
}

const fn ansi_to_rgb(index: u16) -> Rgb {
    match index {
        // Standard.
        0 => Rgb { r: 41, g: 44, b: 51 },    // Black.
        1 => Rgb { r: 224, g: 108, b: 117 }, // Red.
        2 => Rgb { r: 152, g: 195, b: 121 }, // Green.
        3 => Rgb { r: 229, g: 192, b: 123 }, // Yellow.
        4 => Rgb { r: 97, g: 175, b: 239 },  // Blue.
        5 => Rgb { r: 198, g: 120, b: 221 }, // Magenta.
        6 => Rgb { r: 86, g: 182, b: 194 },  // Cyan.
        7 => Rgb { r: 172, g: 178, b: 190 }, // White.

        // High-intensity.
        8 => Rgb { r: 92, g: 99, b: 112 },    // Bright Black.
        9 => Rgb { r: 235, g: 130, b: 138 },  // Bright Red.
        10 => Rgb { r: 170, g: 207, b: 142 }, // Bright Green.
        11 => Rgb { r: 240, g: 206, b: 145 }, // Bright Yellow.
        12 => Rgb { r: 120, g: 188, b: 242 }, // Bright Blue.
        13 => Rgb { r: 210, g: 142, b: 230 }, // Bright Magenta.
        14 => Rgb { r: 108, g: 194, b: 205 }, // Bright Cyan.
        15 => Rgb { r: 200, g: 206, b: 218 }, // Bright White.

        _ => Rgb { r: 255, g: 255, b: 255 },
    }
}
