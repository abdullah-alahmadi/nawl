//! Snapshot tests for the reference template, plus a real compile (ignored by default).

use nawl_core::{has_errors, DocumentFile, Template};

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
    insta::assert_snapshot!(nawl_codegen::template_to_cls(&template()));
}

#[test]
fn document_snapshot() {
    insta::assert_snapshot!(nawl_codegen::document_to_tex(&template(), &document().document));
}

#[test]
fn figures_and_tables() {
    let tex = nawl_codegen::document_to_tex(&template(), &document().document);
    assert!(tex.contains("\\includegraphics[width=0.85\\linewidth]{figures/system-diagram.png}"));
    assert!(tex.contains("\\begin{tabularx}{\\linewidth}{|L|L|L|}"));
    assert!(tex.contains("{\\bfseries Component} & {\\bfseries Part}"));
    assert_eq!(nawl_codegen::generate(&template(), &document().document).figures, vec!["figures/system-diagram.png"]);
}

#[test]
fn references_citations_and_appendix() {
    let tex = nawl_codegen::document_to_tex(&template(), &document().document);
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
        !matches!(b, nawl_core::Block::Heading { attrs, .. } if attrs.id.as_deref() == Some("h-background"))
    });
    broken.body.content.retain(|b| !matches!(b, nawl_core::Block::Figure { .. }));
    let fewer: std::collections::HashSet<String> = ["allen1998"].iter().map(|s| s.to_string()).collect();
    let issues: Vec<String> = t.check_document(&broken, Some(&fewer)).into_iter().map(|i| i.message).collect();
    assert!(issues.iter().any(|m| m.contains("Required chapter \"Background\" is missing")), "{issues:?}");
    assert!(issues.iter().any(|m| m.contains("1 cross-reference points to something that was deleted")), "{issues:?}");
    assert!(issues.iter().any(|m| m.contains("\"smith2021\" isn't in the references")), "{issues:?}");
}

#[test]
fn no_reference_list_without_citations() {
    let mut doc = document().document;
    doc.body.content.retain(|b| !matches!(b, nawl_core::Block::Paragraph { .. }));
    assert!(!nawl_codegen::has_bibliography(&doc));
    assert!(!nawl_codegen::document_to_tex(&template(), &doc).contains("texloombibliography"));
}

#[test]
fn missing_required_fields_show_labels() {
    let mut doc = document().document;
    doc.metadata.remove("advisor");
    doc.metadata.remove("acknowledgements");
    let tex = nawl_codegen::document_to_tex(&template(), &doc);
    assert!(tex.contains("\\texloomfield{advisor}{[Advisor]}"));
    assert!(!tex.contains("acknowledgements"), "optional empty fields are left out");
    let issues = template().check_metadata(&doc.metadata);
    assert_eq!(issues.len(), 1);
    assert!(!has_errors(&issues));
}

/// Run with: NAWL_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p nawl-codegen -- --ignored
#[test]
#[ignore]
fn compiles_to_pdf() {
    let g = nawl_codegen::generate(&template(), &document().document);
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let figures: Vec<(String, Vec<u8>)> =
        g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).expect("figure exists"))).collect();
    let support = g.support_files();
    assert!(support.iter().any(|(p, _)| p == "refs.bib"));
    let files: Vec<nawl_compile::Asset> = support
        .iter()
        .chain(figures.iter())
        .map(|(p, b)| nawl_compile::Asset { path: p, bytes: b })
        .collect();
    let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(out.pdf.starts_with(b"%PDF"));
}

#[test]
fn inline_math_unnumbered_and_multiline_equations() {
    let mut doc = document().document;
    doc.body = serde_json::from_value(serde_json::json!({"type": "doc", "content": [
        {"type": "paragraph", "content": [
            {"type": "text", "text": "where "},
            {"type": "inlineMath", "attrs": {"latex": "D_t"}},
            {"type": "text", "text": " is the deficit"}
        ]},
        {"type": "equation", "attrs": {"latex": "a=b", "numbered": false, "id": "e0"}},
        {"type": "equation", "attrs": {"latex": "\\begin{aligned}x&=1\\\\y&=2\\end{aligned}", "id": "e1"}},
        {"type": "paragraph", "content": [{"type": "ref", "attrs": {"target": "e0"}}, {"type": "ref", "attrs": {"target": "e1"}}]}
    ]}))
    .unwrap();
    let tex = nawl_codegen::document_to_tex(&template(), &doc);
    assert!(tex.contains("where $D_t$ is the deficit"));
    assert!(tex.contains("\\begin{equation*}\na=b\n\\end{equation*}"));
    assert!(tex.contains("\\begin{equation}\\label{tl:e1}\n\\begin{aligned}x&=1\\\\y&=2\\end{aligned}\n\\end{equation}"));
    // A reference to an unnumbered equation has nothing to point at.
    assert!(tex.contains("\\textbf{??}\\eqref{tl:e1}"), "{tex}");
}

#[test]
fn mathlive_placeholders_are_removed() {
    let mut doc = document().document;
    doc.body = serde_json::from_value(serde_json::json!({"type": "doc", "content": [
        {"type": "equation", "attrs": {"latex": "\\begin{aligned}a & =b\\\\ \\placeholder{} & \\placeholder{x}\\end{aligned}"}}
    ]}))
    .unwrap();
    let tex = nawl_codegen::document_to_tex(&template(), &doc);
    assert!(tex.contains("\\begin{aligned}a & =b\\\\  & x\\end{aligned}"), "{tex}");
    assert!(!tex.contains("placeholder"));
}

#[test]
fn unnumbered_headings_and_raw_latex() {
    let tex = nawl_codegen::document_to_tex(&template(), &document().document);
    // Still in the table of contents and the running header, but without a number.
    assert!(
        tex.contains("\\chapter*{Team Contributions}\\addcontentsline{toc}{chapter}{Team Contributions}\\chaptermark{Team Contributions}\n"),
        "{tex}"
    );
    assert!(nawl_codegen::template_to_cls(&template()).contains("\\titleformat{name=\\chapter,numberless}[block]"));

    let mut doc = document().document;
    doc.body = serde_json::from_value(serde_json::json!({"type": "doc", "content": [
        {"type": "heading", "attrs": {"level": 2, "numbered": false, "id": "s"}, "content": [{"type": "text", "text": "Notes"}]},
        {"type": "rawLatex", "attrs": {"latex": "  \\begin{center}\\textbf{50% & more}\\end{center}\n"}},
        {"type": "rawLatex", "attrs": {"latex": "   "}}
    ]}))
    .unwrap();
    let tex = nawl_codegen::document_to_tex(&template(), &doc);
    assert!(tex.contains("\\section*{Notes}\\addcontentsline{toc}{section}{Notes}\\sectionmark{Notes}\n"), "{tex}");
    // Written exactly as typed: nothing escaped.
    assert!(tex.contains("\n\\begin{center}\\textbf{50% & more}\\end{center}\n\n\\end{document}"), "{tex}");
}

#[test]
fn pdflatex_class_uses_classic_fonts() {
    use nawl_codegen::Engine;
    let mut t = template();
    t.fonts.main = nawl_core::FontChoice::Builtin { font: nawl_core::BuiltinFont::Termes };
    let pdf = nawl_codegen::template_to_cls_for(&t, Engine::Pdf);
    assert!(!pdf.contains("fontspec"), "{pdf}");
    assert!(pdf.contains("\\RequirePackage[T1]{fontenc}") && pdf.contains("\\RequirePackage{tgtermes}"));
    let xe = nawl_codegen::template_to_cls_for(&t, Engine::Xe);
    assert!(xe.contains("\\RequirePackage{fontspec}") && xe.contains("\\setmainfont{texgyretermes}"));
    assert_eq!(nawl_codegen::template_to_cls(&t), xe, "XeTeX stays the default");
}

#[test]
fn block_lines_point_at_each_block() {
    let doc = document().document;
    let (tex, lines) = nawl_codegen::document_to_tex_mapped(&template(), &doc);
    let tex_lines: Vec<&str> = tex.lines().collect();
    assert_eq!(lines.len(), doc.body.content.len());
    assert!(lines.windows(2).all(|w| w[0] <= w[1]), "lines only go down: {lines:?}");
    // Headings start exactly on their line.
    for (b, &line) in doc.body.content.iter().zip(&lines) {
        if let nawl_core::Block::Heading { content, .. } = b {
            let text: String = content.iter().filter_map(|i| match i {
                nawl_core::Inline::Text { text, .. } => Some(text.as_str()),
                _ => None,
            }).collect();
            let first_word = text.split_whitespace().next().unwrap();
            assert!(tex_lines[line as usize - 1].contains(first_word), "line {line}: {:?} for {text:?}", tex_lines[line as usize - 1]);
        }
    }
}

/// A table whose first cell spans two rows and whose header spans two columns.
fn spanning_table_doc() -> nawl_core::Document {
    let mut doc = document().document;
    let cell = |kind: &str, text: &str, colspan: u32, rowspan: u32| {
        serde_json::json!({"type": kind, "attrs": {"colspan": colspan, "rowspan": rowspan},
            "content": [{"type": "paragraph", "content": [{"type": "text", "text": text}]}]})
    };
    let table: nawl_core::Block = serde_json::from_value(serde_json::json!({"type": "table", "content": [
        {"type": "tableRow", "content": [cell("tableHeader", "Group", 1, 2), cell("tableHeader", "Results", 2, 1)]},
        {"type": "tableRow", "content": [cell("tableCell", "Low", 1, 1), cell("tableCell", "High", 1, 1)]},
        {"type": "tableRow", "content": [cell("tableCell", "A", 1, 1), cell("tableCell", "1", 1, 1), cell("tableCell", "2", 1, 1)]}
    ]}))
    .unwrap();
    doc.body.content.push(table);
    doc
}

#[test]
fn row_spans_become_multirow() {
    let tex = nawl_codegen::document_to_tex(&template(), &spanning_table_doc());
    assert!(tex.contains(r"\usepackage{multirow}"));
    assert!(tex.contains("\\multirow{2}{=}{{\\bfseries Group}} & \\multicolumn{2}{l|}{{\\bfseries Results}} \\\\ \\cline{2-3}\n"), "{tex}");
    // The second row leaves the spanned column empty; its cells move right.
    assert!(tex.contains(" & Low & High \\\\ \\hline\n"), "{tex}");
    assert!(tex.contains("A & 1 & 2 \\\\ \\hline\n"));
    assert!(!nawl_codegen::document_to_tex(&template(), &document().document).contains("multirow"), "only when needed");
}

/// Run with: NAWL_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p nawl-codegen -- --ignored
#[test]
#[ignore]
fn row_spans_compile() {
    let g = nawl_codegen::generate(&template(), &spanning_table_doc());
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(out.pdf.starts_with(b"%PDF"));
}

/// The example's figure, cropped: 10% off the left, 20% off the top.
fn cropped_doc() -> nawl_core::Document {
    let mut doc = document().document;
    for b in &mut doc.body.content {
        if let nawl_core::Block::Figure { attrs, .. } = b {
            attrs.crop = Some(nawl_core::Crop { left: 10.0, top: 20.0, ..Default::default() });
        }
    }
    doc
}

#[test]
fn cropped_images_are_trimmed() {
    let tex = nawl_codegen::document_to_tex(&template(), &cropped_doc());
    assert!(tex.contains("\\usepackage{adjustbox}"));
    assert!(
        tex.contains("\\adjincludegraphics[trim={0.1000\\width} {0.0000\\height} {0.0000\\width} {0.2000\\height},clip,width=0.85\\linewidth]{figures/system-diagram.png}"),
        "{tex}"
    );
    let plain = nawl_codegen::document_to_tex(&template(), &document().document);
    assert!(!plain.contains("adjustbox") && !plain.contains("trim="), "only cropped images");
    // Too much cropping is limited: something always remains.
    assert_eq!(nawl_core::Crop { left: 80.0, right: -5.0, top: f32::NAN, bottom: 0.0 }.trim(), Some([0.45, 0.0, 0.0, 0.0]));
    assert_eq!(nawl_core::Crop::default().trim(), None);
}

/// Run with: NAWL_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p nawl-codegen -- --ignored
#[test]
#[ignore]
fn cropped_images_compile() {
    let g = nawl_codegen::generate(&template(), &cropped_doc());
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(out.pdf.starts_with(b"%PDF"));
}

/// Row spans and cropped images with an installed pdfLaTeX (skipped without one).
/// Run with: cargo test -p nawl-codegen --test senior_design pdflatex -- --ignored
#[test]
#[ignore]
fn spans_and_crops_compile_with_pdflatex() {
    let Some(latex) = nawl_compile::SystemLatex::find(nawl_compile::LatexProgram::PdfLatex) else {
        eprintln!("no pdfLaTeX installed");
        return;
    };
    let mut doc = cropped_doc();
    doc.body.content.extend(spanning_table_doc().body.content.pop());
    let g = nawl_codegen::generate_for(&template(), &doc, nawl_codegen::Engine::Pdf);
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    let out = latex.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(out.pdf.starts_with(b"%PDF"));
    assert!(out.synctex.is_some(), "pdfLaTeX writes SyncTeX data too");
}

/// The reference template with an approval page (made of title-page items) after
/// the title page, listed in the contents.
fn template_with_fixed_page() -> Template {
    let mut t = template();
    let at = t.front_matter.iter().position(|f| matches!(f, nawl_core::FrontMatterItem::TitlePage)).unwrap() + 1;
    let items = t.title_page.items.iter().filter(|i| !matches!(i.content, nawl_core::TitleContent::Image { .. })).take(3).cloned().collect();
    t.front_matter.insert(at, nawl_core::FrontMatterItem::FixedPage { title: "Approval Page".into(), items, in_toc: true });
    t
}

#[test]
fn fixed_pages_are_placed_like_the_title_page() {
    let t = template_with_fixed_page();
    assert!(!has_errors(&t.validate()), "{:#?}", t.validate());
    let cls = nawl_codegen::template_to_cls(&t);
    let front = &cls[cls.find("\\newcommand\\texloomfrontmatter").unwrap()..];
    let title = front.find("\\tl@titlepage").unwrap();
    let fixed = front.find("\\addcontentsline{toc}{chapter}{Approval Page}").expect("listed in the contents");
    assert!(title < fixed, "after the title page");
    assert!(front[fixed..].contains("\\begin{tikzpicture}[remember picture,overlay]"));
    // Issues on its items point at the fixed page.
    let mut broken = t.clone();
    if let nawl_core::FrontMatterItem::FixedPage { items, .. } = &mut broken.front_matter[1] {
        items[0].content = nawl_core::TitleContent::Rule { length_mm: -1.0, thickness_pt: 1.0 };
    }
    assert!(broken.validate().iter().any(|i| i.path.starts_with("frontMatter[1].items[0]")), "{:#?}", broken.validate());
}

/// Run with: NAWL_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p nawl-codegen -- --ignored
#[test]
#[ignore]
fn fixed_pages_compile() {
    let t = template_with_fixed_page();
    let doc = document().document;
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let run = |engine: nawl_codegen::Engine, latex: Option<nawl_compile::SystemLatex>| {
        let g = nawl_codegen::generate_for(&t, &doc, engine);
        let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
        let support = g.support_files();
        let files: Vec<nawl_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
        let out = match latex {
            Some(l) => l.compile(&g.tex, &files),
            None => nawl_compile::Compiler::locate(&[]).expect("tectonic").compile(&g.tex, &files),
        };
        assert!(out.unwrap_or_else(|e| panic!("{engine:?}: {e:?}")).pdf.starts_with(b"%PDF"));
    };
    run(nawl_codegen::Engine::Xe, None);
    if let Some(l) = nawl_compile::SystemLatex::find(nawl_compile::LatexProgram::PdfLatex) {
        run(nawl_codegen::Engine::Pdf, Some(l));
    }
}
