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
    #[serde(other)]
    Unknown,
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
                {"type": "table", "content": [{"type": "tableRow"}]},
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
    fn field_values_are_text_or_list() {
        let m: Metadata = serde_json::from_value(json!({"title": "T", "team": ["A", "B"]})).unwrap();
        assert_eq!(m["title"], FieldValue::Text("T".into()));
        assert_eq!(m["team"], FieldValue::List(vec!["A".into(), "B".into()]));
    }
}
