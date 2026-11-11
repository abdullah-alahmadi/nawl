//! Beamer theme previews for the theme picker: a two-slide sample deck (title slide
//! and a content slide) compiled with a theme and colour theme, cached on disk so
//! each combination is rendered once per engine.

use std::path::{Path, PathBuf};

use nawl_core::{Document, Template};

use crate::templates;

/// Installed themes that don't compile on their own (with pdfLaTeX or XeLaTeX): they
/// need an institution's logo files or fonts, or are internal parts of other themes.
/// Found with `theme_survey` on a full TeX Live; the picker leaves them out.
pub const BROKEN_THEMES: &[&str] = &[
    "Aachen", "BFH", "cbernoulli", "cdirichlet", "cfermat", "CleanEasy", "cmobius", "compatibility", "criemann", "DLRG",
    "focus", "hdu", "HeavenlyClouds", "hohenheim", "I6dv", "I6pd", "I6pd2", "I6td", "nirma", "NPBT", "nxt", "plain",
    "pureminimalistic", "RWTH", "sTeX", "TianQing", "TorinoTh", "TUDa", "TUDa-mecheng", "TUDa2008", "TUDa2023", "UR", "ZH",
];

/// A small deck that shows what a theme does: title slide, section, frame title,
/// bullets, a block and an alert block.
fn sample() -> Document {
    serde_json::from_value(serde_json::json!({
        "metadata": {
            "title": "Presentation title",
            "subtitle": "A subtitle",
            "authors": [{ "name": "Presenter Name", "affiliations": ["University"] }],
            "date": "2026"
        },
        "doc": { "type": "doc", "content": [
            { "type": "heading", "attrs": { "level": 1, "numbered": true }, "content": [{ "type": "text", "text": "Introduction" }] },
            { "type": "slide", "content": [
                { "type": "slideTitle", "content": [{ "type": "text", "text": "Slide title" }] },
                { "type": "bulletList", "content": [
                    { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "A first point" }] }] },
                    { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "A second point" }] }] }
                ]},
                { "type": "slideBox", "attrs": { "kind": "block", "title": "Block" }, "content": [
                    { "type": "paragraph", "content": [{ "type": "text", "text": "Text in a block." }] }
                ]},
                { "type": "slideBox", "attrs": { "kind": "alert", "title": "Alert" }, "content": [
                    { "type": "paragraph", "content": [{ "type": "text", "text": "Something to notice." }] }
                ]}
            ]}
        ]}
    }))
    .expect("sample deck parses")
}

/// The presentation template with another theme, colour and font theme.
pub fn preview_template(theme: &str, color_theme: &str, font_theme: &str) -> Template {
    let (_, json) = templates::BUILTIN_TEMPLATES
        .iter()
        .find(|(id, _)| *id == "beamer-presentation")
        .expect("presentation template exists");
    let mut t: Template = serde_json::from_str(json).expect("built-in template parses");
    let p = t.presentation.get_or_insert_with(Default::default);
    p.theme = theme.into();
    p.color_theme = color_theme.into();
    p.font_theme = font_theme.into();
    t.meta.id = "nawl-theme-preview".into();
    t
}

/// The LaTeX for a preview: `(main .tex, [(file, bytes)])`.
pub fn preview_files(t: &Template, engine: nawl_codegen::Engine) -> (String, Vec<(String, Vec<u8>)>) {
    let g = nawl_codegen::generate_for(t, &sample(), engine);
    let files = g.support_files();
    (g.tex, files)
}

/// Where a preview is cached: `<cache>/theme-previews/<engine>/<theme>_<color>_<font>.pdf`.
pub fn cache_path(cache: &Path, engine: &str, theme: &str, color_theme: &str, font_theme: &str) -> PathBuf {
    let part = |s: &str| if s.is_empty() { "-".to_string() } else { s.to_string() };
    cache
        .join("theme-previews")
        .join(engine)
        .join(format!("{}_{}_{}.pdf", part(theme), part(color_theme), part(font_theme)))
}

/// Names that may go into `\usetheme{…}` (and a file name).
pub fn safe(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_deck_uses_the_theme() {
        let t = preview_template("Warsaw", "beaver", "");
        assert!(t.validate().is_empty(), "{:?}", t.validate());
        let (tex, files) = preview_files(&t, nawl_codegen::Engine::Xe);
        assert!(tex.contains("\\titlepage"));
        assert!(tex.contains("\\begin{frame}{Slide title}"));
        let cls = String::from_utf8(files[0].1.clone()).unwrap();
        assert!(cls.contains("\\usetheme{Warsaw}") && cls.contains("\\usecolortheme{beaver}"));
        assert!(!safe("a}b") && safe("geode-amethyst"));
        assert_eq!(
            cache_path(Path::new("/c"), "tectonic", "Madrid", "", ""),
            PathBuf::from("/c/theme-previews/tectonic/Madrid_-_-.pdf")
        );
    }
}

#[cfg(test)]
mod survey {
    use super::*;
    use nawl_compile::{Asset, LatexProgram, SystemLatex};

    /// Compiles the preview deck with every installed theme (pdfLaTeX, or XeLaTeX with
    /// NAWL_ENGINE=xelatex) and prints the
    /// ones that fail. Run with: cargo test -p nawl theme_survey -- --ignored --nocapture
    #[test]
    #[ignore]
    fn theme_survey() {
        let xe = std::env::var("NAWL_ENGINE").as_deref() == Ok("xelatex");
        let (program, target) =
            if xe { (LatexProgram::XeLatex, nawl_codegen::Engine::Xe) } else { (LatexProgram::PdfLatex, nawl_codegen::Engine::Pdf) };
        let tex = SystemLatex::find(program).expect("installed TeX");
        let found = nawl_compile::installed::beamer_themes();
        let mut failed = Vec::new();
        let start = std::time::Instant::now();
        for theme in &found.themes {
            let (main, files) = preview_files(&preview_template(theme, "", ""), target);
            let assets: Vec<Asset> = files.iter().map(|(p, b)| Asset { path: p, bytes: b }).collect();
            if let Err(e) = tex.compile(&main, &assets) {
                let msg = match e {
                    nawl_compile::CompileError::Failed { errors, .. } => errors.first().map(|e| e.message.lines().next().unwrap_or("").to_string()).unwrap_or_default(),
                    other => other.to_string(),
                };
                println!("FAIL {theme}: {msg}");
                failed.push(theme.clone());
            }
        }
        println!("{} of {} themes compiled in {:?}; failed: {failed:?}", found.themes.len() - failed.len(), found.themes.len(), start.elapsed());
    }
}

#[cfg(test)]
mod tectonic_previews {
    use super::*;
    use nawl_compile::{Asset, Compiler};

    /// Run with: NAWL_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p nawl previews_compile -- --ignored
    #[test]
    #[ignore]
    fn previews_compile_with_tectonic() {
        let compiler = Compiler::locate(&[]).expect("tectonic");
        for (theme, color) in [("Madrid", ""), ("metropolis", ""), ("Warsaw", "beaver"), ("default", "")] {
            let mut t = preview_template(theme, color, "");
            if theme == "default" {
                t.presentation.as_mut().unwrap().accent_color = Some("#2e7d32".into());
            }
            let (main, files) = preview_files(&t, nawl_codegen::Engine::Xe);
            let assets: Vec<Asset> = files.iter().map(|(p, b)| Asset { path: p, bytes: b }).collect();
            let start = std::time::Instant::now();
            let out = compiler.compile(&main, &assets).unwrap_or_else(|e| panic!("{theme}: {e:?}"));
            assert!(out.pdf.starts_with(b"%PDF"));
            println!("{theme}/{color}: {:?}", start.elapsed());
        }
    }
}
