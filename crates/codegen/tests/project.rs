//! Projects: a book written as one file per chapter becomes a main .tex that
//! \include's one .tex per chapter.

use serde_json::{json, Value};
use nawl_codegen::{chapter_tex_path, generate_for, generate_project_for, Engine, Generated};
use nawl_core::{Body, FieldValue, Project, ProjectFile, Template};

fn builtin(name: &str) -> Template {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../templates/{name}/template.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn p(text: &str) -> Value {
    json!({"type": "paragraph", "content": [{"type": "text", "text": text}]})
}

fn heading(level: u8, id: &str, text: &str) -> Value {
    json!({"type": "heading", "attrs": {"level": level, "id": id}, "content": [{"type": "text", "text": text}]})
}

fn body(blocks: Vec<Value>) -> Body {
    serde_json::from_value(json!({"type": "doc", "content": blocks})).unwrap()
}

/// Two chapters (the second refers to an equation and a figure in the first and cites
/// both entries), an appendix, and optionally a part before the chapters.
fn project(arabic: bool, with_part: bool) -> Project {
    let (t1, t2, t3, s) = if arabic {
        ("المقدمة", "المنهجية", "استبانة الدراسة", ["تمهيد للدراسة.", "انظر المعادلة ", " والشكل ", " كما في ", "أسئلة الاستبانة."])
    } else {
        ("Introduction", "Method", "Survey", ["Background of the study.", "See equation ", " and figure ", " as in ", "The survey questions."])
    };
    let one = body(vec![
        heading(1, "ch-intro", t1),
        p(s[0]),
        json!({"type": "equation", "attrs": {"latex": "E = mc^2", "numbered": true, "id": "eq-energy"}}),
        json!({"type": "figure", "attrs": {"src": "figures/irrigation-system.png", "width": 50, "id": "fig-system"},
               "content": [{"type": "caption", "content": [{"type": "text", "text": "System"}]}]}),
    ]);
    let two = body(vec![
        heading(1, "ch-method", t2),
        heading(2, "sec-design", "Design"),
        json!({"type": "paragraph", "content": [
            {"type": "text", "text": s[1]},
            {"type": "ref", "attrs": {"target": "eq-energy"}},
            {"type": "text", "text": s[2]},
            {"type": "ref", "attrs": {"target": "fig-system"}},
            {"type": "text", "text": s[3]},
            // An English book cites English entries only: Arabic needs XeTeX or
            // LuaTeX, and the English project also compiles with pdfLaTeX.
            {"type": "citation", "attrs": {"keys": ["smith2021", if arabic { "ahmadi2020" } else { "lee2019" }]}},
            {"type": "text", "text": "."}
        ]}),
    ]);
    let app = body(vec![heading(1, "app-survey", t3), p(s[4])]);
    let mut chapters = vec![json!({"file": "chapters/01.loom"}), json!({"file": "chapters/02.loom"}), json!({"file": "chapters/a1.loom", "appendix": true})];
    if with_part {
        chapters.insert(0, json!({"part": if arabic { "الباب الأول: الإطار العام" } else { "Part One: Foundations" }}));
    }
    let file: ProjectFile = serde_json::from_value(json!({
        "format": "texloom-project", "version": 1,
        "metadata": {"title": if arabic { "كتاب تجريبي" } else { "A Test Book" }},
        "bibliography": r#"
@article{smith2021, author = {Smith, John and Lee, Mary}, title = {Deep Learning for Education}, journal = {IEEE Access}, volume = {9}, pages = {10--20}, year = {2021}}
@book{ahmadi2020, author = {الأحمدي, عبدالله}, title = {مقدمة في الحوسبة}, publisher = {دار المعرفة}, address = {جدة}, year = {2020}}
@book{lee2019, author = {Lee, Mary}, title = {Writing Long Documents}, publisher = {Academic Press}, year = {2019}}
"#,
        "chapters": chapters
    }))
    .unwrap();
    assert!(file.validate().is_empty(), "{:?}", file.validate());
    let project = Project::load(&file, |f| -> Result<Body, ()> {
        Ok(match f {
            "chapters/01.loom" => one.clone(),
            "chapters/02.loom" => two.clone(),
            _ => app.clone(),
        })
    })
    .unwrap();
    assert!(project.validate().is_empty(), "{:?}", project.validate());
    project
}

fn position(tex: &str, needle: &str) -> usize {
    tex.find(needle).unwrap_or_else(|| panic!("{needle} not in:\n{tex}"))
}

#[test]
fn main_file_includes_one_file_per_chapter() {
    let g = generate_project_for(&builtin("arabic-book"), &project(true, true), Engine::Xe, None);
    let paths: Vec<&str> = g.chapters.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(paths, ["chapters/ch01", "chapters/ch02", "chapters/ch03"]);
    assert_eq!(chapter_tex_path(2), "chapters/ch03");

    // Main file: the part, the chapters, the reference list, \appendix, the appendix.
    let tex = &g.tex;
    let order = [
        "\\begin{document}",
        "\\texloompart{",
        "\\include{chapters/ch01}",
        "\\include{chapters/ch02}",
        "\\texloombibliography",
        "\\texloomappendix",
        "\\include{chapters/ch03}",
        "\\end{document}",
    ];
    let at: Vec<usize> = order.iter().map(|n| position(tex, n)).collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "out of order: {at:?}\n{tex}");
    assert!(tex.contains("الباب الأول: الإطار العام"), "{tex}");
    assert!(!tex.contains("\\includeonly"));
    // The chapters' text is only in their own files.
    assert!(!tex.contains("تمهيد للدراسة"));
    assert!(g.chapters[0].tex.contains("تمهيد للدراسة"));

    // A reference to another chapter is a normal \ref: LaTeX resolves it across files.
    assert!(g.chapters[0].tex.contains("\\label{tl:eq-energy}"));
    assert!(g.chapters[1].tex.contains("\\eqref{tl:eq-energy}"), "{}", g.chapters[1].tex);
    assert!(g.chapters[1].tex.contains("\\ref{tl:fig-system}"));
    assert!(!g.chapters[1].tex.contains("??"));

    // The bibliography and figures of the whole book; chapter files among the support files.
    assert!(g.bibliography.as_deref().is_some_and(|b| b.contains("smith2021")));
    assert_eq!(g.figures, ["figures/irrigation-system.png"]);
    let support: Vec<String> = g.support_files().into_iter().map(|(p, _)| p).collect();
    assert!(support.contains(&"chapters/ch02.tex".to_string()), "{support:?}");

    // Each chapter knows where its blocks start (for SyncTeX); the first after the
    // header comment.
    assert_eq!(g.chapters[1].block_lines.len(), 3);
    assert_eq!(g.chapters[1].block_lines[0], 2);
    assert!(g.chapters[1].block_lines.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn one_chapter_compiles_with_includeonly() {
    let g = generate_project_for(&builtin("arabic-book"), &project(true, false), Engine::Xe, Some(1));
    let tex = &g.tex;
    assert!(position(tex, "\\includeonly{chapters/ch02}") < position(tex, "\\begin{document}"));
    // Every chapter is still \include'd and written (LaTeX needs the full list).
    assert_eq!(tex.matches("\\include{").count(), 3);
    assert_eq!(g.chapters.len(), 3);
    // A chapter number past the end compiles everything.
    let all = generate_project_for(&builtin("arabic-book"), &project(true, false), Engine::Xe, Some(7));
    assert!(!all.tex.contains("\\includeonly"));
}

/// Without parts, the main file with each \include replaced by its chapter is the
/// same document as the chapters joined into one file: projects change where the
/// text goes, not what is written.
#[test]
fn same_latex_as_a_single_document() {
    for (template, arabic) in [("arabic-book", true), ("senior-design", false)] {
        let t = builtin(template);
        let project = project(arabic, false);
        let split = generate_project_for(&t, &project, Engine::Xe, None);
        let single = generate_for(&t, &project.combined(), Engine::Xe);
        assert_eq!(joined(&split), single.tex, "{template}");
        assert_eq!(split.cls, single.cls);
        assert_eq!(split.figures, single.figures);
    }
}

fn joined(g: &Generated) -> String {
    // The one line a project adds: the appendix note for the contents, written at
    // once (see tex.rs).
    let mut tex: String = g.tex.split_inclusive('\n').filter(|l| !l.starts_with("\\makeatletter\\ifdefined\\tl@appendixtoc")).collect();
    tex = tex.replace("\\texloomappendix\n\n\n", "\\texloomappendix\n\n");
    for c in &g.chapters {
        let content = c.tex.split_once('\n').unwrap().1;
        tex = tex.replace(&format!("\\include{{{}}}\n\n", c.path), content);
    }
    tex
}

/// The template's part style: "الباب الأول" in words, its own page, a contents line;
/// without one, the base class's \part.
#[test]
fn part_style_in_the_class() {
    let arabic = nawl_codegen::template_to_cls(&builtin("arabic-book"));
    let line = arabic.lines().find(|l| l.starts_with("\\newcommand\\texloompart")).expect("defined");
    assert!(line.contains("\\cleardoublepage") && line.contains("\\addcontentsline{toc}{part}"), "{line}");
    assert!(arabic.contains("الأول\\or") && arabic.contains("العشرون"), "Arabic ordinals");
    assert!(line.contains("\\tl@partword{\\number\\value{tl@part}}"));

    let mut english = builtin("senior-design");
    assert!(nawl_codegen::template_to_cls(&english).contains("\\newcommand\\texloompart[1]{\\part{#1}}"));
    english.headings.part = Some(nawl_core::PartStyle {
        text: english.headings.levels[0].text.clone(),
        label: "Part {n}".into(),
        number_words: true,
        own_page: false,
        in_toc: false,
        label_gap_pt: 6.0,
    });
    assert!(english.validate().is_empty(), "{:?}", english.validate());
    let cls = nawl_codegen::template_to_cls(&english);
    assert!(cls.contains("One\\or Two"), "English numbers");
    let line = cls.lines().find(|l| l.starts_with("\\newcommand\\texloompart")).unwrap();
    assert!(!line.contains("cleardoublepage") && !line.contains("addcontentsline"), "{line}");
}

#[test]
fn project_metadata_fills_the_title_page() {
    let mut project = project(false, false);
    project.metadata.insert("author".into(), FieldValue::Text("Abdullah".into()));
    let g = generate_project_for(&builtin("senior-design"), &project, Engine::Xe, None);
    assert!(g.tex.contains("\\texloomfield{title}{A Test Book}"), "{}", g.tex);
}

/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test project -- --ignored
#[test]
#[ignore]
fn projects_compile_with_tectonic() {
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    for (template, arabic) in [("arabic-book", true), ("senior-design", false)] {
        let g = generate_project_for(&builtin(template), &project(arabic, true), Engine::Xe, None);
        let out = compile(&g, |tex, files| compiler.compile(tex, files));
        assert!(out.pdf.starts_with(b"%PDF"), "{template}");
        // To look at them: NAWL_SAVE_PDF=<folder>.
        if let Some(dir) = std::env::var_os("NAWL_SAVE_PDF") {
            std::fs::write(std::path::Path::new(&dir).join(format!("project-{template}.pdf")), &out.pdf).unwrap();
        }
        assert_resolved(&out.log, template);
    }
}

/// A one-chapter compile in the folder of a full compile (Tectonic keeping its
/// intermediate files with -k): only that chapter is typeset, and its references to
/// other chapters still resolve, from their .aux files.
/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test project -- --ignored
#[test]
#[ignore]
fn one_chapter_compile_keeps_other_chapters_numbers() {
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    let dir = tempfile::tempdir().unwrap();
    let run = |only: Option<usize>| -> (nawl_compile::CompileOutput, Vec<String>) {
        let g = generate_project_for(&builtin("arabic-book"), &project(true, true), Engine::Xe, only);
        let out = compile(&g, |tex, files| compiler.compile_in(dir.path(), tex, files));
        (out, g.chapters.iter().map(|c| format!("{}.tex", c.path)).collect())
    };
    let (full, paths) = run(None);
    assert!(full.log.contains("chapters/ch01.tex") && full.log.contains("chapters/ch02.tex"));
    assert!(dir.path().join("chapters/ch01.aux").is_file(), "the build folder keeps each chapter's .aux");
    // SyncTeX knows the chapter files: some place in the PDF comes from chapter 2.
    let sync = nawl_compile::SyncTex::read_with(dir.path(), "main", &paths).expect("synctex");
    let from_chapter_two = (1..=12).any(|page| (0..20).any(|i| sync.place_at(page, 300.0, 40.0 * i as f32).is_some_and(|(f, _)| f == 2)));
    assert!(from_chapter_two);

    let (one, _) = run(Some(1));
    assert!(!one.log.contains("chapters/ch01.tex"), "chapter 1 isn't typeset");
    assert!(one.log.contains("chapters/ch02.tex"));
    let problems: Vec<&str> = one.log.lines().filter(|l| l.contains("undefined")).collect();
    assert!(problems.is_empty(), "chapter 2's references to chapter 1: {problems:#?}");
    assert!(one.pdf.len() < full.pdf.len(), "fewer pages");
}

/// Run with: cargo test -p nawl-codegen --test project pdflatex -- --ignored
#[test]
#[ignore]
fn english_project_compiles_with_pdflatex() {
    let Some(latex) = nawl_compile::SystemLatex::find(nawl_compile::LatexProgram::PdfLatex) else {
        eprintln!("no pdfLaTeX installed");
        return;
    };
    let g = generate_project_for(&builtin("senior-design"), &project(false, true), Engine::Pdf, None);
    let out = compile(&g, |tex, files| latex.compile(tex, files));
    assert!(out.pdf.starts_with(b"%PDF"));
    assert_resolved(&out.log, "senior-design (pdfLaTeX)");
}

fn compile(
    g: &Generated,
    run: impl FnOnce(&str, &[nawl_compile::Asset]) -> Result<nawl_compile::CompileOutput, nawl_compile::CompileError>,
) -> nawl_compile::CompileOutput {
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    run(&g.tex, &files).unwrap_or_else(|e| panic!("{e:?}"))
}

/// References between chapters and citations resolved; no missing glyphs.
fn assert_resolved(log: &str, what: &str) {
    let problems: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("undefined") || l.contains("Missing character") || l.contains("No file chapters/"))
        .collect();
    assert!(problems.is_empty(), "{what}: {problems:#?}");
}
