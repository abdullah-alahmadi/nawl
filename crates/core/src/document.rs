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
}

/// Values for the template's metadata fields, by field id.
pub type Metadata = BTreeMap<String, FieldValue>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub enum FieldValue {
    Text(String),
    List(Vec<String>),
}

impl FieldValue {
    pub fn is_empty(&self) -> bool {
        match self {
            FieldValue::Text(s) => s.trim().is_empty(),
            FieldValue::List(v) => v.iter().all(|s| s.trim().is_empty()),
        }
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
        content: Vec<TableFigurePart>,
    },
    /// A table without a caption (e.g. pasted).
    Table {
        #[serde(default)]
        content: Vec<TableRow>,
    },
    #[serde(other)]
    Unknown,
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

impl Body {
    /// Image paths used by figures anywhere in the document, without duplicates.
    pub fn figure_sources(&self) -> Vec<String> {
        fn walk(blocks: &[Block], out: &mut Vec<String>) {
            for b in blocks {
                match b {
                    Block::Figure { attrs, .. } if !attrs.src.is_empty() && !out.contains(&attrs.src) => {
                        out.push(attrs.src.clone())
                    }
                    Block::Blockquote { content } => walk(content, out),
                    Block::BulletList { content } | Block::OrderedList { content, .. } => {
                        content.iter().for_each(|li| walk(&li.content, out))
                    }
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.content, &mut out);
        out
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MathAttrs {
    #[serde(default)]
    pub latex: String,
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
    #[serde(other)]
    Unknown,
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
        let Block::Paragraph { content } = &body.content[1] else { panic!() };
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
        let Block::TableFigure { content } = &body.content[1] else { panic!() };
        let TableFigurePart::Table { content: rows } = &content[1] else { panic!() };
        assert!(rows[0].content[0].is_header());
        assert_eq!(body.figure_sources(), vec!["figures/a.png".to_string()]);
    }

    #[test]
    fn field_values_are_text_or_list() {
        let m: Metadata = serde_json::from_value(json!({"title": "T", "team": ["A", "B"]})).unwrap();
        assert_eq!(m["title"], FieldValue::Text("T".into()));
        assert_eq!(m["team"], FieldValue::List(vec!["A".into(), "B".into()]));
    }
}
