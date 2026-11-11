//! Reading BibTeX: the entry list for the editor's citation picker and on-screen
//! citation labels, and the entries Nawl formats itself for reference lists
//! with Arabic (BibTeX runs for the others) and for Word.

use std::collections::{BTreeMap, HashSet};

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
    /// The authors in full, for editing the entry.
    pub people: Vec<PersonName>,
    /// Every field as written (verbatim), for editing the entry.
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonName {
    pub given: String,
    /// Family name with its prefix ("van Gogh").
    pub family: String,
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
    let authors = e.author().unwrap_or_default();
    BibEntry {
        key: e.key.clone(),
        kind: e.entry_type.to_string(),
        authors: people.unwrap_or_default().into_iter().map(|p| p.name).filter(|n| !n.is_empty()).collect(),
        year,
        title: field("title").unwrap_or_default(),
        people: authors
            .into_iter()
            .map(|p| PersonName {
                given: p.given_name,
                family: if p.prefix.is_empty() { p.name } else { format!("{} {}", p.prefix, p.name) },
            })
            .collect(),
        fields: e.fields.iter().map(|(k, v)| (k.clone(), v.format_verbatim())).collect(),
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

        let full = parse("@article{x, author = {van Gogh, Vincent and {LoRa Alliance}}, journal = {Art \\& Science}, volume = 3}");
        let e = &full.entries[0];
        assert_eq!(e.people[0].family, "van Gogh");
        assert_eq!(e.people[0].given, "Vincent");
        assert_eq!(e.people[1].family, "LoRa Alliance");
        assert_eq!(e.fields["volume"], "3");
    }
}
