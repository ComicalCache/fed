use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use fancy_regex::Regex;
use ignore::WalkBuilder;

use crate::protocols::grep::pstate::GrepFile;
use crate::util;

pub struct GrepScan {
    pub root: PathBuf,

    pub files: Vec<GrepFile>,
    pub errors: Vec<String>,
}

impl GrepScan {
    pub fn new(
        root: PathBuf, query: &str, opened_docs: HashMap<PathBuf, String>, revision: u64,
        curr_revision: &AtomicU64,
    ) -> Result<Self, String> {
        let is_stale = || curr_revision.load(Ordering::Relaxed) != revision;

        if is_stale() {
            return Err("Search cancelled".to_string());
        }

        let regex = Regex::new(query).map_err(|err| format!("Invalid regex: {err}"))?;
        let root = util::path::normalize(root);
        let mut opened_docs: HashMap<_, _> = opened_docs
            .into_iter()
            .map(|(path, text)| (util::path::normalize(path), text))
            .collect();

        let mut files = Vec::new();
        let mut errors = Vec::new();

        let walker = WalkBuilder::new(&root)
            .hidden(false)
            .require_git(false)
            .follow_links(false)
            .filter_entry(|entry| entry.file_name() != ".git")
            .build();
        for entry in walker {
            if is_stale() {
                return Err("Search cancelled".to_string());
            }

            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    errors.push(err.to_string());
                    continue;
                }
            };

            if let Some(err) = entry.error() {
                errors.push(err.to_string());
            }

            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }

            let path = util::path::normalize(entry.path());
            let text = if let Some(text) = opened_docs.remove(&path) {
                text
            } else {
                let bytes = match fs::read(&path) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        errors.push(format!("{}: {err}", path.display()));
                        continue;
                    }
                };

                let Ok(text) = String::from_utf8(bytes) else { continue };

                text
            };

            if text.contains('\0') {
                continue;
            }

            if is_stale() {
                return Err("Search cancelled".to_string());
            }

            match GrepFile::new(path, text, &regex) {
                Ok(file) if !file.matches.is_empty() => files.push(file),
                Ok(_) => {}
                Err(err) => errors.push(err),
            }
        }

        files.sort_unstable_by(|a, b| a.path.cmp(&b.path));

        if is_stale() {
            return Err("Search cancelled".to_string());
        }

        Ok(GrepScan { root, files, errors })
    }
}
