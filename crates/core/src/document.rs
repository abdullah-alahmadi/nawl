//! The document model. Its JSON shape is exactly the editor's ProseMirror JSON
//! (`{"type": "paragraph", "content": [...]}`), so the frontend's document can be
//! deserialized directly into typed Rust without a conversion layer.
//!
//! Unknown node and mark types deserialize to `Unknown` instead of failing, so a
//! newer editor never makes an older backend reject a document.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Value of `format` in document files.
pub const DOC_FORMAT: &str = "texloom";
/// Current document file version.
pub const DOC_VERSION: u32 = 1;

/// A `.loom` document file: `{"format": "texloom", "version": 1, "metadata": {...}, "doc": {...}}`.
///
/// Note: re-serializing drops unknown node types. Code that saves documents should
/// write the editor's JSON as-is and use this type only for reading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentFile {
    pub format: String,
    pub version: u32,
    #[serde(flatten)]
    pub document: Document,
}

/// Metadata values plus the body.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Document {
    #[serde(default)]
    pub metadata: Metadata,
    #[serde(rename = "doc")]
    pub body: Body,
    /// BibTeX source for citations.
    #[serde(default)]
    pub bibliography: String,
}

/// Values for the template's metadata fields, by field id.
pub type Metadata = BTreeMap<String, FieldValue>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub enum FieldValue {
    Text(String),
    List(Vec<String>),
    Authors(Vec<AuthorInfo>),
}

impl FieldValue {
    pub fn is_empty(&self) -> bool {
        match self {
            FieldValue::Text(s) => s.trim().is_empty(),
            FieldValue::List(v) => v.iter().all(|s| s.trim().is_empty()),
            FieldValue::Authors(a) => a.iter().all(|a| a.name.trim().is_empty()),
        }
    }
}

/// One author of a paper (an `authors` field).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuthorInfo {
    pub name: String,
    /// One or more affiliations, in order.
    #[serde(default)]
    pub affiliations: Vec<String>,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub orcid: String,
    /// Society membership, e.g. "Member, IEEE".
    #[serde(default)]
    pub membership: String,
    #[serde(default)]
    pub corresponding: bool,
    /// Short biography (IEEE journals print these at the end).
    #[serde(default)]
    pub biography: String,
    /// Photo for the biography, relative to the document folder (`figures/…`).
    #[serde(default)]
    pub photo: String,
    /// The name in English, for an Arabic document's English title and abstract
    /// (and Word export); empty when it's the same.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[ts(as = "Option<String>", optional)]
    pub name_en: String,
    /// The affiliations in English, in the same order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub affiliations_en: Vec<String>,
}

/// What an authors field gives a title page besides the names, by placeholder
/// suffix: `{authors_affiliations}`, `{authors_en}`, `{authors_affiliations_en}`,
/// `{authors_email}`.
pub const AUTHOR_SUFFIXES: [&str; 4] = ["_affiliations_en", "_affiliations", "_en", "_email"];

/// The value of an authors field's derived placeholder (`suffix` from
/// [`AUTHOR_SUFFIXES`]), one entry per line: the affiliations (without repeats), the
/// English names or affiliations, or the corresponding author's e-mail.
pub fn author_values(authors: &[AuthorInfo], suffix: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |s: &str| {
        let s = s.trim();
        if !s.is_empty() && !out.iter().any(|x| x == s) {
            out.push(s.to_string());
        }
    };
    match suffix {
        "_affiliations" => authors.iter().flat_map(|a| &a.affiliations).for_each(|f| add(f)),
        "_affiliations_en" => authors.iter().flat_map(|a| &a.affiliations_en).for_each(|f| add(f)),
        "_en" => authors.iter().for_each(|a| add(&a.name_en)),
        "_email" => {
            let who = authors.iter().find(|a| a.corresponding && !a.email.trim().is_empty()).or_else(|| authors.iter().find(|a| !a.email.trim().is_empty()));
            if let Some(a) = who {
                add(&a.email);
            }
        }
        _ => {}
    }
    out
}

/// Authors' names and affiliations with the numbers that tie them together, as
/// journals print them when the authors don't all share one affiliation:
/// "Name¹, Name²" over "¹Affiliation", "²Affiliation". Each entry is its text and its
/// number(s) ("1", "1,2"); the numbers are `None` everywhere when they aren't needed
/// (one affiliation, or every author has the same ones). `suffix` is `""` for the
/// names or one of [`AUTHOR_SUFFIXES`].
pub fn author_entries(authors: &[AuthorInfo], suffix: &str) -> Vec<(String, Option<Vec<usize>>)> {
    // The affiliations without repeats, in order of appearance.
    let mut unique: Vec<&str> = Vec::new();
    for a in authors {
        for f in a.affiliations.iter().map(|f| f.trim()).filter(|f| !f.is_empty()) {
            if !unique.contains(&f) {
                unique.push(f);
            }
        }
    }
    let numbers = |a: &AuthorInfo| -> Vec<usize> {
        let mut n: Vec<usize> = a.affiliations.iter().filter_map(|f| unique.iter().position(|u| *u == f.trim()).map(|i| i + 1)).collect();
        n.dedup();
        n
    };
    let first = authors.first().map(numbers);
    let numbered = unique.len() > 1 && authors.iter().any(|a| Some(numbers(a)) != first);
    let mark = |a: &AuthorInfo| numbered.then(|| numbers(a)).filter(|n| !n.is_empty());
    match suffix {
        "" => authors.iter().filter(|a| !a.name.trim().is_empty()).map(|a| (a.name.trim().to_string(), mark(a))).collect(),
        "_en" if numbered => authors.iter().filter(|a| !a.name_en.trim().is_empty()).map(|a| (a.name_en.trim().to_string(), mark(a))).collect(),
        "_affiliations" if numbered => unique.iter().enumerate().map(|(i, f)| (f.to_string(), Some(vec![i + 1]))).collect(),
        // Each numbered affiliation's English form (from the first author who gives one).
        "_affiliations_en" if numbered => unique
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let english = authors.iter().find_map(|a| {
                    let k = a.affiliations.iter().position(|x| x.trim() == *f)?;
                    a.affiliations_en.get(k).map(|e| e.trim()).filter(|e| !e.is_empty())
                })?;
                Some((english.to_string(), Some(vec![i + 1])))
            })
            .collect(),
        _ => author_values(authors, suffix).into_iter().map(|v| (v, None)).collect(),
    }
}

/// An authors field and the derived value a placeholder names (`authors_en` →
/// ("authors", "_en")).
pub fn split_author_placeholder(name: &str) -> Option<(&str, &'static str)> {
    AUTHOR_SUFFIXES.iter().find_map(|s| name.strip_suffix(s).map(|base| (base, *s)))
}

impl AuthorInfo {
    /// The English name, or the name when none is given.
    pub fn english_name(&self) -> &str {
        if self.name_en.trim().is_empty() { &self.name } else { &self.name_en }
    }

    /// The English form of affiliation `i`, or the affiliation itself.
    pub fn english_affiliation(&self, i: usize) -> Option<&str> {
        let own = self.affiliations.get(i)?;
        Some(self.affiliations_en.get(i).map(String::as_str).filter(|s| !s.trim().is_empty()).unwrap_or(own))
    }
}

impl AuthorInfo {
    /// Reads either structured authors or "Name | Affiliation; Affiliation | Email" lines.
    pub fn from_field(v: Option<&FieldValue>) -> Vec<AuthorInfo> {
        let lines: Vec<String> = match v {
            Some(FieldValue::Authors(a)) => {
                return a.iter().filter(|a| !a.name.trim().is_empty()).cloned().collect();
            }
            Some(FieldValue::List(l)) => l.clone(),
            Some(FieldValue::Text(s)) => s.lines().map(str::to_string).collect(),
            None => Vec::new(),
        };
        lines
            .iter()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| {
                let mut parts = l.split('|').map(str::trim);
                let name = parts.next().unwrap_or("").to_string();
                let affiliations =
                    parts.next().unwrap_or("").split(';').map(str::trim).filter(|a| !a.is_empty()).map(String::from).collect();
                let email = parts.next().unwrap_or("").to_string();
                AuthorInfo { name, affiliations, email, ..Default::default() }
            })
            .collect()
    }
}

/// The ProseMirror `doc` node.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Body {
    #[serde(default)]
    pub content: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    Paragraph {
        #[serde(default)]
        attrs: TextAttrs,
        #[serde(default)]
        content: Vec<Inline>,
    },
    Heading {
        attrs: HeadingAttrs,
        #[serde(default)]
        content: Vec<Inline>,
    },
    BulletList {
        #[serde(default)]
        content: Vec<ListItem>,
    },
    OrderedList {
        #[serde(default)]
        attrs: OrderedListAttrs,
        #[serde(default)]
        content: Vec<ListItem>,
    },
    Blockquote {
        #[serde(default)]
        content: Vec<Block>,
    },
    CodeBlock {
        #[serde(default)]
        content: Vec<Inline>,
    },
    HorizontalRule,
    Equation {
        #[serde(default)]
        attrs: MathAttrs,
    },
    /// An image with a caption below it.
    Figure {
        attrs: FigureAttrs,
        #[serde(default)]
        content: Vec<Caption>,
    },
    /// A table with a caption above it: content is `[caption, table]`.
    TableFigure {
        #[serde(default)]
        attrs: IdAttrs,
        #[serde(default)]
        content: Vec<TableFigurePart>,
    },
    /// Everything after this marker is an appendix (chapters lettered A, B…).
    AppendixStart,
    /// LaTeX written into the document as-is (the escape hatch for TikZ, special
    /// tables…).
    RawLatex {
        #[serde(default)]
        attrs: RawLatexAttrs,
    },
    /// A table without a caption (e.g. pasted).
    Table {
        #[serde(default)]
        content: Vec<TableRow>,
    },
    /// A presentation slide: its title line, then its content.
    Slide {
        #[serde(default)]
        attrs: SlideAttrs,
        #[serde(default)]
        content: Vec<Block>,
    },
    /// The title line of a slide (first child of `Slide`).
    SlideTitle {
        #[serde(default)]
        content: Vec<Inline>,
    },
    /// Columns side by side on a slide: content is `Column`s.
    Columns {
        #[serde(default)]
        content: Vec<Block>,
    },
    Column {
        #[serde(default)]
        attrs: ColumnAttrs,
        #[serde(default)]
        content: Vec<Block>,
    },
    /// A titled box on a slide (beamer's block, alertblock, exampleblock).
    SlideBox {
        #[serde(default)]
        attrs: SlideBoxAttrs,
        #[serde(default)]
        content: Vec<Block>,
    },
    /// What follows appears on the next click (beamer's `\pause`).
    SlidePause,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideAttrs {
    #[serde(default)]
    pub id: Option<String>,
    /// No title bar, header or footer (a full-bleed picture, a closing slide).
    #[serde(default)]
    pub plain: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnAttrs {
    /// Share of the slide's text width.
    #[serde(default = "default_column_width")]
    pub width_percent: f32,
}

impl Default for ColumnAttrs {
    fn default() -> Self {
        Self { width_percent: default_column_width() }
    }
}

fn default_column_width() -> f32 {
    50.0
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideBoxAttrs {
    #[serde(default)]
    pub kind: SlideBoxKind,
    #[serde(default)]
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SlideBoxKind {
    #[default]
    Block,
    /// Highlighted (beamer's alertblock).
    Alert,
    /// beamer's exampleblock.
    Example,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FigureAttrs {
    /// Image path relative to the document's folder, e.g. `figures/block-diagram.png`.
    #[serde(default)]
    pub src: String,
    /// Image width as a percentage of the text width.
    #[serde(default = "default_width")]
    pub width_percent: f32,
    /// Stable id that cross-references point at.
    #[serde(default)]
    pub id: Option<String>,
    /// Parts of the image cut off at its edges.
    #[serde(default)]
    pub crop: Option<Crop>,
}

/// How much of an image is cut off at each edge, in percent of its width (left,
/// right) or height (top, bottom).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Crop {
    #[serde(default)]
    pub left: f32,
    #[serde(default)]
    pub right: f32,
    #[serde(default)]
    pub top: f32,
    #[serde(default)]
    pub bottom: f32,
}

impl Crop {
    /// The four edges as fractions (left, bottom, right, top: graphicx's `trim`
    /// order), each kept between 0 and 0.45 so part of the image always remains.
    /// `None` when nothing is cut.
    pub fn trim(&self) -> Option<[f32; 4]> {
        let f = |v: f32| if v.is_finite() { (v / 100.0).clamp(0.0, 0.45) } else { 0.0 };
        let t = [f(self.left), f(self.bottom), f(self.right), f(self.top)];
        t.iter().any(|v| *v > 0.0005).then_some(t)
    }
}

/// Just a stable id, for nodes that cross-references can point at.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IdAttrs {
    #[serde(default)]
    pub id: Option<String>,
}

fn default_width() -> f32 {
    80.0
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Caption {
    #[serde(default)]
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TableFigurePart {
    Caption {
        #[serde(default)]
        content: Vec<Inline>,
    },
    Table {
        #[serde(default)]
        content: Vec<TableRow>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableRow {
    #[serde(default)]
    pub content: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    /// `tableCell` or `tableHeader`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub attrs: CellAttrs,
    #[serde(default)]
    pub content: Vec<Block>,
}

impl TableCell {
    pub fn is_header(&self) -> bool {
        self.kind == "tableHeader"
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellAttrs {
    #[serde(default = "one")]
    pub colspan: u32,
    #[serde(default = "one")]
    pub rowspan: u32,
}

impl Default for CellAttrs {
    fn default() -> Self {
        Self { colspan: 1, rowspan: 1 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    #[serde(default)]
    pub content: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadingAttrs {
    pub level: u8,
    #[serde(default)]
    pub id: Option<String>,
    /// Unnumbered headings ("Acknowledgment") get no number and can't be referenced.
    #[serde(default = "yes")]
    pub numbered: bool,
    /// Written in the other direction than the document (an English heading in an
    /// Arabic document).
    #[serde(default)]
    pub dir: Option<TextDir>,
}

/// A paragraph's own direction, when it differs from the document's.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TextAttrs {
    #[serde(default)]
    pub dir: Option<TextDir>,
}

/// Left to right or right to left (the editor's `dir` attribute).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDir {
    Ltr,
    Rtl,
    /// The editor's "auto": decided by the first strong letter, like the document.
    Auto,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawLatexAttrs {
    #[serde(default)]
    pub latex: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderedListAttrs {
    #[serde(default = "one")]
    pub start: u32,
}

impl Default for OrderedListAttrs {
    fn default() -> Self {
        Self { start: 1 }
    }
}

fn one() -> u32 {
    1
}

/// Calls `f` for every block, including blocks nested in lists, quotes and table cells.
pub fn walk_blocks<'a>(blocks: &'a [Block], f: &mut impl FnMut(&'a Block)) {
    for b in blocks {
        f(b);
        match b {
            Block::Blockquote { content } => walk_blocks(content, f),
            Block::BulletList { content } | Block::OrderedList { content, .. } => {
                content.iter().for_each(|li| walk_blocks(&li.content, f))
            }
            Block::TableFigure { content, .. } => {
                for part in content {
                    if let TableFigurePart::Table { content: rows } = part {
                        rows.iter().flat_map(|r| &r.content).for_each(|c| walk_blocks(&c.content, f));
                    }
                }
            }
            Block::Table { content } => content.iter().flat_map(|r| &r.content).for_each(|c| walk_blocks(&c.content, f)),
            Block::Slide { content, .. }
            | Block::Columns { content }
            | Block::Column { content, .. }
            | Block::SlideBox { content, .. } => walk_blocks(content, f),
            _ => {}
        }
    }
}

/// Calls `f` for every inline node in the document (paragraphs, headings, captions, cells…).
pub fn walk_inlines<'a>(blocks: &'a [Block], f: &mut impl FnMut(&'a Inline)) {
    walk_blocks(blocks, &mut |b| {
        let inl: Vec<&'a [Inline]> = match b {
            Block::Paragraph { content, .. }
            | Block::Heading { content, .. }
            | Block::CodeBlock { content }
            | Block::SlideTitle { content } => vec![content],
            Block::Figure { content, .. } => content.iter().map(|c| c.content.as_slice()).collect(),
            Block::TableFigure { content, .. } => content
                .iter()
                .filter_map(|p| match p {
                    TableFigurePart::Caption { content } => Some(content.as_slice()),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        };
        inl.into_iter().flatten().for_each(&mut *f);
    });
}

impl Body {
    /// Everything a cross-reference can point at, by id.
    pub fn targets(&self) -> std::collections::HashMap<String, TargetKind> {
        let mut out = std::collections::HashMap::new();
        let mut appendix = false;
        walk_blocks(&self.content, &mut |b| {
            let (id, kind) = match b {
                Block::AppendixStart => {
                    appendix = true;
                    return;
                }
                Block::Heading { attrs, .. } if attrs.numbered => {
                    (&attrs.id, TargetKind::Heading { level: attrs.level.clamp(1, 4), appendix })
                }
                Block::Figure { attrs, .. } => (&attrs.id, TargetKind::Figure),
                Block::TableFigure { attrs, .. } => (&attrs.id, TargetKind::Table),
                // Unnumbered equations have no number to refer to.
                Block::Equation { attrs } if attrs.numbered => (&attrs.id, TargetKind::Equation),
                _ => return,
            };
            if let Some(id) = id.as_ref().filter(|i| !i.is_empty()) {
                out.entry(id.clone()).or_insert(kind);
            }
        });
        out
    }

    /// Citation keys in order of first use, without duplicates.
    pub fn cited_keys(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        walk_inlines(&self.content, &mut |i| {
            if let Inline::Citation { attrs } = i {
                for k in &attrs.keys {
                    if !k.is_empty() && !out.contains(k) {
                        out.push(k.clone());
                    }
                }
            }
        });
        out
    }

    /// Ids that references point at but that don't exist (deleted targets).
    pub fn broken_refs(&self) -> Vec<String> {
        let targets = self.targets();
        let mut out = Vec::new();
        walk_inlines(&self.content, &mut |i| {
            if let Inline::Ref { attrs } = i
                && !targets.contains_key(&attrs.target)
                && !out.contains(&attrs.target)
            {
                out.push(attrs.target.clone());
            }
        });
        out
    }

    /// Titles of level-1 headings (chapters or top-level sections).
    pub fn top_level_titles(&self) -> Vec<String> {
        self.content
            .iter()
            .filter_map(|b| match b {
                Block::Heading { attrs, content } if attrs.level == 1 => Some(
                    content
                        .iter()
                        .filter_map(|i| match i {
                            Inline::Text { text, .. } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<String>(),
                ),
                _ => None,
            })
            .collect()
    }

    /// Image paths used by figures anywhere in the document, without duplicates.
    pub fn figure_sources(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        walk_blocks(&self.content, &mut |b| {
            if let Block::Figure { attrs, .. } = b {
                if !attrs.src.is_empty() && !out.contains(&attrs.src) {
                    out.push(attrs.src.clone());
                }
            }
        });
        out
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MathAttrs {
    #[serde(default)]
    pub latex: String,
    #[serde(default)]
    pub id: Option<String>,
    /// Display equations: whether it gets a number (and can be referenced).
    #[serde(default = "yes")]
    pub numbered: bool,
}

impl Default for MathAttrs {
    fn default() -> Self {
        Self { latex: String::new(), id: None, numbered: true }
    }
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
    },
    HardBreak,
    InlineMath {
        #[serde(default)]
        attrs: MathAttrs,
    },
    /// "Figure 3.1": a reference to a heading, figure, table or equation by id.
    Ref {
        #[serde(default)]
        attrs: RefAttrs,
    },
    /// A citation of one or more bibliography entries.
    Citation {
        #[serde(default)]
        attrs: CitationAttrs,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RefAttrs {
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CitationAttrs {
    #[serde(default)]
    pub keys: Vec<String>,
}

/// What a cross-reference can point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// Heading level 1–4; `appendix` when it comes after the appendix marker.
    Heading { level: u8, appendix: bool },
    Figure,
    Table,
    Equation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Mark {
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
    Link { attrs: LinkAttrs },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkAttrs {
    #[serde(default)]
    pub href: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_editor_json_and_tolerates_unknown_nodes() {
        let body: Body = serde_json::from_value(json!({
            "type": "doc",
            "content": [
                {"type": "heading", "attrs": {"level": 2, "textAlign": null}, "content": [{"type": "text", "text": "Hi"}]},
                {"type": "paragraph", "content": [
                    {"type": "text", "text": "x", "marks": [{"type": "bold"}, {"type": "highlight", "attrs": {"color": "red"}}]},
                    {"type": "mention", "attrs": {"id": 1}}
                ]},
                {"type": "paragraph"},
                {"type": "orderedList", "attrs": {"start": 3, "type": null}, "content": []},
                {"type": "callout", "content": [{"type": "paragraph"}]},
                {"type": "equation", "attrs": {"latex": "x^2"}}
            ]
        }))
        .unwrap();
        assert_eq!(body.content.len(), 6);
        assert!(matches!(&body.content[0], Block::Heading { attrs, .. } if attrs.level == 2));
        let Block::Paragraph { content, .. } = &body.content[1] else { panic!() };
        assert!(matches!(&content[0], Inline::Text { marks, .. } if marks == &vec![Mark::Bold, Mark::Unknown]));
        assert_eq!(content[1], Inline::Unknown);
        assert!(matches!(&body.content[3], Block::OrderedList { attrs, .. } if attrs.start == 3));
        assert_eq!(body.content[4], Block::Unknown);
    }

    #[test]
    fn parses_figures_and_tables() {
        let body: Body = serde_json::from_value(json!({"type": "doc", "content": [
            {"type": "figure", "attrs": {"src": "figures/a.png", "widthPercent": 60},
             "content": [{"type": "caption", "content": [{"type": "text", "text": "Block diagram"}]}]},
            {"type": "tableFigure", "content": [
                {"type": "caption", "content": [{"type": "text", "text": "Costs"}]},
                {"type": "table", "content": [
                    {"type": "tableRow", "content": [
                        {"type": "tableHeader", "attrs": {"colspan": 1, "rowspan": 1, "colwidth": null},
                         "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Part"}]}]}
                    ]}
                ]}
            ]},
            {"type": "figure", "attrs": {"src": "figures/a.png"}}
        ]}))
        .unwrap();
        assert!(matches!(&body.content[0], Block::Figure { attrs, .. } if attrs.width_percent == 60.0));
        let Block::TableFigure { content, .. } = &body.content[1] else { panic!() };
        let TableFigurePart::Table { content: rows } = &content[1] else { panic!() };
        assert!(rows[0].content[0].is_header());
        assert_eq!(body.figure_sources(), vec!["figures/a.png".to_string()]);
    }

    #[test]
    fn finds_targets_refs_and_citations() {
        let body: Body = serde_json::from_value(json!({"type": "doc", "content": [
            {"type": "heading", "attrs": {"level": 1, "id": "h1"}, "content": [{"type": "text", "text": "Intro"}]},
            {"type": "paragraph", "content": [
                {"type": "text", "text": "See "},
                {"type": "ref", "attrs": {"target": "f1"}},
                {"type": "ref", "attrs": {"target": "gone"}},
                {"type": "citation", "attrs": {"keys": ["b", "a"]}},
                {"type": "citation", "attrs": {"keys": ["a"]}}
            ]},
            {"type": "figure", "attrs": {"src": "x.png", "id": "f1"},
             "content": [{"type": "caption", "content": [{"type": "citation", "attrs": {"keys": ["c"]}}]}]},
            {"type": "appendixStart"},
            {"type": "equation", "attrs": {"latex": "x", "id": "e1"}},
            {"type": "heading", "attrs": {"level": 1, "id": "ack", "numbered": false}, "content": [{"type": "text", "text": "Thanks"}]},
            {"type": "rawLatex", "attrs": {"latex": "\\vspace{1cm}"}}
        ]}))
        .unwrap();
        let t = body.targets();
        assert_eq!(t["h1"], TargetKind::Heading { level: 1, appendix: false });
        assert_eq!(t["f1"], TargetKind::Figure);
        assert_eq!(t["e1"], TargetKind::Equation);
        assert!(!t.contains_key("ack"), "unnumbered headings can't be referenced");
        assert!(matches!(&body.content[6], Block::RawLatex { attrs } if attrs.latex == "\\vspace{1cm}"));
        assert_eq!(body.broken_refs(), vec!["gone".to_string()]);
        assert_eq!(body.cited_keys(), vec!["b", "a", "c"]);
        assert_eq!(body.top_level_titles(), vec!["Intro", "Thanks"]);
    }

    #[test]
    fn authors_from_objects_or_lines() {
        let m: Metadata = serde_json::from_value(json!({
            "a": [{"name": "Sara", "affiliations": ["U1", "U2"], "corresponding": true}],
            "b": ["Omar | U1; U3 | o@x.org", "Lina"]
        }))
        .unwrap();
        let a = AuthorInfo::from_field(m.get("a"));
        assert_eq!(a[0].affiliations, vec!["U1", "U2"]);
        assert!(a[0].corresponding);
        let b = AuthorInfo::from_field(m.get("b"));
        assert_eq!(b[0].affiliations, vec!["U1", "U3"]);
        assert_eq!(b[0].email, "o@x.org");
        assert!(b[1].affiliations.is_empty());
    }

    #[test]
    fn field_values_are_text_or_list() {
        let m: Metadata = serde_json::from_value(json!({"title": "T", "team": ["A", "B"]})).unwrap();
        assert_eq!(m["title"], FieldValue::Text("T".into()));
        assert_eq!(m["team"], FieldValue::List(vec!["A".into(), "B".into()]));
    }
}
