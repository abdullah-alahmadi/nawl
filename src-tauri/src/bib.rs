//! Reading BibTeX for the editor: entry list for the citation picker and the
//! on-screen citation labels. (The PDF uses real BibTeX, run by the TeX engine.)

use std::collections::HashSet;

use biblatex::{Bibliography, ChunksExt};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BibEntry {
    pub key: String,
    /// `article`, `book`…
    pub kind: String,
    /// Family names of the authors (or editors), in order.
    pub authors: Vec<String>,
    pub year: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BibInfo {
    pub entries: Vec<BibEntry>,
    /// Parse error with its line number; entries are empty when this is set.
    pub error: Option<String>,
}

pub fn parse(src: &str) -> BibInfo {
    match Bibliography::parse(src) {
        Ok(bib) => BibInfo { entries: bib.iter().map(entry).collect(), error: None },
        Err(e) => {
            let line = src[..e.span.start.min(src.len())].matches('\n').count() + 1;
            BibInfo { entries: Vec::new(), error: Some(format!("Line {line}: {e}")) }
        }
    }
}

/// Keys of all entries, or `None` when the source doesn't parse.
pub fn keys(src: &str) -> Option<HashSet<String>> {
    Bibliography::parse(src).ok().map(|b| b.iter().map(|e| e.key.clone()).collect())
}

fn entry(e: &biblatex::Entry) -> BibEntry {
    let people = e.author().or_else(|_| e.editors().map(|eds| eds.into_iter().flat_map(|(p, _)| p).collect()));
    let field = |name: &str| e.get(name).map(|c| c.format_verbatim()).filter(|s| !s.trim().is_empty());
    let year = field("year").or_else(|| field("date").map(|d| d.chars().take(4).collect()));
    BibEntry {
        key: e.key.clone(),
        kind: e.entry_type.to_string(),
        authors: people.unwrap_or_default().into_iter().map(|p| p.name).filter(|n| !n.is_empty()).collect(),
        year,
        title: field("title").unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_and_reports_errors_with_lines() {
        let info = parse(
            "@book{knuth, author = {Donald E. Knuth}, title = {The {\\TeX}book}, year = 1984}\n\
             @misc{lora, author = {{LoRa Alliance}}, title = {Spec}, date = {2019-05-01}}",
        );
        assert!(info.error.is_none(), "{:?}", info.error);
        assert_eq!(info.entries[0].authors, vec!["Knuth"]);
        assert_eq!(info.entries[0].year.as_deref(), Some("1984"));
        assert_eq!(info.entries[1].authors, vec!["LoRa Alliance"]);
        assert_eq!(info.entries[1].year.as_deref(), Some("2019"));

        let bad = parse("@book{ok, title = {x}}\n\n@book{broken, title = {x}");
        assert!(bad.error.unwrap().starts_with("Line"));
        assert!(keys("@book{a, title={x}}").unwrap().contains("a"));
    }
}
