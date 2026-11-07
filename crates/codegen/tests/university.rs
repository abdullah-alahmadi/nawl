//! University report templates: cover pages before chapters, references and
//! appendices, and a title-page logo the author picks (an image field).

use texloom_core::{DocumentFile, Template};

fn majmaah() -> (Template, texloom_core::Document) {
    let t: Template = serde_json::from_str(include_str!("../../../templates/majmaah-engineering/template.json")).unwrap();
    let doc: DocumentFile = serde_json::from_str(include_str!("../../../examples/majmaah-report.loom")).unwrap();
    (t, doc.document)
}

#[test]
fn majmaah_template_is_valid() {
    let (t, doc) = majmaah();
    let issues = t.validate();
    assert!(!texloom_core::has_errors(&issues), "{issues:#?}");
    assert!(t.structure.cover_pages);
    let g = texloom_codegen::generate(&t, &doc);
    // The logo comes from the document folder, not from the template.
    assert!(g.assets.is_empty(), "{:?}", g.assets);
    assert!(g.figures.contains(&"figures/logo-placeholder.png".to_string()));
}

#[test]
fn chapters_open_on_cover_pages() {
    let (t, _) = majmaah();
    let cls = texloom_codegen::template_to_cls(&t);
    // From the main text on, chapters (numbered and not) get a page of their own.
    let covers = cls.lines().find(|l| l.starts_with("\\g@addto@macro\\texloommainmatter")).expect("cover pages in the main matter");
    assert_eq!(covers.matches("[\\vspace*{\\fill}\\clearpage]").count(), 2, "{covers}");
    assert!(covers.contains("{\\vspace*{\\fill}\\normalfont"));
    // The appendices: one cover for all of them, in the table of contents.
    let appendix = cls.lines().find(|l| l.starts_with("\\newcommand\\texloomappendix")).unwrap();
    assert!(appendix.contains("\\centering Appendices\\par"), "{appendix}");
    assert!(appendix.contains("\\addcontentsline{toc}{chapter}{Appendices}"));

    // Without the option nothing changes.
    let mut plain = t.clone();
    plain.structure.cover_pages = false;
    let cls = texloom_codegen::template_to_cls(&plain);
    assert!(!cls.contains("\\g@addto@macro\\texloommainmatter"));
    assert!(!cls.contains("\\centering Appendices"));
}

#[test]
fn title_page_logo_comes_from_an_image_field() {
    let (t, mut doc) = majmaah();
    let cls = texloom_codegen::template_to_cls(&t);
    assert!(cls.contains("\\ifcsname tl@f@logo\\endcsname\\node[anchor=north,inner sep=0pt]"), "logo node missing");
    assert!(cls.contains("\\includegraphics[width=32mm]{\\csname tl@f@logo\\endcsname}"));
    let tex = texloom_codegen::document_to_tex(&t, &doc);
    assert!(tex.contains("\\texloomfield{logo}{figures/logo-placeholder.png}"), "{tex}");

    // No logo picked (or an unsafe path): nothing drawn, nothing to copy.
    doc.metadata.remove("logo");
    assert!(!texloom_codegen::document_to_tex(&t, &doc).contains("\\texloomfield{logo}"));
    doc.metadata.insert("logo".into(), texloom_core::FieldValue::Text("../outside.png".into()));
    assert!(!texloom_codegen::document_to_tex(&t, &doc).contains("\\texloomfield{logo}"));
}

#[test]
fn image_field_must_exist() {
    let (mut t, _) = majmaah();
    if let texloom_core::TitleContent::Image { asset, .. } = &mut t.title_page.items[0].content {
        *asset = "{crest}".into();
    }
    let issues = t.validate();
    assert!(issues.iter().any(|i| i.path == "titlePage.items[0].content.asset"), "{issues:#?}");
}

#[test]
#[ignore]
fn majmaah_report_compiles() {
    let compiler = texloom_compile::Compiler::locate(&[]).expect("tectonic");
    let (t, doc) = majmaah();
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let g = texloom_codegen::generate(&t, &doc);
    let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<texloom_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| texloom_compile::Asset { path: p, bytes: b }).collect();
    let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(out.pdf.starts_with(b"%PDF"));
}
