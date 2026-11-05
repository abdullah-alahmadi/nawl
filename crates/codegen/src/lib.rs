//! Generates LaTeX from Texloom templates and documents.
//!
//! - [`template_to_cls`]: a template becomes a standalone LaTeX class.
//! - [`document_to_tex`]: a document becomes a `.tex` file using that class.
//! - [`generate`]: both at once, plus the list of template assets the build needs.

mod cls;
mod latex;
mod tex;

pub use cls::template_to_cls;
pub use latex::escape;
pub use tex::{document_to_tex, has_bibliography};

use texloom_core::{Document, Template, TitleContent};

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
    /// Contents of `refs.bib`, when the document has a reference list.
    pub bibliography: Option<String>,
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
    let assets = template
        .title_page
        .items
        .iter()
        .filter_map(|i| match &i.content {
            TitleContent::Image { asset, .. } => Some(asset.clone()),
            _ => None,
        })
        .collect();
    Generated {
        class_name: template.meta.id.clone(),
        cls: template_to_cls(template),
        tex: document_to_tex(template, doc),
        assets,
        figures: doc.body.figure_sources(),
        bibliography: has_bibliography(doc).then(|| doc.bibliography.clone()),
    }
}
