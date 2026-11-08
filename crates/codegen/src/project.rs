//! Projects (one document file per chapter) → a main `.tex` that `\include`s one
//! `.tex` per chapter.
//!
//! The chapters are joined into one document first ([`Project::combined`]), so
//! everything that looks at the whole book (languages, references between chapters,
//! the reference list Texloom writes for Arabic documents) works as for a single
//! file; only where the body's blocks are written differs.

use texloom_core::{DocumentKind, Project, ProjectItem, Template};

use crate::tex::{Layout, Piece};
use crate::{Engine, Generated};

/// Path of the `n`th chapter's file (0-based) in the build folder, without `.tex`.
/// By position, not by the chapter's own file name: always a valid `\include` name
/// (no spaces or Arabic letters), and reordering chapters recompiles the whole book
/// anyway.
pub fn chapter_tex_path(n: usize) -> String {
    format!("chapters/ch{:02}", n + 1)
}

/// Like [`crate::generate_for`], for a project. `only_chapter` (0-based, among the
/// chapters) compiles just that chapter with `\includeonly`; the build folder must
/// then still hold the other chapters' `.aux` files from a full compile.
///
/// Callers should run [`Template::validate`], [`texloom_core::ProjectFile::validate`]
/// and [`Project::validate`] first. Presentations aren't split into files.
pub fn generate_project_for(template: &Template, project: &Project, engine: Engine, only_chapter: Option<usize>) -> Generated {
    let doc = project.combined();
    let layout = (template.kind != DocumentKind::Presentation).then(|| layout(project, only_chapter));
    crate::language::with_lr_numbers(engine == Engine::Lua, || crate::generate_with(template, &doc, engine, layout.as_ref()))
}

/// Where each chapter's blocks are in the combined document. Must match how
/// [`Project::combined`] joins them (an appendix marker before the first appendix).
fn layout(project: &Project, only_chapter: Option<usize>) -> Layout {
    let mut pieces = Vec::new();
    let mut at = 0;
    let mut in_appendix = false;
    let mut n = 0;
    for item in &project.items {
        match item {
            ProjectItem::Part { title } => pieces.push(Piece::Part(title.clone())),
            ProjectItem::Chapter { body, appendix, .. } => {
                if *appendix && !in_appendix {
                    pieces.push(Piece::Main(at));
                    at += 1;
                    in_appendix = true;
                }
                let len = body.content.len();
                pieces.push(Piece::Chapter { path: chapter_tex_path(n), range: at..at + len });
                at += len;
                n += 1;
            }
        }
    }
    Layout { pieces, include_only: only_chapter.filter(|&i| i < n).map(|i| vec![chapter_tex_path(i)]) }
}
