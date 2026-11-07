//! Generates LaTeX from Texloom templates and documents.
//!
//! - [`template_to_cls`]: a template becomes a standalone LaTeX class.
//! - [`document_to_tex`]: a document becomes a `.tex` file using that class.
//! - [`generate`]: both at once, plus the list of template assets the build needs.

mod cls;
pub mod language;
pub mod refs;
pub mod strings;
mod latex;
mod presentation;
mod publisher;
mod tex;

pub use cls::{template_to_cls, template_to_cls_for};
pub use latex::escape;
pub use publisher::{check_recipe, kit_override, xetex_override, Authors};
pub use tex::{document_to_tex, document_to_tex_mapped, has_bibliography};

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

    /// Generated support files (class, bibliography) as (path, contents) pairs.
    pub fn support_files(&self) -> Vec<(String, Vec<u8>)> {
        let mut files = vec![(self.cls_file_name(), self.cls.as_bytes().to_vec())];
        if let Some(bib) = &self.bibliography {
            files.push(("refs.bib".into(), bib.as_bytes().to_vec()));
        }
        files
    }
}

/// Callers should run [`Template::validate`] first; generation assumes a valid template.
pub fn generate(template: &Template, doc: &Document) -> Generated {
    generate_for(template, doc, Engine::Xe)
}

/// Like [`generate`], for a particular engine.
pub fn generate_for(template: &Template, doc: &Document, engine: Engine) -> Generated {
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
    let (tex, block_lines) = document_to_tex_mapped(template, doc);
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
    }
}
