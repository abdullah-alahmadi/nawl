//! Arabic documents: right to left, Arabic font, English passages and words.

use texloom_core::{Digits, Document, Language, LanguageSettings, Template};

/// The reference report template set to Arabic.
fn arabic_template(digits: Digits) -> Template {
    let mut t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
    t.language = Some(LanguageSettings { main: Language::Arabic, other: Some(Language::English), digits });
    t.captions.figure_label = "شكل".into();
    t.captions.table_label = "جدول".into();
    t.headings.levels[0].label = "الفصل {n}".into();
    t.structure.appendix_label = "الملحق {n}".into();
    t
}

fn arabic_document() -> Document {
    let p = |text: &str| serde_json::json!({"type": "paragraph", "content": [{"type": "text", "text": text}]});
    let body = serde_json::json!({"type": "doc", "content": [
        {"type": "heading", "attrs": {"level": 1, "id": "h-intro"}, "content": [{"type": "text", "text": "مقدمة"}]},
        p("تتناول هذه الدراسة استخدام deep neural networks في التعليم، كما في نظام Windows 11 لعام 2024."),
        {"type": "paragraph", "content": [
            {"type": "text", "text": "كما ورد في الدراسات السابقة "},
            {"type": "citation", "attrs": {"keys": ["smith2021", "ahmadi2020"]}},
            {"type": "text", "text": "."}
        ]},
        {"type": "paragraph", "content": [
            {"type": "text", "text": "نموذج "},
            {"type": "text", "text": "deep ", "marks": [{"type": "italic"}]},
            {"type": "text", "text": "neural", "marks": [{"type": "bold"}]},
            {"type": "text", "text": " networks الجديد، انظر المعادلة "},
            {"type": "ref", "attrs": {"target": "eq-1"}},
            {"type": "text", "text": "."}
        ]},
        {"type": "equation", "attrs": {"latex": "a^2+b^2=c^2", "numbered": true, "id": "eq-1"}},
        {"type": "paragraph", "attrs": {"dir": "ltr"}, "content": [{"type": "text", "text": "An English paragraph inside the Arabic report, with the word كتاب in it."}]},
        {"type": "heading", "attrs": {"level": 2, "dir": "ltr"}, "content": [{"type": "text", "text": "Related Work"}]},
        {"type": "codeBlock", "content": [{"type": "text", "text": "print(\"hello\")"}]},
        {"type": "bulletList", "content": [{"type": "listItem", "content": [p("عنصر أول في القائمة")]}]},
        {"type": "tableFigure", "attrs": {"id": "t-1"}, "content": [
            {"type": "caption", "content": [{"type": "text", "text": "نتائج التجربة"}]},
            {"type": "table", "content": [
                {"type": "tableRow", "content": [
                    {"type": "tableHeader", "content": [p("الطريقة")]},
                    {"type": "tableHeader", "content": [p("الدقة")]}
                ]},
                {"type": "tableRow", "content": [
                    {"type": "tableCell", "content": [p("CNN")]},
                    {"type": "tableCell", "content": [p("95%")]}
                ]}
            ]}
        ]}
    ]});
    let mut metadata = texloom_core::Metadata::new();
    for (k, v) in [("title", "أثر التعلم العميق Deep Learning في التعليم"), ("university", "جامعة الملك عبدالعزيز"), ("department", "قسم علوم الحاسب"), ("course", "مشروع التخرج"), ("advisor", "د. أحمد علي"), ("date", "١٤٤٦هـ"), ("abstract", "ملخص البحث.")] {
        metadata.insert(k.into(), texloom_core::FieldValue::Text(v.into()));
    }
    metadata.insert("team".into(), texloom_core::FieldValue::List(vec!["عبدالله الأحمدي".into()]));
    let bibliography = r#"
@article{smith2021, author = {Smith, John and Lee, Mary}, title = {Deep Learning for Education}, journal = {IEEE Access}, volume = {9}, pages = {10--20}, year = {2021}}
@book{ahmadi2020, author = {الأحمدي, عبدالله}, title = {مقدمة في الحوسبة}, publisher = {دار المعرفة}, address = {جدة}, year = {2020}}
"#;
    Document { metadata, body: serde_json::from_value(body).unwrap(), bibliography: bibliography.into() }
}

#[test]
fn arabic_report_tex() {
    let t = arabic_template(Digits::ArabicIndic);
    assert!(!texloom_core::has_errors(&t.validate()), "{:?}", t.validate());
    let tex = texloom_codegen::document_to_tex(&t, &arabic_document());
    assert!(tex.contains("\\setdefaultlanguage[numerals=mashriq]{arabic}"), "{tex}");
    // Equation numbers at the end of the line: the left.
    assert!(tex.contains("\\PassOptionsToPackage{leqno}{amsmath}\n\\documentclass"), "{tex}");
    assert!(tex.contains("استخدام \\textenglish{deep neural networks} في"));
    assert!(tex.contains("نظام \\textenglish{Windows 11} لعام 2024"));
    // One run across italic and bold parts, so the English words keep their order.
    assert!(tex.contains("نموذج \\textenglish{\\emph{deep }\\textbf{neural} networks} الجديد"), "{tex}");
    assert!(tex.contains("\\begin{english}\nAn English paragraph inside the Arabic report, with the word \\textarabic{كتاب} in it.\n\\end{english}"));
    // Headings keep the document's layout; their English words keep their order.
    assert!(tex.contains("\\section{\\textenglish{Related Work}}"), "{tex}");
    assert!(tex.contains("\\begin{english}\n\\begin{verbatim}"));
    // Metadata: English words in the Arabic title are wrapped too.
    assert!(tex.contains("\\texloomfield{title}{أثر التعلم العميق \\textenglish{Deep Learning} في التعليم}"));
    // Cross-references use the template's Arabic words.
    assert!(tex.contains("\\eqref{tl:eq-1}"));
}

#[test]
fn english_document_with_arabic_words_gets_arabic() {
    let t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
    let mut doc: texloom_core::DocumentFile = serde_json::from_str(include_str!("../../../examples/senior-design-report.loom")).unwrap();
    let plain = texloom_codegen::document_to_tex(&t, &doc.document);
    assert!(!plain.contains("polyglossia"), "English documents stay as they were");
    assert!(!plain.contains("leqno"), "English equation numbers stay on the right");
    doc.document.body.content.push(
        serde_json::from_value(serde_json::json!({"type": "paragraph", "content": [{"type": "text", "text": "The Arabic word كتاب means book."}]})).unwrap(),
    );
    // A whole paragraph in Arabic: polyglossia's environment is `Arabic` (`\arabic`
    // is LaTeX's counter command, and `\begin{arabic}` doesn't compile).
    doc.document.body.content.push(
        serde_json::from_value(serde_json::json!({"type": "paragraph", "attrs": {"dir": "rtl"}, "content": [{"type": "text", "text": "السلام عليكم"}]})).unwrap(),
    );
    let tex = texloom_codegen::document_to_tex(&t, &doc.document);
    assert!(tex.contains("\\setdefaultlanguage{english}\n\\setotherlanguage[numerals=maghrib]{arabic}"), "{tex}");
    assert!(tex.contains("The Arabic word \\textarabic{كتاب} means book."));
    assert!(tex.contains("\\begin{Arabic}\nالسلام عليكم\n\\end{Arabic}"), "{tex}");
}

/// The thesis's defence date in both calendars, and its references as two lists:
/// Arabic first, then the others, numbered on.
#[test]
fn thesis_dates_and_split_references() {
    let (_, t, doc) = arabic_samples().into_iter().find(|(n, ..)| *n == "arabic-thesis").unwrap();
    assert!(!texloom_core::has_errors(&t.validate()), "{{date_hijri}} is a valid placeholder: {:?}", t.validate());
    let g = texloom_codegen::generate(&t, &doc);
    assert!(g.tex.contains("\\texloomfield{date}{١٤ مايو ٢٠٢٦}"), "{}", g.tex);
    // 14 May 2026 is 27 Dhu al-Qadah 1447 (Umm al-Qura).
    assert!(g.tex.contains("\\texloomfield{date_hijri}{٢٧ ذو القعدة ١٤٤٧هـ}"), "{}", g.tex);
    assert!(g.cls.contains("وأجيزت بتاريخ \\tl@f{date_hijri} الموافق \\tl@f{date}م"), "{}", g.cls);
    let arabic = g.tex.find("\\section*{المراجع العربية}").expect("Arabic list");
    let foreign = g.tex.find("\\section*{المراجع الأجنبية}").expect("foreign list");
    let ahmadi = g.tex.find("\\bibitem").unwrap();
    assert!(arabic < ahmadi && ahmadi < foreign, "Arabic entries first");
    assert!(g.tex.contains("\\setcounter{enumiv}{1}"), "numbers run on");
}

/// Title-page text is drawn in TikZ nodes, which run left to right unless told
/// otherwise: in right-to-left templates each node's text is wrapped in `\RL`.
#[test]
fn title_page_text_runs_right_to_left() {
    let cls = texloom_codegen::template_to_cls(&builtin("arabic-thesis"));
    assert!(cls.contains("{\\RL{\\tl@f{title}}};"), "{cls}");
    assert!(cls.contains("{\\RL{إعداد}};"), "fixed text too: {cls}");
    let english = texloom_codegen::template_to_cls(&builtin("senior-design"));
    assert!(!english.contains("\\RL{"), "English title pages are unchanged");
}

/// Run with: TEXLOOM_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p texloom-codegen --test arabic -- --ignored
/// Leaves the PDFs in $TEXLOOM_ARABIC_OUT when set, to look at.
#[test]
#[ignore]
fn arabic_report_compiles() {
    let compiler = texloom_compile::Compiler::locate(&[]).expect("tectonic");
    for digits in [Digits::ArabicIndic, Digits::Western] {
        let g = texloom_codegen::generate(&arabic_template(digits), &arabic_document());
        let support = g.support_files();
        let files: Vec<texloom_compile::Asset> = support.iter().map(|(p, b)| texloom_compile::Asset { path: p, bytes: b }).collect();
        let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{digits:?}: {e:?}"));
        assert!(out.pdf.starts_with(b"%PDF"));
        if let Ok(dir) = std::env::var("TEXLOOM_ARABIC_OUT") {
            let dir = std::path::Path::new(&dir);
            std::fs::write(dir.join(format!("arabic-{digits:?}.pdf")), &out.pdf).unwrap();
            std::fs::write(dir.join("main.tex"), &g.tex).unwrap();
            for (p, b) in &support {
                std::fs::write(dir.join(p), b).unwrap();
            }
        }
        let missing: Vec<&str> = out.log.lines().filter(|l| l.contains("Missing character")).collect();
        assert!(missing.is_empty(), "characters the fonts can't show ({digits:?}): {missing:#?}");
    }
}

fn builtin(name: &str) -> Template {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../templates/{name}/template.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn with_metadata(mut doc: Document, pairs: &[(&str, &str)]) -> Document {
    for (k, v) in pairs {
        doc.metadata.insert((*k).into(), texloom_core::FieldValue::Text((*v).into()));
    }
    doc
}

/// Each built-in Arabic template with a document that fills it.
fn arabic_samples() -> Vec<(&'static str, Template, Document)> {
    let thesis = with_metadata(
        arabic_document(),
        &[
            ("title", "أثر التعلم العميق في تحسين جودة التعليم الجامعي"),
            ("title_en", "The Effect of Deep Learning on University Education"),
            ("author", "عبدالله الأحمدي"),
            ("supervisor", "أ.د. محمد العتيبي"),
            ("university", "جامعة الملك عبدالعزيز"),
            ("college", "كلية الحاسبات وتقنية المعلومات"),
            ("department", "قسم علوم الحاسب"),
            ("degree", "الماجستير في علوم الحاسب"),
            ("date", "2026-05-14"),
            ("abstract", "تتناول هذه الرسالة أثر التعلم العميق في التعليم.\n\nوتوصي بمزيد من الدراسات."),
            ("abstract_en", "This thesis studies the effect of deep learning on education."),
            ("dedication", "إلى والديّ الكريمين."),
            ("acknowledgements", "الحمد لله أولًا وآخرًا."),
        ],
    );
    let book = with_metadata(
        arabic_document(),
        &[("title", "مدخل إلى علوم الحاسب"), ("subtitle", "الطبعة الأولى"), ("author", "عبدالله الأحمدي"), ("publisher", "دار المعرفة"), ("place", "جدة"), ("year", "٢٠٢٥"), ("preface", "هذا الكتاب مدخل ميسر.")],
    );
    let article: texloom_core::DocumentFile =
        serde_json::from_str(include_str!("../../../examples/arabic-article.loom")).unwrap();
    let slides = serde_json::json!({"type": "doc", "content": [
        {"type": "heading", "attrs": {"level": 1}, "content": [{"type": "text", "text": "مقدمة"}]},
        {"type": "slide", "content": [
            {"type": "slideTitle", "content": [{"type": "text", "text": "أهداف العرض"}]},
            {"type": "bulletList", "content": [
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "التعرف على machine learning"}]}]},
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "تطبيقات في التعليم"}]}]}
            ]}
        ]}
    ]});
    let mut deck = Document { metadata: Default::default(), body: serde_json::from_value(slides).unwrap(), bibliography: String::new() };
    deck = with_metadata(deck, &[("title", "التعلم الآلي في التعليم"), ("institute", "جامعة الملك عبدالعزيز")]);
    vec![
        ("arabic-thesis", builtin("arabic-thesis"), thesis),
        ("arabic-book", builtin("arabic-book"), book),
        ("arabic-article", builtin("arabic-article"), article.document),
        ("arabic-presentation", builtin("arabic-presentation"), deck),
    ]
}

#[test]
fn arabic_templates_generate() {
    for (name, t, doc) in arabic_samples() {
        assert!(!texloom_core::has_errors(&t.validate()), "{name}: {:?}", t.validate());
        let g = texloom_codegen::generate(&t, &doc);
        assert!(g.tex.contains("\\setdefaultlanguage[numerals=mashriq]{arabic}"), "{name}");
    }
    // The journal article's English section names the authors in English.
    let (_, t, doc) = arabic_samples().into_iter().find(|(n, ..)| *n == "arabic-article").unwrap();
    let tex = texloom_codegen::generate(&t, &doc).tex;
    let english = &tex[tex.find("\\begin{english}").expect("English section")..];
    assert!(english.contains("\\begin{center}\\textenglish{Abdullah Al-Ahmadi and Sarah Al-Qahtani}\\end{center}"), "{english}");
    // The thesis's English abstract page is set in English, listed in the contents.
    let cls = texloom_codegen::template_to_cls(&builtin("arabic-thesis"));
    assert!(cls.contains("\\addcontentsline{toc}{chapter}{\\textenglish{Abstract}}\\begin{english}\\chapter*{\\textenglish{Abstract}}"), "{cls}");
}

/// Run with: TEXLOOM_TECTONIC=… cargo test -p texloom-codegen --test arabic -- --ignored
#[test]
#[ignore]
fn arabic_templates_compile() {
    let compiler = texloom_compile::Compiler::locate(&[]).expect("tectonic");
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for (name, t, doc) in arabic_samples() {
        let g = texloom_codegen::generate(&t, &doc);
        let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
        let support = g.support_files();
        let files: Vec<texloom_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| texloom_compile::Asset { path: p, bytes: b }).collect();
        // Right-to-left slides need LuaLaTeX (beamer-rl): the installed one, if any.
        let out = if t.kind == texloom_core::DocumentKind::Presentation {
            let Some(lua) = texloom_compile::SystemLatex::find(texloom_compile::LatexProgram::LuaLatex) else {
                eprintln!("{name}: no LuaLaTeX installed, skipped");
                continue;
            };
            let g = texloom_codegen::generate_for(&t, &doc, texloom_codegen::Engine::Lua);
            let support = g.support_files();
            let files: Vec<texloom_compile::Asset> = support.iter().map(|(p, b)| texloom_compile::Asset { path: p, bytes: b }).collect();
            lua.compile(&g.tex, &files)
        } else {
            compiler.compile(&g.tex, &files)
        }
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        if let Ok(dir) = std::env::var("TEXLOOM_ARABIC_OUT") {
            std::fs::write(std::path::Path::new(&dir).join(format!("{name}.pdf")), &out.pdf).unwrap();
        }
        let missing: Vec<&str> = out.log.lines().filter(|l| l.contains("Missing character")).collect();
        assert!(missing.is_empty(), "{name}: characters the fonts can't show: {missing:#?}");
    }
}

#[test]
fn decimal_numbers_stay_left_to_right() {
    let (_, t, doc) = arabic_samples().into_iter().find(|(n, _, _)| *n == "arabic-article").unwrap();
    let tex = texloom_codegen::document_to_tex(&t, &doc);
    let line = tex.lines().find(|l| l.contains("54")).unwrap_or("");
    // A cell (paragraph) starting with the number: the paragraph starts first.
    assert!(line.contains(r"\leavevmode \textenglish{54.2}"), "{line}");
    assert!(tex.contains("\\textenglish{54.2}"), "{line}");
}
