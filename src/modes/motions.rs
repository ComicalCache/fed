use crossterm::event::{KeyCode, KeyModifiers};

use crate::types::{KeyChord, Motion};

pub const MOTIONS: [(KeyChord, Motion); 17] = [
    (KeyChord::new(KeyCode::Char('h'), KeyModifiers::empty()), Motion::Left),
    (KeyChord::new(KeyCode::Char('j'), KeyModifiers::empty()), Motion::Down),
    (KeyChord::new(KeyCode::Char('k'), KeyModifiers::empty()), Motion::Up),
    (KeyChord::new(KeyCode::Char('l'), KeyModifiers::empty()), Motion::Right),
    (KeyChord::new(KeyCode::Char('w'), KeyModifiers::empty()), Motion::NextWord),
    (KeyChord::new(KeyCode::Char('W'), KeyModifiers::SHIFT), Motion::NextWordEnd),
    (KeyChord::new(KeyCode::Char('b'), KeyModifiers::empty()), Motion::PrevWord),
    (KeyChord::new(KeyCode::Char('B'), KeyModifiers::SHIFT), Motion::PrevWordEnd),
    (KeyChord::new(KeyCode::Char('<'), KeyModifiers::empty()), Motion::BeginningOfLine),
    (KeyChord::new(KeyCode::Char('>'), KeyModifiers::empty()), Motion::EndOfLine),
    (KeyChord::new(KeyCode::Char('g'), KeyModifiers::empty()), Motion::EndOfFile),
    (KeyChord::new(KeyCode::Char('G'), KeyModifiers::SHIFT), Motion::BeginningOfFile),
    (KeyChord::new(KeyCode::Char('}'), KeyModifiers::empty()), Motion::NextEmptyLine),
    (KeyChord::new(KeyCode::Char('{'), KeyModifiers::empty()), Motion::PrevEmptyLine),
    (KeyChord::new(KeyCode::Char('s'), KeyModifiers::empty()), Motion::NextWhitespace),
    (KeyChord::new(KeyCode::Char('S'), KeyModifiers::SHIFT), Motion::PrevWhitespace),
    (KeyChord::new(KeyCode::Char('.'), KeyModifiers::empty()), Motion::MatchingOpposite),
];
