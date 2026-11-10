//! Generates LaTeX from Texloom templates and documents.
//!
//! - [`template_to_cls`]: a template becomes a standalone LaTeX class.
//! - [`document_to_tex`]: a document becomes a `.tex` file using that class.
//! - [`generate`]: both at once, plus the list of template assets the build needs.

mod armath;
mod cls;
pub mod language;
pub mod refs;
pub mod strings;
mod latex;
mod presentation;
mod project;
mod publisher;
mod tex;

pub use cls::{template_to_cls, template_to_cls_for};
pub use latex::escape;
pub use project::{chapter_tex_path, generate_project_for};
pub use publisher::{check_recipe, kit_override, xetex_override, Authors};
pub use tex::{document_to_tex, document_to_tex_mapped, has_bibliography, ChapterTex};

use texloom_core::{Document, DocumentKind, Template, TitleContent};

/// The TeX engine the generated files are for. Tectonic is XeTeX-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Engine {
    /// XeLaTeX (and Tectonic): fonts through fontspec, PDF written by xdvipdfmx.
    #[default]
    Xe,
    /// pdfLaTeX: classic 8-bit fonts (T1), and publisher kits run unmodified.
    Pdf,
    /// LuaLaTeX: fonts through fontspec.
    Lua,
}

impl Engine {
    /// fontspec (OpenType and system fonts) is available.
    pub fn unicode_fonts(self) -> bool {
        self != Engine::Pdf
    }
}

/// Everything needed to compile a document: write `cls` as `<class_name>.cls` and
/// `tex` as the main file, and copy each of `assets` (paths relative to the
/// template folder) and `figures` (relative to the document's folder) next to them.
#[derive(Debug, Clone)]
pub struct Generated {
    pub class_name: String,
    pub cls: String,
    pub tex: String,
    pub assets: Vec<String>,
    pub figures: Vec<String>,
    /// A publisher's kit files (paths relative to the template folder). They go next
    /// to the main file, flattened, because classes load their companions by name.
    pub kit_files: Vec<String>,
    /// Contents of `refs.bib`, when the document has a reference list.
    pub bibliography: Option<String>,
    /// Line of `tex` (1-based) where each top-level block of the body starts.
    pub block_lines: Vec<u32>,
    /// Files of the template's library fonts (`fonts/cairo-Regular.ttf`…), relative
    /// to the font library; they go into the build folder at the same paths.
    pub fonts: Vec<String>,
    /// For a project: each chapter's `.tex`, `\include`d by `tex` (empty otherwise).
    pub chapters: Vec<ChapterTex>,
}

/// The files of a template's library fonts (see [`texloom_core::fonts`]).
pub fn library_font_files(template: &Template) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for choice in template.all_fonts() {
        if let texloom_core::FontChoice::Library { id } = choice
            && let Some(font) = texloom_core::fonts::find(id)
        {
            for file in font.files() {
                if !files.contains(&file) {
                    files.push(file);
                }
            }
        }
    }
    files
}

/// Where a kit file goes in the build folder: its file name only.
pub fn kit_destination(rel: &str) -> String {
    rel.rsplit('/').next().unwrap_or(rel).to_string()
}

impl Generated {
    pub fn cls_file_name(&self) -> String {
        format!("{}.cls", self.class_name)
    }

    /// Generated support files (class, bibliography, a project's chapter files) as
    /// (path, contents) pairs.
    pub fn support_files(&self) -> Vec<(String, Vec<u8>)> {
        let mut files = vec![(self.cls_file_name(), self.cls.as_bytes().to_vec())];
        if let Some(bib) = &self.bibliography {
            files.push(("refs.bib".into(), bib.as_bytes().to_vec()));
        }
        for c in &self.chapters {
            files.push((format!("{}.tex", c.path), c.tex.as_bytes().to_vec()));
        }
        files
    }
}

/// Callers should run [`Template::validate`] first; generation assumes a valid template.
pub fn generate(template: &Template, doc: &Document) -> Generated {
    generate_for(template, doc, Engine::Xe)
}

/// A document that sets each Arabic formula on a page of its own, exactly its size:
/// pictures of formulas for Word, which can't lay math out right to left. The class,
/// fonts and language setup are the template's, so the pictures match the PDF.
/// `formulas`: the LaTeX and whether it's a displayed equation. Sizes come from the
/// compile's log, see [`formula_picture_sizes`].
pub fn formula_pictures(template: &Template, formulas: &[(String, bool)], engine: Engine) -> Generated {
    let empty = Document { metadata: Default::default(), body: Default::default(), bibliography: String::new() };
    let mut g = generate_for(template, &empty, engine);
    let langs = language::Languages::of(template, &empty);
    let preamble_end = g.tex.find(r"\begin{document}").unwrap_or(g.tex.len());
    g.tex.truncate(preamble_end);
    g.tex.push_str(&armath::picture_pages(formulas, &langs));
    g.bibliography = None;
    g.block_lines.clear();
    g
}

/// Width, height above the baseline and depth below it of each formula picture (pt),
/// from the log of a [`formula_pictures`] compile.
pub fn formula_picture_sizes(log: &str) -> Vec<[f32; 3]> {
    armath::picture_sizes(log)
}

/// Like [`generate`], for a particular engine.
pub fn generate_for(template: &Template, doc: &Document, engine: Engine) -> Generated {
    language::with_lr_numbers(engine == Engine::Lua, || generate_with(template, doc, engine, None))
}

fn generate_with(template: &Template, doc: &Document, engine: Engine, layout: Option<&tex::Layout>) -> Generated {
    let assets: Vec<String> = template
        .placed_items()
        .into_iter()
        .filter_map(|(_, i)| match &i.content {
            // An image field's picture comes from the document folder (in `figures`).
            TitleContent::Image { asset, .. }
                if template.base.is_none() && template.kind == DocumentKind::Article && texloom_core::image_field(asset).is_none() =>
            {
                Some(asset.clone())
            }
            _ => None,
        })
        .collect();
    let (tex, block_lines, chapters) = tex::write_tex(template, doc, layout);
    Generated {
        class_name: template.meta.id.clone(),
        cls: template_to_cls_for(template, engine),
        tex,
        assets,
        figures: {
            // Figure images plus biography photos, all relative to the document folder.
            let mut f = doc.body.figure_sources();
            for p in publisher::photo_sources(doc).into_iter().chain(presentation::image_sources(template, doc)) {
                if !f.contains(&p) {
                    f.push(p);
                }
            }
            f
        },
        kit_files: template.base.as_ref().map(|b| b.files.clone()).unwrap_or_default(),
        bibliography: has_bibliography(doc).then(|| doc.bibliography.clone()),
        block_lines,
        fonts: library_font_files(template),
        chapters,
    }
}
