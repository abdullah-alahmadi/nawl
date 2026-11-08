//! Projects: a book or thesis written as one `.loom` file per chapter.
//!
//! A project folder holds a project file (`.loomproj`) with what the whole book shares
//! (template, metadata, bibliography, chapter order) and the chapter files, which are
//! ordinary `.loom` documents with only a body. See PLAN.md, "Projects".

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{walk_blocks, Block, Body, Document, Issue, Metadata, Severity};

/// Value of `format` in project files.
pub const PROJECT_FORMAT: &str = "texloom-project";
/// Current project file version.
pub const PROJECT_VERSION: u32 = 1;
/// File extension of project files.
pub const PROJECT_EXTENSION: &str = "loomproj";

/// A `.loomproj` file. Like a `.loom`, it also embeds a copy of the template under
/// `"template"`, which the app reads and writes itself.
///
/// Note: as for documents, code that saves projects should write the editor's JSON
/// as-is (unknown fields survive) and use this type for reading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectFile {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub metadata: Metadata,
    /// BibTeX source shared by all chapters.
    #[serde(default)]
    pub bibliography: String,
    /// The book's chapters and parts, in order.
    #[serde(default)]
    pub chapters: Vec<ProjectEntry>,
}

/// One entry of a project's chapter list: a part title, or a chapter file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub enum ProjectEntry {
    /// A part ("الباب الأول: الإطار النظري"): groups the chapters after it, up to the
    /// next part. Not a file.
    Part { part: String },
    /// A chapter file, relative to the project file's folder.
    Chapter {
        file: String,
        /// The chapter is an appendix. Appendices come after the other chapters.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        appendix: bool,
    },
}

impl ProjectFile {
    /// An empty project with the current format and version.
    pub fn new() -> Self {
        ProjectFile {
            format: PROJECT_FORMAT.into(),
            version: PROJECT_VERSION,
            metadata: Metadata::new(),
            bibliography: String::new(),
            chapters: Vec::new(),
        }
    }

    /// The chapter files, in order.
    pub fn chapter_files(&self) -> impl Iterator<Item = &str> {
        self.chapters.iter().filter_map(|e| match e {
            ProjectEntry::Chapter { file, .. } => Some(file.as_str()),
            ProjectEntry::Part { .. } => None,
        })
    }

    /// Problems with the chapter list itself (before any chapter is read): unsafe or
    /// repeated paths, appendices before chapters, parts without chapters.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        if self.format != PROJECT_FORMAT {
            issues.push(error("format", crate::tr!("Not a Texloom project file", "ليس ملف مشروع Texloom")));
        }
        if self.version > PROJECT_VERSION {
            issues.push(error(
                "version",
                crate::tr!(
                    "The project was made by a newer Texloom (version {})",
                    "أُنشئ المشروع بإصدار أحدث من Texloom (الإصدار {})",
                    self.version
                ),
            ));
        }
        if self.chapter_files().next().is_none() {
            issues.push(error("chapters", crate::tr!("The project has no chapters", "لا يحتوي المشروع على فصول")));
        }
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut in_appendix = false;
        for (i, entry) in self.chapters.iter().enumerate() {
            let path = format!("chapters[{i}]");
            match entry {
                ProjectEntry::Chapter { file, appendix } => {
                    if !is_safe_chapter_path(file) {
                        issues.push(error(
                            &path,
                            crate::tr!(
                                "Chapter file \"{file}\" must be inside the project folder",
                                "يجب أن يكون ملف الفصل \"{file}\" داخل مجلد المشروع"
                            ),
                        ));
                    }
                    // Paths are compared without case: Windows and macOS file systems ignore it.
                    if let Some(first) = seen.insert(file.replace('\\', "/").to_lowercase(), i) {
                        issues.push(error(
                            &path,
                            crate::tr!(
                                "Chapter file \"{file}\" is listed twice (also chapters[{first}])",
                                "ملف الفصل \"{file}\" مذكور مرتين (وأيضًا chapters[{first}])"
                            ),
                        ));
                    }
                    if *appendix {
                        in_appendix = true;
                    } else if in_appendix {
                        issues.push(error(
                            &path,
                            crate::tr!(
                                "Chapter \"{file}\" comes after the appendices; appendices must come last",
                                "الفصل \"{file}\" يأتي بعد الملاحق؛ يجب أن تكون الملاحق في النهاية"
                            ),
                        ));
                    }
                }
                ProjectEntry::Part { part } => {
                    if in_appendix {
                        issues.push(error(&path, crate::tr!("Parts can't come among the appendices", "لا يمكن أن تأتي الأبواب بين الملاحق")));
                    }
                    if part.trim().is_empty() {
                        issues.push(warn(&path, crate::tr!("A part has no title", "باب بلا عنوان")));
                    }
                    let next_is_chapter = matches!(self.chapters.get(i + 1), Some(ProjectEntry::Chapter { appendix: false, .. }));
                    if !next_is_chapter {
                        issues.push(warn(
                            &path,
                            crate::tr!("Part \"{}\" has no chapters", "الباب \"{}\" لا يحتوي على فصول", part.trim()),
                        ));
                    }
                }
            }
        }
        issues
    }
}

impl Default for ProjectFile {
    fn default() -> Self {
        ProjectFile::new()
    }
}

/// A chapter path that stays inside the project folder: relative, no `..`, no drive.
pub fn is_safe_chapter_path(path: &str) -> bool {
    let p = path.replace('\\', "/");
    !p.trim().is_empty()
        && !p.starts_with('/')
        && !p.contains(':')
        && p.split('/').all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

/// A project with its chapters read: what code generation and checks work on.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Project {
    pub metadata: Metadata,
    pub bibliography: String,
    pub items: Vec<ProjectItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectItem {
    Part { title: String },
    Chapter {
        /// The chapter's file, as written in the project file (for messages).
        file: String,
        body: Body,
        appendix: bool,
    },
}

impl Project {
    /// Builds a project from its file and a reader for chapter bodies (by path, as
    /// written in the project file). A chapter that can't be read is an error.
    pub fn load<E>(file: &ProjectFile, mut read: impl FnMut(&str) -> Result<Body, E>) -> Result<Project, (String, E)> {
        let mut items = Vec::new();
        for entry in &file.chapters {
            items.push(match entry {
                ProjectEntry::Part { part } => ProjectItem::Part { title: part.clone() },
                ProjectEntry::Chapter { file, appendix } => ProjectItem::Chapter {
                    body: read(file).map_err(|e| (file.clone(), e))?,
                    file: file.clone(),
                    appendix: *appendix,
                },
            });
        }
        Ok(Project { metadata: file.metadata.clone(), bibliography: file.bibliography.clone(), items })
    }

    /// The chapters, in order: (file, body, appendix).
    pub fn chapters(&self) -> impl Iterator<Item = (&str, &Body, bool)> {
        self.items.iter().filter_map(|i| match i {
            ProjectItem::Chapter { file, body, appendix } => Some((file.as_str(), body, *appendix)),
            ProjectItem::Part { .. } => None,
        })
    }

    /// The whole book as one document: the chapters' bodies joined, with the appendix
    /// marker before the first appendix. Parts aren't blocks; code generation writes
    /// them between chapters.
    pub fn combined(&self) -> Document {
        let mut content = Vec::new();
        let mut in_appendix = false;
        for (_, body, appendix) in self.chapters() {
            if appendix && !in_appendix {
                content.push(Block::AppendixStart);
                in_appendix = true;
            }
            content.extend(body.content.iter().cloned());
        }
        Document { metadata: self.metadata.clone(), body: Body { content }, bibliography: self.bibliography.clone() }
    }

    /// Problems that span chapters: ids used in two chapters (references would be
    /// ambiguous), appendix markers inside chapters (the project marks appendices),
    /// chapters that don't start with a chapter heading.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        let mut owner: HashMap<String, &str> = HashMap::new();
        for (n, (file, body, _)) in self.chapters().enumerate() {
            let path = format!("chapters.{file}");
            let mut ids: Vec<&str> = Vec::new();
            let mut has_marker = false;
            walk_blocks(&body.content, &mut |b| match b {
                Block::AppendixStart => has_marker = true,
                _ => {
                    if let Some(id) = target_id(b) {
                        ids.push(id);
                    }
                }
            });
            if has_marker {
                issues.push(error(
                    &path,
                    crate::tr!(
                        "Chapter \"{file}\" has an appendix marker; mark appendix chapters in the chapter list instead",
                        "يحتوي الفصل \"{file}\" على علامة بداية الملاحق؛ حدِّد فصول الملاحق في قائمة الفصول بدلًا من ذلك"
                    ),
                ));
            }
            for id in ids {
                match owner.get(id) {
                    Some(other) if *other != file => issues.push(error(
                        &path,
                        crate::tr!(
                            "Id \"{id}\" is used in both \"{other}\" and \"{file}\"",
                            "المعرّف \"{id}\" مستخدم في \"{other}\" و\"{file}\" معًا"
                        ),
                    )),
                    Some(_) => {}
                    None => {
                        owner.insert(id.to_string(), file);
                    }
                }
            }
            let starts_with_chapter = matches!(body.content.first(), Some(Block::Heading { attrs, .. }) if attrs.level == 1);
            if !starts_with_chapter {
                issues.push(warn(
                    &path,
                    crate::tr!(
                        "Chapter {} (\"{file}\") doesn't start with a chapter heading",
                        "الفصل {} (\"{file}\") لا يبدأ بعنوان فصل",
                        n + 1
                    ),
                ));
            }
        }
        issues
    }
}

/// What the editor needs to know about the chapters it doesn't have open: their
/// titles, numbered targets and citations, to number the open chapter where the
/// previous ones end, show references to other chapters, and number citations in
/// book order.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectIndex {
    pub chapters: Vec<ChapterIndex>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChapterIndex {
    pub file: String,
    /// The text of the chapter's first heading (empty when it has none).
    pub title: String,
    pub appendix: bool,
    /// The part the chapter belongs to (the last part before it), if any.
    pub part: Option<String>,
    pub words: u32,
    /// Headings, figures, tables and equations with ids, in order (unnumbered ones
    /// too, marked: they take no number).
    pub targets: Vec<IndexTarget>,
    /// Citation keys in order of first use in the chapter, without duplicates.
    pub citations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexTarget {
    pub id: String,
    pub kind: IndexTargetKind,
    /// Heading level (1–6); 0 for the others.
    pub level: u8,
    pub numbered: bool,
    /// A heading's text, a caption, an equation's LaTeX.
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum IndexTargetKind {
    Heading,
    Figure,
    Table,
    Equation,
}

impl Project {
    /// The index of every chapter.
    pub fn index(&self) -> ProjectIndex {
        let mut part = None;
        let mut chapters = Vec::new();
        for item in &self.items {
            match item {
                ProjectItem::Part { title } => part = Some(title.clone()),
                ProjectItem::Chapter { file, body, appendix } => {
                    chapters.push(ChapterIndex::of(file, body, *appendix, if *appendix { None } else { part.clone() }))
                }
            }
        }
        ProjectIndex { chapters }
    }
}

impl ChapterIndex {
    pub fn of(file: &str, body: &Body, appendix: bool, part: Option<String>) -> ChapterIndex {
        let mut targets = Vec::new();
        walk_blocks(&body.content, &mut |b| {
            let Some(id) = target_id(b) else { return };
            let (kind, level, numbered, text) = match b {
                Block::Heading { attrs, content } => (IndexTargetKind::Heading, attrs.level, attrs.numbered, inline_text(content)),
                Block::Figure { content, .. } => {
                    (IndexTargetKind::Figure, 0, true, content.iter().map(|c| inline_text(&c.content)).collect::<Vec<_>>().join(" "))
                }
                Block::TableFigure { content, .. } => {
                    let caption = content.iter().filter_map(|p| match p {
                        crate::TableFigurePart::Caption { content } => Some(inline_text(content)),
                        _ => None,
                    });
                    (IndexTargetKind::Table, 0, true, caption.collect::<Vec<_>>().join(" "))
                }
                Block::Equation { attrs } => (IndexTargetKind::Equation, 0, attrs.numbered, attrs.latex.clone()),
                _ => return,
            };
            targets.push(IndexTarget { id: id.to_string(), kind, level, numbered, text });
        });
        let mut words = 0u32;
        crate::walk_inlines(&body.content, &mut |i| {
            if let crate::Inline::Text { text, .. } = i {
                words += text.split_whitespace().count() as u32;
            }
        });
        let title = match body.content.first() {
            Some(Block::Heading { content, .. }) => inline_text(content),
            _ => String::new(),
        };
        ChapterIndex { file: file.to_string(), title, appendix, part, words, targets, citations: body.cited_keys() }
    }
}

/// The plain text of inline content.
fn inline_text(content: &[crate::Inline]) -> String {
    content
        .iter()
        .filter_map(|i| match i {
            crate::Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// The id of a block that references can point at (whether numbered or not: ids
/// must be unique either way).
fn target_id(b: &Block) -> Option<&str> {
    let id = match b {
        Block::Heading { attrs, .. } => &attrs.id,
        Block::Figure { attrs, .. } => &attrs.id,
        Block::TableFigure { attrs, .. } => &attrs.id,
        Block::Equation { attrs } => &attrs.id,
        _ => return None,
    };
    id.as_deref().filter(|i| !i.is_empty())
}

fn error(path: &str, message: String) -> Issue {
    Issue { severity: Severity::Error, path: path.into(), message }
}

fn warn(path: &str, message: String) -> Issue {
    Issue { severity: Severity::Warning, path: path.into(), message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::has_errors;
    use serde_json::json;

    fn body(v: serde_json::Value) -> Body {
        serde_json::from_value(v).unwrap()
    }

    fn chapter(title: &str, id: &str, extra: serde_json::Value) -> Body {
        let mut content = vec![json!({"type": "heading", "attrs": {"level": 1, "id": id}, "content": [{"type": "text", "text": title}]})];
        if let serde_json::Value::Array(more) = extra {
            content.extend(more);
        }
        body(json!({"type": "doc", "content": content}))
    }

    #[test]
    fn reads_the_file_format() {
        let f: ProjectFile = serde_json::from_value(json!({
            "format": "texloom-project", "version": 1,
            "template": {"ignored": "here"},
            "metadata": {"title": "كتاب"},
            "bibliography": "@book{a, title={A}}",
            "chapters": [
                {"part": "الباب الأول"},
                {"file": "chapters/01.loom"},
                {"file": "chapters/a1.loom", "appendix": true}
            ]
        }))
        .unwrap();
        assert_eq!(f.chapters[0], ProjectEntry::Part { part: "الباب الأول".into() });
        assert_eq!(f.chapters[2], ProjectEntry::Chapter { file: "chapters/a1.loom".into(), appendix: true });
        assert_eq!(f.chapter_files().collect::<Vec<_>>(), ["chapters/01.loom", "chapters/a1.loom"]);
        assert!(f.validate().is_empty(), "{:?}", f.validate());
        // `appendix: false` isn't written.
        let back = serde_json::to_value(&f).unwrap();
        assert_eq!(back["chapters"][1], json!({"file": "chapters/01.loom"}));
    }

    #[test]
    fn checks_the_chapter_list() {
        let mut f = ProjectFile::new();
        assert!(has_errors(&f.validate()), "no chapters");
        f.chapters = vec![
            ProjectEntry::Chapter { file: "../outside.loom".into(), appendix: false },
            ProjectEntry::Chapter { file: "c:/abs.loom".into(), appendix: false },
            ProjectEntry::Chapter { file: "a.loom".into(), appendix: true },
            ProjectEntry::Chapter { file: "A.loom".into(), appendix: false },
            ProjectEntry::Part { part: "".into() },
        ];
        let issues = f.validate();
        let at = |p: &str| issues.iter().filter(|i| i.path == p).count();
        assert_eq!(at("chapters[0]"), 1, "outside the folder");
        assert_eq!(at("chapters[1]"), 1, "absolute");
        assert_eq!(at("chapters[3]"), 2, "listed twice, after the appendices");
        assert_eq!(at("chapters[4]"), 3, "part among appendices, no title, no chapters");
    }

    #[test]
    fn joins_chapters_and_checks_across_them() {
        let f: ProjectFile = serde_json::from_value(json!({
            "format": "texloom-project", "version": 1,
            "chapters": [{"part": "P"}, {"file": "one.loom"}, {"file": "two.loom"}, {"file": "app.loom", "appendix": true}]
        }))
        .unwrap();
        let read = |file: &str| -> Result<Body, ()> {
            Ok(match file {
                "one.loom" => chapter("One", "h1", json!([{"type": "equation", "attrs": {"latex": "x", "id": "eq"}}])),
                // Same equation id as chapter one, and no chapter heading first.
                "two.loom" => body(json!({"type": "doc", "content": [
                    {"type": "paragraph"},
                    {"type": "equation", "attrs": {"latex": "y", "id": "eq"}}
                ]})),
                _ => chapter("Survey", "h3", json!([{"type": "appendixStart"}])),
            })
        };
        let p = Project::load(&f, read).unwrap();
        let doc = p.combined();
        let kinds: Vec<&str> = doc
            .body
            .content
            .iter()
            .map(|b| match b {
                Block::Heading { .. } => "heading",
                Block::Equation { .. } => "equation",
                Block::AppendixStart => "appendix",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["heading", "equation", "other", "equation", "appendix", "heading", "appendix"]);
        let issues = p.validate();
        assert!(issues.iter().any(|i| i.message.contains("\"eq\"") && i.path == "chapters.two.loom"), "{issues:?}");
        assert!(issues.iter().any(|i| i.path == "chapters.two.loom" && i.severity == Severity::Warning));
        assert!(issues.iter().any(|i| i.path == "chapters.app.loom" && i.severity == Severity::Error), "marker inside a chapter");
        // The index: titles, parts, targets in order, citations, words.
        let index = p.index();
        let files: Vec<&str> = index.chapters.iter().map(|c| c.file.as_str()).collect();
        assert_eq!(files, ["one.loom", "two.loom", "app.loom"]);
        let one = &index.chapters[0];
        assert_eq!((one.title.as_str(), one.part.as_deref(), one.words), ("One", Some("P"), 1));
        assert_eq!(one.targets.iter().map(|t| (t.id.as_str(), t.kind)).collect::<Vec<_>>(), [("h1", IndexTargetKind::Heading), ("eq", IndexTargetKind::Equation)]);
        assert_eq!(one.targets[1].text, "x");
        assert_eq!(index.chapters[1].title, "", "no heading first");
        assert!(index.chapters[2].appendix && index.chapters[2].part.is_none());
        // A chapter that can't be read names its file.
        let missing = Project::load(&f, |file| if file == "two.loom" { Err("gone") } else { Ok(Body::default()) });
        assert_eq!(missing.unwrap_err(), ("two.loom".to_string(), "gone"));
    }
}
