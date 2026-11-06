//! The presentation template turns the sample deck into beamer slides, and
//! (ignored by default) compiles them.

use std::path::Path;

use texloom_core::{DocumentFile, Template};

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn template() -> Template {
    serde_json::from_str(&std::fs::read_to_string(root().join("templates/beamer-presentation/template.json")).unwrap()).unwrap()
}

fn deck() -> DocumentFile {
    serde_json::from_str(&std::fs::read_to_string(root().join("examples/presentation.loom")).unwrap()).unwrap()
}

#[test]
fn deck_becomes_beamer_slides() {
    let t = template();
    assert!(t.validate().is_empty(), "{:?}", t.validate());
    let g = texloom_codegen::generate(&t, &deck().document);
    assert!(g.cls.contains("\\LoadClass[aspectratio=169,11pt]{beamer}"));
    assert!(g.cls.contains("\\usetheme{Madrid}"));
    let tex = &g.tex;
    assert!(tex.contains("\\title{Sensor-Driven Irrigation for Small Farms}"));
    assert!(tex.contains("\\subtitle{Senior design project review}"));
    assert!(tex.contains("\\author{Sara Ahmed \\and Omar Khalid}"));
    assert!(tex.contains("\\begin{frame}\n\\titlepage\n\\end{frame}"));
    assert!(tex.contains("\\section{Motivation}"));
    assert!(tex.contains("\\begin{frame}{Why automate irrigation?}"));
    assert!(tex.contains("\\pause"));
    assert!(tex.contains("\\begin{columns}[T]\n\\begin{column}{0.55\\textwidth}"));
    assert!(tex.contains("\\begin{alertblock}{Key idea}"));
    assert!(tex.contains("\\begin{exampleblock}{Example}"));
    assert!(tex.contains("\\begin{frame}[plain]{Thank you}"));
    assert!(tex.contains("\\begin{frame}[allowframebreaks]{References}"));
    assert_eq!(g.figures, vec!["figures/system-diagram.png".to_string()]);
}

#[test]
fn loose_content_and_code_get_slides() {
    let t = template();
    let doc: texloom_core::Document = serde_json::from_value(serde_json::json!({
        "metadata": {},
        "doc": { "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "Before any slide" }] },
            { "type": "slide", "content": [
                { "type": "slideTitle", "content": [{ "type": "text", "text": "Code" }] },
                { "type": "codeBlock", "content": [{ "type": "text", "text": "x = 1" }] }
            ]}
        ]}
    }))
    .unwrap();
    let tex = texloom_codegen::document_to_tex(&t, &doc);
    assert!(tex.contains("\\begin{frame}\nBefore any slide"), "{tex}");
    assert!(tex.contains("\\begin{frame}[fragile]{Code}"), "{tex}");
    // A missing (required) title shows as a placeholder, like in articles.
    assert!(tex.contains("\\title{[Title]}"), "{tex}");
}

/// Run with: TEXLOOM_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p texloom-codegen -- --ignored
#[test]
#[ignore]
fn deck_compiles() {
    let compiler = texloom_compile::Compiler::locate(&[]).expect("tectonic");
    let g = texloom_codegen::generate(&template(), &deck().document);
    let figures: Vec<(String, Vec<u8>)> =
        g.figures.iter().map(|f| (f.clone(), std::fs::read(root().join("examples").join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<texloom_compile::Asset> =
        support.iter().chain(figures.iter()).map(|(p, b)| texloom_compile::Asset { path: p, bytes: b }).collect();
    let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(out.pdf.starts_with(b"%PDF"));
}

#[test]
fn slides_in_an_article_become_plain_latex() {
    let article: Template =
        serde_json::from_str(&std::fs::read_to_string(root().join("templates/senior-design/template.json")).unwrap()).unwrap();
    let tex = texloom_codegen::document_to_tex(&article, &deck().document);
    assert!(!tex.contains("\\begin{frame}"));
    assert!(!tex.contains("\\pause"));
    assert!(!tex.contains("alertblock"));
    assert!(tex.contains("\\noindent\\textbf{Why automate irrigation?}"));
    assert!(tex.contains("\\begin{minipage}[t]{0.53\\linewidth}"));
}

#[test]
fn accent_colour_sets_the_structure_colour() {
    let mut t = template();
    t.presentation.as_mut().unwrap().accent_color = Some("#1f4e79".into());
    assert!(t.validate().is_empty());
    let cls = texloom_codegen::template_to_cls(&t);
    assert!(cls.contains("\\usecolortheme[RGB={31,78,121}]{structure}"), "{cls}");
    t.presentation.as_mut().unwrap().accent_color = Some("blue".into());
    assert!(!t.validate().is_empty());
}
