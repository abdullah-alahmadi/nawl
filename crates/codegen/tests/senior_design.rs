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
fn references_citations_and_appendix() {
    let tex = texloom_codegen::document_to_tex(&template(), &document().document);
    assert!(tex.contains("\\label{tl:fig-system}"));
    assert!(tex.contains("Figure~\\ref{tl:fig-system} shows"));
    assert!(tex.contains("\\eqref{tl:eq-balance}"));
    assert!(tex.contains("Appendix~\\ref{tl:h-design-criteria}"));
    assert!(tex.contains("\\cite{smith2021,lora2019}"));
    // The reference list comes right before the appendices.
    let bib = tex.find("\\texloombibliography").unwrap();
    let app = tex.find("\\texloomappendix").unwrap();
    assert!(bib < app);
}

#[test]
fn document_checks() {
    let t = template();
    let doc = document().document;
    let keys: std::collections::HashSet<String> =
        ["allen1998", "smith2021", "lora2019"].iter().map(|s| s.to_string()).collect();
    assert!(t.check_document(&doc, Some(&keys)).is_empty(), "{:#?}", t.check_document(&doc, Some(&keys)));

    // Remove a required chapter, break a reference, cite an unknown key.
    let mut broken = doc.clone();
    broken.body.content.retain(|b| {
        !matches!(b, texloom_core::Block::Heading { attrs, .. } if attrs.id.as_deref() == Some("h-background"))
    });
    broken.body.content.retain(|b| !matches!(b, texloom_core::Block::Figure { .. }));
    let fewer: std::collections::HashSet<String> = ["allen1998"].iter().map(|s| s.to_string()).collect();
    let issues: Vec<String> = t.check_document(&broken, Some(&fewer)).into_iter().map(|i| i.message).collect();
    assert!(issues.iter().any(|m| m.contains("Required chapter \"Background\" is missing")), "{issues:?}");
    assert!(issues.iter().any(|m| m.contains("1 cross-reference points to something that was deleted")), "{issues:?}");
    assert!(issues.iter().any(|m| m.contains("\"smith2021\" isn't in the references")), "{issues:?}");
}

#[test]
fn no_reference_list_without_citations() {
    let mut doc = document().document;
    doc.body.content.retain(|b| !matches!(b, texloom_core::Block::Paragraph { .. }));
    assert!(!texloom_codegen::has_bibliography(&doc));
    assert!(!texloom_codegen::document_to_tex(&template(), &doc).contains("texloombibliography"));
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
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let figures: Vec<(String, Vec<u8>)> =
        g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).expect("figure exists"))).collect();
    let support = g.support_files();
    assert!(support.iter().any(|(p, _)| p == "refs.bib"));
    let files: Vec<texloom_compile::Asset> = support
        .iter()
        .chain(figures.iter())
        .map(|(p, b)| texloom_compile::Asset { path: p, bytes: b })
        .collect();
    let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(out.pdf.starts_with(b"%PDF"));
}
