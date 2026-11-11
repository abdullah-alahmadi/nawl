//! Nawl's own article templates: a title block on the first page, the abstract
//! under it, one or two columns.

use nawl_core::{DocumentFile, Template, has_errors};

const ARTICLES: [&str; 3] = ["general-article", "article-two-column", "short-paper"];

fn template(id: &str) -> Template {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../templates/{id}/template.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn document() -> DocumentFile {
    serde_json::from_str(include_str!("../../../examples/article.nawl")).expect("document parses")
}

#[test]
fn article_templates_are_valid() {
    for id in ARTICLES {
        let t = template(id);
        assert!(!has_errors(&t.validate()), "{id}: {:#?}", t.validate());
        assert_eq!(t.meta.id, id);
    }
}

#[test]
fn layout_settings_are_checked() {
    let paths = |t: &Template| t.validate().into_iter().map(|i| i.path).collect::<Vec<_>>();
    let mut t = template("general-article");
    t.page.columns = 3;
    t.title_page.block_height_mm = Some(10.0);
    let found = paths(&t);
    assert!(found.contains(&"page.columns".to_string()), "{found:?}");
    assert!(found.contains(&"titlePage.blockHeightMm".to_string()), "inside the top margin: {found:?}");
    // The id names the class, which mustn't be one of LaTeX's own.
    t = template("general-article");
    t.meta.id = "article".into();
    assert!(paths(&t).contains(&"meta.id".to_string()));
}

/// The title block flows: rows are placed below the real bottom of the row above,
/// so a long title pushes the authors (and the text) down instead of overlapping.
#[test]
fn title_block_is_on_the_first_page() {
    let cls = nawl_codegen::template_to_cls(&template("general-article"));
    assert!(!cls.contains("\\begin{titlepage}"), "no page of its own");
    assert!(cls.contains("\\par\\noindent\\begin{tikzpicture}\\path (0,0) -- (\\linewidth,0);"), "{cls}");
    assert!(cls.contains("\\coordinate (tl@row1) at (current bounding box.south -| 0,0);"), "{cls}");
    assert!(cls.contains("at ([shift={(80mm,"), "positions across are relative to the text's left edge: {cls}");
    // Authors run on in a line.
    assert!(cls.contains("{\\renewcommand\\texloomsep{, }"), "{cls}");
    // The abstract under the title, as a section heading on the same page; the text
    // follows on page 1, numbered from 1.
    assert!(cls.contains("\\tl@ifnonempty{abstract}{\\section*{Abstract}\\tl@f{abstract}\\par}%"), "{cls}");
    assert!(cls.contains("\\newcommand\\texloomfrontmatter{%\n  \\pagenumbering{arabic}\\pagestyle{texloommain}"), "{cls}");
    assert!(cls.contains("\\newcommand\\texloommainmatter{%\n  \\let\\ps@plain\\ps@texloomchapter\\pagestyle{texloommain}\\tl@marks}"), "{cls}");
}

#[test]
fn two_columns_span_the_title_block() {
    let cls = nawl_codegen::template_to_cls(&template("article-two-column"));
    assert!(cls.contains("\\LoadClass[10pt,oneside,twocolumn]{article}"), "{cls}");
    assert!(cls.contains("\\setlength\\columnsep{6mm}"), "{cls}");
    assert!(cls.contains("  \\twocolumn[{%\n  \\tl@titlepage\n"), "{cls}");
    assert!(cls.contains("\\par\\vspace{\\baselineskip}}]%"), "{cls}");
}

/// Templates with a title page of their own are unchanged.
#[test]
fn title_pages_stay_pages() {
    let t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
    let cls = nawl_codegen::template_to_cls(&t);
    assert!(cls.contains("\\begin{titlepage}"));
    assert!(!cls.contains("\\twocolumn"));
    assert!(cls.contains("\\pagenumbering{roman}"));
}

/// Run with: NAWL_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p nawl-codegen --test articles -- --ignored
/// Leaves the PDFs in $NAWL_ARTICLE_OUT when set, to look at.
#[test]
#[ignore]
fn article_templates_compile() {
    let doc = document().document;
    let out = std::env::var_os("NAWL_ARTICLE_OUT").map(std::path::PathBuf::from);
    let pdflatex = nawl_compile::SystemLatex::find(nawl_compile::LatexProgram::PdfLatex);
    for id in ARTICLES {
        let t = template(id);
        let mut runs = vec![(nawl_codegen::Engine::Xe, None)];
        if let Some(l) = pdflatex.clone() {
            runs.push((nawl_codegen::Engine::Pdf, Some(l)));
        }
        for (engine, latex) in runs {
            let g = nawl_codegen::generate_for(&t, &doc, engine);
            let support = g.support_files();
            let files: Vec<nawl_compile::Asset> = support.iter().map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
            let result = match &latex {
                Some(l) => l.compile(&g.tex, &files),
                None => nawl_compile::Compiler::locate(&[]).expect("tectonic").compile(&g.tex, &files),
            };
            let pdf = result.unwrap_or_else(|e| panic!("{id} ({engine:?}): {e:?}")).pdf;
            assert!(pdf.starts_with(b"%PDF"), "{id} ({engine:?})");
            if let Some(dir) = &out {
                std::fs::write(dir.join(format!("{id}-{engine:?}.pdf")), pdf).unwrap();
            }
        }
    }
}
