use std::path::PathBuf;

use fancy_regex::Regex;

use crate::state::ViewStoreTypes::SearchMatch;

#[derive(Clone)]
pub struct GrepFile {
    pub path: PathBuf,
    pub text: String,

    pub matches: Vec<SearchMatch>,
}

impl GrepFile {
    pub fn new(path: PathBuf, text: String, regex: &Regex) -> Result<Self, String> {
        let matches = regex
            .find_iter(&text)
            .map(|result| {
                result.map(|m| SearchMatch { start: m.start(), end: m.end(), replacement: None })
            })
            .collect::<Result<_, _>>()
            .map_err(|err| format!("{}: {err}", path.display()))?;
        Ok(GrepFile { path, text, matches })
    }
}
