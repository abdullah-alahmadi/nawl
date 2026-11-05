//! Every built-in publisher template generates LaTeX for the sample article, and
//! (ignored by default) compiles to a PDF with the real classes.

use std::path::Path;

use texloom_core::{DocumentFile, Template};

const JOURNALS: &[&str] =
    &["ieee-journal", "ieee-conference", "elsevier-article", "acm-conference", "springer-lncs", "aps-physical-review"];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn template(id: &str) -> Template {
    let text = std::fs::read_to_string(root().join(format!("templates/{id}/template.json"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn article() -> DocumentFile {
    serde_json::from_str(&std::fs::read_to_string(root().join("examples/journal-article.loom")).unwrap()).unwrap()
}

#[test]
fn journal_templates_generate_publisher_front_matter() {
    let doc = article().document;
    for id in JOURNALS {
        let t = template(id);
        assert!(t.validate().is_empty(), "{id}: {:?}", t.validate());
        let g = texloom_codegen::generate(&t, &doc);
        let class = &t.base.as_ref().unwrap().class;
        assert!(g.cls.contains(&format!("{{{class}}}")), "{id} loads {class}");
        assert!(g.tex.contains("\\title{Sensor-Driven Irrigation"), "{id} has the title");
        assert!(!g.tex.contains("\\texloomfrontmatter"), "{id} uses the publisher title block");
    }
    let ieee = texloom_codegen::document_to_tex(&template("ieee-journal"), &doc);
    assert!(ieee.contains("Sara Ahmed and Omar Khalid are with Department of Electrical Engineering"));
    assert!(ieee.contains("\\begin{IEEEkeywords}"));
    let els = texloom_codegen::document_to_tex(&template("elsevier-article"), &doc);
    assert!(els.contains("\\affiliation[2]{organization={Institute of Agricultural Engineering"));
    assert!(els.contains("irrigation \\sep LoRa"));
}

/// Run with: TEXLOOM_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p texloom-codegen -- --ignored
#[test]
#[ignore]
fn journal_templates_compile() {
    let compiler = texloom_compile::Compiler::locate(&[]).expect("tectonic");
    let doc = article().document;
    for id in JOURNALS {
        let g = texloom_codegen::generate(&template(id), &doc);
        let figures: Vec<(String, Vec<u8>)> =
            g.figures.iter().map(|f| (f.clone(), std::fs::read(root().join("examples").join(f)).unwrap())).collect();
        let support = g.support_files();
        let files: Vec<texloom_compile::Asset> =
            support.iter().chain(figures.iter()).map(|(p, b)| texloom_compile::Asset { path: p, bytes: b }).collect();
        let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{id}: {e:?}"));
        assert!(out.pdf.starts_with(b"%PDF"), "{id}");
    }
}
