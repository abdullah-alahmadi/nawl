//! Snapshot tests for the reference template, plus a real compile (ignored by default).

use texloom_core::{has_errors, DocumentFile, Template};

fn template() -> Template {
    serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).expect("template parses")
}

fn document() -> DocumentFile {
    serde_json::from_str(include_str!("../../../examples/senior-design-report.loom")).expect("document parses")
}

#[test]
fn reference_template_is_valid() {
    let issues = template().validate();
    assert!(issues.is_empty(), "{issues:#?}");
    assert!(template().check_metadata(&document().document.metadata).is_empty());
}

#[test]
fn class_snapshot() {
    insta::assert_snapshot!(texloom_codegen::template_to_cls(&template()));
}

#[test]
fn document_snapshot() {
    insta::assert_snapshot!(texloom_codegen::document_to_tex(&template(), &document().document));
}

#[test]
fn figures_and_tables() {
    let tex = texloom_codegen::document_to_tex(&template(), &document().document);
    assert!(tex.contains("\\includegraphics[width=0.85\\linewidth]{figures/system-diagram.png}"));
    assert!(tex.contains("\\begin{tabularx}{\\linewidth}{|L|L|L|}"));
    assert!(tex.contains("{\\bfseries Component} & {\\bfseries Part}"));
    assert_eq!(texloom_codegen::generate(&template(), &document().document).figures, vec!["figures/system-diagram.png"]);
}

#[test]
fn missing_required_fields_show_labels() {
    let mut doc = document().document;
    doc.metadata.remove("advisor");
    doc.metadata.remove("acknowledgements");
    let tex = texloom_codegen::document_to_tex(&template(), &doc);
    assert!(tex.contains("\\texloomfield{advisor}{[Advisor]}"));
    assert!(!tex.contains("acknowledgements"), "optional empty fields are left out");
    let issues = template().check_metadata(&doc.metadata);
    assert_eq!(issues.len(), 1);
    assert!(!has_errors(&issues));
}

/// Run with: TEXLOOM_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p texloom-codegen -- --ignored
#[test]
#[ignore]
fn compiles_to_pdf() {
    let g = texloom_codegen::generate(&template(), &document().document);
    let compiler = texloom_compile::Compiler::locate(&[]).expect("tectonic");
    let cls = g.cls_file_name();
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let figures: Vec<(String, Vec<u8>)> =
        g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).expect("figure exists"))).collect();
    let mut files = vec![texloom_compile::Asset { path: &cls, bytes: g.cls.as_bytes() }];
    files.extend(figures.iter().map(|(p, b)| texloom_compile::Asset { path: p, bytes: b }));
    let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(out.pdf.starts_with(b"%PDF"));
}
