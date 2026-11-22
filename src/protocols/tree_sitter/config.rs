use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;

use tree_sitter_highlight::HighlightConfiguration;

use crate::types::SyntaxHighlight;

pub(super) fn get(path: &Path) -> Option<Result<Arc<HighlightConfiguration>, String>> {
    static ZIG: OnceLock<Result<Arc<HighlightConfiguration>, String>> = OnceLock::new();
    static RUST: OnceLock<Result<Arc<HighlightConfiguration>, String>> = OnceLock::new();
    static C: OnceLock<Result<Arc<HighlightConfiguration>, String>> = OnceLock::new();
    static HTML: OnceLock<Result<Arc<HighlightConfiguration>, String>> = OnceLock::new();
    static JAVASCRIPT: OnceLock<Result<Arc<HighlightConfiguration>, String>> = OnceLock::new();
    static CSS: OnceLock<Result<Arc<HighlightConfiguration>, String>> = OnceLock::new();

    match path.extension()?.to_str()? {
        "zig" => Some(ZIG.get_or_init(|| zig().map(Arc::new)).clone()),
        "rs" => Some(RUST.get_or_init(|| rust().map(Arc::new)).clone()),
        "c" | "h" => Some(C.get_or_init(|| c().map(Arc::new)).clone()),
        "html" => Some(HTML.get_or_init(|| html().map(Arc::new)).clone()),
        "js" => Some(JAVASCRIPT.get_or_init(|| javascript().map(Arc::new)).clone()),
        "css" => Some(CSS.get_or_init(|| css().map(Arc::new)).clone()),
        _ => None,
    }
}

fn zig() -> Result<HighlightConfiguration, String> {
    // FIX: upstream tree-sitter-zig targets lua which is not supported by
    // tree-sitter-highlighter.
    let query = self::normalize(
        tree_sitter_zig::HIGHLIGHTS_QUERY
            .replace("#lua-match?", "#match?")
            .replace("@comment @spell", "@comment")
            .as_str(),
    );

    let mut config = HighlightConfiguration::new(
        tree_sitter_zig::LANGUAGE.into(),
        "zig",
        &query,
        tree_sitter_zig::INJECTIONS_QUERY,
        "",
    )
    .map_err(|err| err.to_string())?;
    config.configure(&SyntaxHighlight::ALL.map(SyntaxHighlight::name));

    Ok(config)
}

fn rust() -> Result<HighlightConfiguration, String> {
    // FIX: upstream tree-sitter-rust doesn't show variables.
    let query = self::normalize(&format!(
        "(identifier) @variable\n{}",
        tree_sitter_rust::HIGHLIGHTS_QUERY,
    ));

    let mut config = HighlightConfiguration::new(
        tree_sitter_rust::LANGUAGE.into(),
        "rust",
        &query,
        tree_sitter_rust::INJECTIONS_QUERY,
        "",
    )
    .map_err(|err| err.to_string())?;
    config.configure(&SyntaxHighlight::ALL.map(SyntaxHighlight::name));

    Ok(config)
}

fn c() -> Result<HighlightConfiguration, String> {
    let query = self::normalize(tree_sitter_c::HIGHLIGHT_QUERY);

    let mut config =
        HighlightConfiguration::new(tree_sitter_c::LANGUAGE.into(), "c", &query, "", "")
            .map_err(|err| err.to_string())?;
    config.configure(&SyntaxHighlight::ALL.map(SyntaxHighlight::name));

    Ok(config)
}

fn html() -> Result<HighlightConfiguration, String> {
    let query = self::normalize(
        tree_sitter_html::HIGHLIGHTS_QUERY
            .replace("@tag", "@type")
            .as_str()
    );

    let mut config = HighlightConfiguration::new(
        tree_sitter_html::LANGUAGE.into(),
        "html",
        &query,
        tree_sitter_html::INJECTIONS_QUERY,
        ""
    )
    .map_err(|err| err.to_string())?;
    config.configure(&SyntaxHighlight::ALL.map(SyntaxHighlight::name));

    Ok(config)
}

fn javascript() -> Result<HighlightConfiguration, String> {
    let query = self::normalize(tree_sitter_javascript::HIGHLIGHT_QUERY);

    let mut config = HighlightConfiguration::new(
        tree_sitter_javascript::LANGUAGE.into(),
        "javascript",
        &query,
        tree_sitter_javascript::INJECTIONS_QUERY,
        tree_sitter_javascript::LOCALS_QUERY
    )
    .map_err(|err| err.to_string())?;
    config.configure(&SyntaxHighlight::ALL.map(SyntaxHighlight::name));

    Ok(config)
}

fn css() -> Result<HighlightConfiguration, String> {
    let query = self::normalize(tree_sitter_css::HIGHLIGHTS_QUERY);

    let mut config = HighlightConfiguration::new(
        tree_sitter_css::LANGUAGE.into(),
        "css",
        &query,
        "",
        ""
    )
    .map_err(|err| err.to_string())?;
    config.configure(&SyntaxHighlight::ALL.map(SyntaxHighlight::name));

    Ok(config)
}

fn normalize(query: &str) -> String {
    query
        .replace("@property", "@variable.member")
        .replace("@delimiter", "@punctuation.delimiter")
        .replace("@constructor", "@type")
        .replace("@escape", "@string.escape")
        .replace("@attribute", "@keyword")
}
