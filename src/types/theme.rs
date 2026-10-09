use crate::types::{Face, Rgb};

#[derive(Clone)]
pub struct Theme {
    pub default: Face,
    pub cursor: Face,
    pub active_cursor: Face,
    pub gutter: Face,
    pub mode_line: Face,
    pub ruler: Face,

    pub mp: Face,
    pub selection: Face,
    pub search_match: Face,

    pub dir_header: Face,
    pub dir_symlink: Face,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            default: Face {
                fg: Some(Rgb::new(172, 178, 190)),
                bg: Some(Rgb::new(41, 44, 51)),
                ..Face::default()
            },
            cursor: Face {
                fg: Some(Rgb::new(41, 44, 51)),
                bg: Some(Rgb::new(172, 178, 190)),
                ..Face::default()
            },
            active_cursor: Face {
                fg: Some(Rgb::new(41, 44, 51)),
                bg: Some(Rgb::new(254, 120, 25)),
                ..Face::default()
            },
            selection: Face {
                fg: Some(Rgb::new(41, 44, 51)),
                bg: Some(Rgb::new(97, 175, 239)),
                ..Face::default()
            },
            gutter: Face {
                fg: Some(Rgb::new(101, 103, 105)),
                bg: Some(Rgb::new(36, 40, 46)),
                ..Face::default()
            },
            mode_line: Face {
                fg: Some(Rgb::new(172, 178, 190)),
                bg: Some(Rgb::new(59, 61, 66)),
                ..Face::default()
            },
            ruler: Face { bg: Some(Rgb::new(59, 61, 66)), ..Face::default() },
            search_match: Face {
                fg: Some(Rgb::new(41, 44, 51)),
                bg: Some(Rgb::new(229, 192, 123)),
                ..Face::default()
            },
            mp: Face {
                fg: Some(Rgb::new(172, 178, 190)),
                bg: Some(Rgb::new(59, 61, 66)),
                ..Face::default()
            },
            dir_header: Face { bold: Some(true), ..Face::default() },
            dir_symlink: Face { fg: Some(Rgb::new(173, 111, 24)), ..Face::default() },
        }
    }
}
