//! Arabic documents: right to left, Arabic font, English passages and words.

use nawl_core::{Digits, Document, Language, LanguageSettings, Template};

/// The reference report template set to Arabic.
fn arabic_template(digits: Digits) -> Template {
    let mut t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
    t.language = Some(LanguageSettings { main: Language::Arabic, other: Some(Language::English), digits, math: Default::default() });
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
    let mut metadata = nawl_core::Metadata::new();
    for (k, v) in [("title", "أثر التعلم العميق Deep Learning في التعليم"), ("university", "جامعة المعرفة"), ("department", "قسم علوم الحاسب"), ("course", "مشروع التخرج"), ("advisor", "د. أحمد علي"), ("date", "١٤٤٦هـ"), ("abstract", "ملخص البحث.")] {
        metadata.insert(k.into(), nawl_core::FieldValue::Text(v.into()));
    }
    metadata.insert("team".into(), nawl_core::FieldValue::List(vec!["عبدالله الأحمدي".into()]));
    let bibliography = r#"
@article{smith2021, author = {Smith, John and Lee, Mary}, title = {Deep Learning for Education}, journal = {IEEE Access}, volume = {9}, pages = {10--20}, year = {2021}}
@book{ahmadi2020, author = {الأحمدي, عبدالله}, title = {مقدمة في الحوسبة}, publisher = {دار المعرفة}, address = {جدة}, year = {2020}}
"#;
    Document { metadata, body: serde_json::from_value(body).unwrap(), bibliography: bibliography.into() }
}

/// A paragraph's own alignment (the signature of an official letter on the left):
/// left and right as seen on the page, in either direction; table cells keep theirs.
#[test]
fn aligned_paragraphs() {
    let p = |text: &str, align: &str| serde_json::json!({"type": "paragraph", "attrs": {"align": align}, "content": [{"type": "text", "text": text}]});
    let cell = serde_json::json!({"type": "tableCell", "content": [p("خلية", "center")]});
    let body = serde_json::json!({"type": "doc", "content": [
        p("مدير الجامعة", "left"), p("والله ولي التوفيق", "center"), p("يمين", "right"),
        {"type": "paragraph", "attrs": {"align": "right", "dir": "ltr"}, "content": [{"type": "text", "text": "English"}]},
        {"type": "paragraph", "attrs": {"align": "left"}},
        {"type": "table", "content": [{"type": "tableRow", "content": [cell]}]},
    ]});
    let doc = Document { metadata: Default::default(), body: serde_json::from_value(body).unwrap(), bibliography: String::new() };
    let tex = nawl_codegen::document_to_tex(&builtin("arabic-admin-order"), &doc);
    assert!(tex.contains("{\\raggedright مدير الجامعة\\par}\n\n{\\centering والله ولي التوفيق\\par}\n\n{\\raggedleft يمين\\par}\n\n"), "{tex}");
    assert!(tex.contains("\\begin{english}\n{\\raggedleft English\\par}\n\\end{english}"), "{tex}");
    assert_eq!(tex.matches("\\raggedright").count(), 1, "an empty paragraph writes nothing: {tex}");
    assert_eq!(tex.matches("\\centering").count(), 1, "not in a table cell: {tex}");
}

#[test]
fn arabic_report_tex() {
    let t = arabic_template(Digits::ArabicIndic);
    assert!(!nawl_core::has_errors(&t.validate()), "{:?}", t.validate());
    let tex = nawl_codegen::document_to_tex(&t, &arabic_document());
    assert!(tex.contains("\\setdefaultlanguage[numerals=mashriq]{arabic}"), "{tex}");
    // Equation numbers at the end of the line: the left.
    assert!(tex.contains("\\PassOptionsToPackage{leqno}{amsmath}\n\\documentclass"), "{tex}");
    // The dot in "١.١" comes from the Latin font: Amiri's Arabic full stop looks like
    // the zero ("١٠١"). Western digits keep the font's own dot.
    assert!(tex.contains("\\DeclareRobustCommand\\tlnumdot{{\\englishfont .}}\n\\providecommand\\@SepMark{.}\\renewcommand\\@SepMark{\\tlnumdot}"), "{tex}");
    assert!(!nawl_codegen::document_to_tex(&arabic_template(Digits::Western), &arabic_document()).contains("\\tlnumdot"));
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

/// The English sample article citing an Arabic source as well (no Arabic in its text).
fn english_article_citing_arabic() -> (Template, Document) {
    let t = builtin("general-article");
    let mut doc: nawl_core::DocumentFile = serde_json::from_str(include_str!("../../../examples/article.nawl")).unwrap();
    doc.document.bibliography.push_str(
        "@article{alghamdi2021, author = {الغامدي, خالد and الزهراني, فهد}, title = {التنبؤ بالطلب على الطاقة الكهربائية في المنطقة الغربية باستخدام نماذج السلاسل الزمنية}, journal = {المجلة العربية للهندسة الكهربائية}, volume = {9}, pages = {77--98}, year = {2021}}\n",
    );
    let mut body = serde_json::to_value(&doc.document.body).unwrap();
    let text = body.to_string().replacen(r#"{"keys":["doe2021"]}"#, r#"{"keys":["doe2021","alghamdi2021"]}"#, 1);
    body = serde_json::from_str(&text).unwrap();
    doc.document.body = serde_json::from_value(body).unwrap();
    (t, doc.document)
}

/// An English document citing an Arabic source gets Arabic (its font and direction),
/// though its own text has none: else the entry would print as missing characters.
/// The entry is a right-to-left paragraph in the left-to-right list.
#[test]
fn english_document_citing_arabic_gets_arabic() {
    let (t, doc) = english_article_citing_arabic();
    let tex = nawl_codegen::document_to_tex(&t, &doc);
    assert!(tex.contains("\\setotherlanguage[numerals=maghrib]{arabic}"), "{tex}");
    assert!(tex.contains("\\cite{doe2021,alghamdi2021}"), "{tex}");
    assert!(tex.contains("\\bibitem{alghamdi2021} \\tl@rtlentry{خالد الغامدي"), "{tex}");
}

/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test arabic -- --ignored
#[test]
#[ignore]
fn english_document_citing_arabic_compiles() {
    let (t, doc) = english_article_citing_arabic();
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    let g = nawl_codegen::generate(&t, &doc);
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    let out = compiler.compile(&g.tex, &files).unwrap();
    let missing: Vec<&str> = out.log.lines().filter(|l| l.contains("Missing character")).collect();
    assert!(missing.is_empty(), "{missing:#?}");
}

/// Aref Ruqaa has no Arabic decimal or thousands separator: the 9-page sample (with
/// "٢٫٣١٪" in its abstract and decimals in its formulas) loses none of them.
/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test arabic -- --ignored
#[test]
#[ignore]
fn fonts_without_arabic_separators_compile() {
    let mut t = builtin("arabic-article");
    t.fonts.arabic = Some(nawl_core::FontChoice::Library { id: "aref-ruqaa".into() });
    let doc: nawl_core::DocumentFile = serde_json::from_str(&example_file("arabic-load-forecasting.nawl")).unwrap();
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    let g = nawl_codegen::generate(&t, &doc.document);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut extra: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(root.join("examples").join(f)).unwrap())).collect();
    // The library's font files, fetched into src-tauri/fonts by scripts/fetch-fonts.mjs.
    extra.extend(g.fonts.iter().map(|f| (f.clone(), std::fs::read(root.join("src-tauri").join(f)).unwrap())));
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(extra.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    let out = compiler.compile(&g.tex, &files).unwrap();
    let lost: Vec<&str> = out.log.lines().filter(|l| l.contains("Missing character") && (l.contains('٫') || l.contains('٬') || l.contains('٪'))).collect();
    assert!(lost.is_empty(), "{lost:#?}");
}

/// A document with Arabic formulas of every kind: inline, numbered, multi-line,
/// cases, a matrix, and one formula kept international.
fn arabic_math_document() -> (Template, Document) {
    let mut t = builtin("arabic-article");
    t.language.as_mut().unwrap().math = nawl_core::MathStyle::Arabic;
    let eq = |latex: &str| serde_json::json!({"type": "equation", "attrs": {"latex": latex, "numbered": true, "id": null}});
    let body = serde_json::json!({"type": "doc", "content": [
        {"type": "heading", "attrs": {"level": 1}, "content": [{"type": "text", "text": "الرياضيات"}]},
        {"type": "paragraph", "content": [
            {"type": "text", "text": "نظرية فيثاغورس: "},
            {"type": "inlineMath", "attrs": {"latex": "س^2+ص^2=ع^2"}},
            {"type": "text", "text": " والنسبة التقريبية "},
            {"type": "inlineMath", "attrs": {"latex": "\\pi\\approx 3.14"}},
            {"type": "text", "text": "."}
        ]},
        eq("\\int_0^1 س^2\\,دس=\\frac{1}{3}"),
        eq("\\lim_{س\\to\\infty}\\frac{1}{س}=0"),
        eq("\\sum_{ك=1}^{ن}ك=\\frac{ن(ن+1)}{2}"),
        eq("\\sqrt{س^2+1}<ص,\\qquad \\sin^2 س+\\cos^2 س=1"),
        eq("د(س)=\\begin{cases}س&س\\ge 0\\\\-س&س<0\\end{cases}"),
        eq("\\begin{aligned}(س+ص)^2&=س^2+2سص+ص^2\\\\(س-ص)^2&=س^2-2سص+ص^2\\end{aligned}"),
        eq("\\begin{pmatrix}1&2\\\\3&4\\end{pmatrix}\\begin{pmatrix}س\\\\ص\\end{pmatrix}=\\begin{pmatrix}5\\\\6\\end{pmatrix}"),
        {"type": "equation", "attrs": {"latex": "E=mc^2", "numbered": true, "id": null, "style": "international"}},
    ]});
    let mut doc = arabic_document();
    doc.body = serde_json::from_value(body).unwrap();
    doc.bibliography = String::new();
    (t, doc)
}

/// Arabic formulas are mirrored as a whole, their letters and numbers mirrored
/// back; the variables' font (Aref Ruqaa) goes with the document.
#[test]
fn arabic_math_is_mirrored() {
    let (t, doc) = arabic_math_document();
    assert!(!nawl_core::has_errors(&t.validate()), "{:?}", t.validate());
    let g = nawl_codegen::generate(&t, &doc);
    assert!(g.tex.contains("\\let\\tlmathfont\\tlfontarefruqaa"), "{}", g.tex);
    assert!(g.cls.contains("\\newfontfamily\\tlfontarefruqaa[Script=Arabic,"), "{}", g.cls);
    assert!(g.fonts.iter().any(|f| f == "fonts/aref-ruqaa-Regular.ttf"), "{:?}", g.fonts);
    assert!(g.tex.contains("\\tlarmath{{\\tla{س}}^{\\tlw{٢}}+{\\tla{ص}}^{\\tlw{٢}}={\\tla{ع}}^{\\tlw{٢}}}"), "{}", g.tex);
    assert!(g.tex.contains("\\begin{equation}\n\\tlardisplay{\\int_{\\tlw{٠}}^{\\tlw{١}} {\\tla{س}}^{\\tlw{٢}}\\,{\\tla{د}}{\\tla{س}}=\\frac{{\\tlw{١}}}{{\\tlw{٣}}}}\n\\end{equation}"), "{}", g.tex);
    // A formula can keep the international way.
    assert!(g.tex.contains("\\begin{equation}\nE=mc^{\\text{٢}}\n\\end{equation}"), "{}", g.tex);
    // Templates without it are unchanged, down to the fonts.
    let plain = nawl_codegen::generate(&builtin("arabic-article"), &doc);
    assert!(!plain.tex.contains("\\tlarmath{") && plain.fonts.is_empty(), "{:?}", plain.fonts);
}

/// Arabic formulas compile (Tectonic), with every letter there.
/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test arabic arabic_math -- --ignored
#[test]
#[ignore]
fn arabic_math_compiles() {
    let (t, doc) = arabic_math_document();
    let g = nawl_codegen::generate(&t, &doc);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fonts: Vec<(String, Vec<u8>)> = g.fonts.iter().map(|f| (f.clone(), std::fs::read(root.join("src-tauri").join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(fonts.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    let out = nawl_compile::Compiler::locate(&[]).expect("tectonic").compile(&g.tex, &files).unwrap();
    assert!(out.pdf.starts_with(b"%PDF"));
    if let Ok(dir) = std::env::var("NAWL_ARABIC_OUT") {
        std::fs::write(std::path::Path::new(&dir).join("arabic-math.pdf"), &out.pdf).unwrap();
    }
    let missing: Vec<&str> = out.log.lines().filter(|l| l.contains("Missing character")).collect();
    assert!(missing.is_empty(), "{missing:#?}");
}

/// The Arabic mathematics example with the template it carries (the Arabic article
/// with formulas the Arabic way).
fn arabic_math_example() -> (Template, Document) {
    let file: serde_json::Value = serde_json::from_str(&example_file("arabic-math.nawl")).unwrap();
    let template: Template = serde_json::from_value(file["template"].clone()).unwrap();
    let doc: nawl_core::DocumentFile = serde_json::from_value(file).unwrap();
    (template, doc.document)
}

/// The example is complete: a valid template that differs from the built-in article
/// only in how formulas are written, no warnings, every kind of formula.
#[test]
fn arabic_math_example_is_complete() {
    let (t, doc) = arabic_math_example();
    assert!(t.validate().is_empty(), "{:?}", t.validate());
    let mut plain = t.clone();
    plain.language.as_mut().unwrap().math = Default::default();
    plain.meta = builtin("arabic-article").meta;
    assert_eq!(plain, builtin("arabic-article"), "run scripts/embed-example-templates.mjs");
    assert!(t.check_document(&doc, None).is_empty(), "{:?}", t.check_document(&doc, None));
    let tex = nawl_codegen::document_to_tex(&t, &doc);
    for piece in ["\\mathop{\\tlw{نهـــا}}_", "\\mathop{\\tlw{جا}}\\nolimits", "\\begin{aligned}", "\\begin{cases}", "\\begin{pmatrix}", "\\sum_", "\\int_", "{\\tlw{إذا كان }}"] {
        assert!(tex.contains(piece), "{piece}: {tex}");
    }
    // One formula keeps the international way.
    assert!(tex.contains("\\begin{equation}\\label{tl:eq-einstein}\nE=mc^{\\text{٢}}\n\\end{equation}"), "{tex}");
}

/// The example compiles, with every letter there and its references resolved.
/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test arabic arabic_math_example -- --ignored
#[test]
#[ignore]
fn arabic_math_example_compiles() {
    let (t, doc) = arabic_math_example();
    let g = nawl_codegen::generate(&t, &doc);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fonts: Vec<(String, Vec<u8>)> = g.fonts.iter().map(|f| (f.clone(), std::fs::read(root.join("src-tauri").join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(fonts.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    let out = nawl_compile::Compiler::locate(&[]).expect("tectonic").compile(&g.tex, &files).unwrap();
    assert!(out.pdf.starts_with(b"%PDF"));
    let problems: Vec<&str> = out.log.lines().filter(|l| l.contains("Missing character") || l.contains("undefined")).collect();
    assert!(problems.is_empty(), "{problems:#?}");
}

/// Pictures of formulas for Word: one page per formula, exactly its size, with the
/// template's own setup; sizes are read from the log.
#[test]
fn formula_pictures_document() {
    let (t, _) = arabic_math_document();
    let formulas = vec![("س^2".to_string(), false), ("\\int_0^1 س\\,دس".to_string(), true)];
    let g = nawl_codegen::formula_pictures(&t, &formulas, nawl_codegen::Engine::Xe);
    assert!(g.tex.contains("\\let\\tlmathfont\\tlfontarefruqaa"), "the template's setup: {}", g.tex);
    assert_eq!(g.tex.matches("\\begin{document}").count(), 1, "{}", g.tex);
    assert!(g.tex.contains("\\tlformula{\\tlarmath{{\\tla{س}}^{\\tlw{٢}}}}\n\\tlformula{\\tlardisplay{\\int_{\\tlw{٠}}^{\\tlw{١}} {\\tla{س}}\\,{\\tla{د}}{\\tla{س}}}}\n\\end{document}"), "{}", g.tex);
    assert!(g.fonts.iter().any(|f| f == "fonts/aref-ruqaa-Regular.ttf"));
    // Width, height and depth with the 2pt padding (the width has it already).
    let sizes = nawl_codegen::formula_picture_sizes("noise\nTLF:79.78139pt:8.80933pt:4.40399pt\n TLF:82.5pt:17pt:10pt\nTLF:broken\n");
    assert_eq!(sizes, vec![[79.78139, 10.80933, 6.40399], [82.5, 19.0, 12.0]]);
}

/// The formula pages compile: as many pages as formulas, each with its size.
/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test arabic formula_pictures -- --ignored
#[test]
#[ignore]
fn formula_pictures_compile() {
    let (t, _) = arabic_math_document();
    let formulas = vec![
        ("س^2+ص^2=ع^2".to_string(), false),
        ("\\int_0^1 س^2\\,دس=\\frac{1}{3}".to_string(), true),
        ("\\lim_{س\\to\\infty}\\frac{1}{س}=0".to_string(), true),
    ];
    let g = nawl_codegen::formula_pictures(&t, &formulas, nawl_codegen::Engine::Xe);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fonts: Vec<(String, Vec<u8>)> = g.fonts.iter().map(|f| (f.clone(), std::fs::read(root.join("src-tauri").join(f)).unwrap())).collect();
    let support = g.support_files();
    let files: Vec<nawl_compile::Asset> = support.iter().chain(fonts.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
    let out = nawl_compile::Compiler::locate(&[]).expect("tectonic").compile(&g.tex, &files).unwrap();
    if let Ok(dir) = std::env::var("NAWL_ARABIC_OUT") {
        std::fs::write(std::path::Path::new(&dir).join("formula-pictures.pdf"), &out.pdf).unwrap();
    }
    let sizes = nawl_codegen::formula_picture_sizes(&out.log);
    assert_eq!(sizes.len(), 3, "{}", out.log);
    assert!(sizes.iter().all(|[w, h, d]| *w > 20.0 && *h > 5.0 && *d > 2.0), "{sizes:?}");
    // The display integral is taller than the inline formula.
    assert!(sizes[1][1] + sizes[1][2] > sizes[0][1] + sizes[0][2] + 5.0, "{sizes:?}");
    assert!(out.pdf.starts_with(b"%PDF"));
}

#[test]
fn english_document_with_arabic_words_gets_arabic() {
    let t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
    let mut doc: nawl_core::DocumentFile = serde_json::from_str(include_str!("../../../examples/senior-design-report.nawl")).unwrap();
    let plain = nawl_codegen::document_to_tex(&t, &doc.document);
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
    let tex = nawl_codegen::document_to_tex(&t, &doc.document);
    assert!(tex.contains("\\setdefaultlanguage{english}\n\\setotherlanguage[numerals=maghrib]{arabic}"), "{tex}");
    assert!(tex.contains("The Arabic word \\textarabic{كتاب} means book."));
    assert!(tex.contains("\\begin{Arabic}\nالسلام عليكم\n\\end{Arabic}"), "{tex}");
}

/// Appendices are listed in the contents with their label ("الملحق ا"), not a bare
/// letter that looks like the digit "١"; and in two-sided documents the page left
/// blank before a chapter has no header or page number.
#[test]
fn appendix_contents_and_blank_pages() {
    let cls = nawl_codegen::template_to_cls(&builtin("arabic-thesis"));
    assert!(cls.contains("\\newcommand\\tl@appendixtoc{\\let\\tl@ltop\\l@chapter\\def\\l@chapter##1##2{\\begingroup\\def\\numberline####1{\\mbox{الملحق ####1}\\enspace}"), "{cls}");
    assert!(cls.contains("\\appendix\\addtocontents{toc}{\\protect\\tl@appendixtoc}"), "{cls}");
    let book = nawl_codegen::template_to_cls(&builtin("arabic-book"));
    assert!(book.contains("\\renewcommand\\cleardoublepage{\\clearpage\\if@twoside\\ifodd\\c@page\\else\\hbox{}\\thispagestyle{empty}"), "{book}");
    // One-sided documents leave \cleardoublepage alone.
    assert!(!nawl_codegen::template_to_cls(&builtin("arabic-article")).contains("\\renewcommand\\cleardoublepage"));
}

/// polyglossia's Arabic drops the chapter from equation numbers ("(١)" beside
/// "شكل ١.١"): with per-chapter numbering it goes back, also when polyglossia sets
/// the numbers again. Without it (or without chapters), polyglossia's is right.
#[test]
fn arabic_equations_numbered_per_chapter() {
    let fix = "\\gappto\\arabic@globalnumbers{\\tl@chapterequation}";
    let book = builtin("arabic-book");
    assert!(book.structure.per_chapter_numbering);
    let tex = nawl_codegen::document_to_tex(&book, &arabic_document());
    assert!(tex.contains(fix) && tex.contains("\\renewcommand\\theequation{\\ifnum\\c@chapter>\\z@\\thechapter\\@SepMark\\fi\\@arabic\\c@equation}"), "{tex}");
    let mut whole = book.clone();
    whole.structure.per_chapter_numbering = false;
    assert!(!nawl_codegen::document_to_tex(&whole, &arabic_document()).contains(fix));
    assert!(!nawl_codegen::document_to_tex(&builtin("arabic-article"), &arabic_document()).contains(fix));
}

/// LuaLaTeX gets numbers in right-to-left text in \LR (it would set their digits
/// backwards); XeTeX, which orders them itself, gets the text as it is.
#[test]
fn numbers_in_right_to_left_text_for_lualatex() {
    let (t, doc) = english_article_citing_arabic();
    let lua = nawl_codegen::generate_for(&t, &doc, nawl_codegen::Engine::Lua).tex;
    assert!(lua.contains("\\LR{2021}"), "{lua}");
    assert!(lua.contains("\\AtBeginDocument{\\providecommand\\LR[1]{#1}}"));
    let xe = nawl_codegen::generate(&t, &doc).tex;
    assert!(!xe.contains("\\LR{"), "{xe}");
    // A number with separators is one unit: "١٨(٢):٣٣–٥٢".
    let (_, t, doc) = arabic_samples().into_iter().find(|(n, ..)| *n == "arabic-thesis").unwrap();
    let mut doc = doc;
    doc.body = serde_json::from_value(serde_json::json!({"type": "doc", "content": [
        {"type": "paragraph", "content": [{"type": "text", "text": "المجلد ١٨(٢):٣٣–٥٢، سنة ٢٠٢١."}]}
    ]}))
    .unwrap();
    let lua = nawl_codegen::generate_for(&t, &doc, nawl_codegen::Engine::Lua).tex;
    assert!(lua.contains("المجلد \\LR{١٨(٢):٣٣--٥٢}، سنة \\LR{٢٠٢١}."), "{lua}");
}

/// The thesis's defence date in both calendars, and its references as two lists:
/// Arabic first, then the others, numbered on.
#[test]
fn thesis_dates_and_split_references() {
    let (_, t, doc) = arabic_samples().into_iter().find(|(n, ..)| *n == "arabic-thesis").unwrap();
    assert!(!nawl_core::has_errors(&t.validate()), "{{date_hijri}} is a valid placeholder: {:?}", t.validate());
    let g = nawl_codegen::generate(&t, &doc);
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

/// A font from Nawl's library is loaded by file from the build folder's fonts/, and
/// its files are listed to go there.
#[test]
fn library_fonts_go_with_the_document() {
    let (_, mut t, doc) = arabic_samples().into_iter().find(|(n, ..)| *n == "arabic-thesis").unwrap();
    t.fonts.arabic = Some(nawl_core::FontChoice::Library { id: "cairo".into() });
    assert!(!nawl_core::has_errors(&t.validate()), "{:?}", t.validate());
    let g = nawl_codegen::generate(&t, &doc);
    assert!(g.tex.contains("\\setmainfont[Script=Arabic,Path=fonts/,BoldFont=cairo-Bold.ttf]{cairo-Regular.ttf}"), "{}", g.tex);
    assert!(g.tex.contains("\\newfontfamily\\arabicfonttt[Script=Arabic,Ligatures=TeX,Path=fonts/,BoldFont=cairo-Bold.ttf]{cairo-Regular.ttf}"));
    assert_eq!(g.fonts, ["fonts/cairo-Regular.ttf", "fonts/cairo-Bold.ttf"]);
    // A family without a bold: an emboldened regular.
    t.fonts.arabic = Some(nawl_core::FontChoice::Library { id: "lalezar".into() });
    assert!(nawl_codegen::generate(&t, &doc).tex.contains("[Script=Arabic,Path=fonts/,AutoFakeBold=1.5]{lalezar-Regular.ttf}"));
    // An id the library doesn't have is a template error.
    t.fonts.arabic = Some(nawl_core::FontChoice::Library { id: "nope".into() });
    assert!(t.validate().iter().any(|i| i.path == "fonts.arabic.id"));
}

/// Heading levels and title-page items can have a font of their own: a family command
/// in the class (shaped as Arabic here), used by their format; its files are sent.
#[test]
fn headings_and_titles_have_their_own_fonts() {
    let (_, mut t, doc) = arabic_samples().into_iter().find(|(n, ..)| *n == "arabic-thesis").unwrap();
    t.headings.levels[0].text.font = Some(nawl_core::FontChoice::Library { id: "reem-kufi".into() });
    if let nawl_core::TitleContent::Text { style, .. } = &mut t.title_page.items[0].content {
        style.font = Some(nawl_core::FontChoice::Library { id: "aref-ruqaa".into() });
    }
    assert!(!nawl_core::has_errors(&t.validate()), "{:?}", t.validate());
    let g = nawl_codegen::generate(&t, &doc);
    assert!(g.cls.contains("\\newfontfamily\\tlfontreemkufi[Script=Arabic,Ligatures=TeX,Path=fonts/,BoldFont=reem-kufi-Bold.ttf]{reem-kufi-Regular.ttf}"), "{}", g.cls);
    assert!(g.cls.contains("\\tlfontreemkufi\\fontsize"), "the chapter format uses it");
    assert!(g.cls.contains("\\tlfontarefruqaa\\fontsize"), "the title item uses it");
    assert!(g.fonts.contains(&"fonts/reem-kufi-Bold.ttf".to_string()) && g.fonts.contains(&"fonts/aref-ruqaa-Regular.ttf".to_string()));
    // pdfLaTeX can't load font files: the commands do nothing.
    let pdf = nawl_codegen::template_to_cls_for(&t, nawl_codegen::Engine::Pdf);
    assert!(pdf.contains("\\providecommand\\tlfontreemkufi{}"), "{pdf}");
    // A heading level's unknown library font is a template error at its own path.
    t.headings.levels[1].text.font = Some(nawl_core::FontChoice::Library { id: "nope".into() });
    assert!(t.validate().iter().any(|i| i.path == "headings.levels[1].text.font"));
}

/// Title-page text is drawn in TikZ nodes, which run left to right unless told
/// otherwise: in right-to-left templates each node's text is wrapped in `\RL`.
#[test]
fn title_page_text_runs_right_to_left() {
    let cls = nawl_codegen::template_to_cls(&builtin("arabic-thesis"));
    assert!(cls.contains("{\\RL{\\tl@f{title}}};"), "{cls}");
    assert!(cls.contains("{\\RL{إعداد}};"), "fixed text too: {cls}");
    let english = nawl_codegen::template_to_cls(&builtin("senior-design"));
    assert!(!english.contains("\\RL{"), "English title pages are unchanged");
}

/// Run with: NAWL_TECTONIC=src-tauri/binaries/nawl-tectonic-<triple> cargo test -p nawl-codegen --test arabic -- --ignored
/// Leaves the PDFs in $NAWL_ARABIC_OUT when set, to look at.
#[test]
#[ignore]
fn arabic_report_compiles() {
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    for digits in [Digits::ArabicIndic, Digits::Western] {
        let g = nawl_codegen::generate(&arabic_template(digits), &arabic_document());
        let support = g.support_files();
        let files: Vec<nawl_compile::Asset> = support.iter().map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
        let out = compiler.compile(&g.tex, &files).unwrap_or_else(|e| panic!("{digits:?}: {e:?}"));
        assert!(out.pdf.starts_with(b"%PDF"));
        if let Ok(dir) = std::env::var("NAWL_ARABIC_OUT") {
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
        doc.metadata.insert((*k).into(), nawl_core::FieldValue::Text((*v).into()));
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
            ("university", "جامعة المعرفة"),
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
    let article: nawl_core::DocumentFile =
        serde_json::from_str(include_str!("../../../examples/arabic-article.nawl")).unwrap();
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
    deck = with_metadata(deck, &[("title", "التعلم الآلي في التعليم"), ("institute", "جامعة المعرفة")]);
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
        assert!(!nawl_core::has_errors(&t.validate()), "{name}: {:?}", t.validate());
        let g = nawl_codegen::generate(&t, &doc);
        assert!(g.tex.contains("\\setdefaultlanguage[numerals=mashriq]{arabic}"), "{name}");
    }
    // The journal article (Nawl's own layout, so every part has its own style and
    // font): a title block with the authors' affiliations, contact e-mail and English
    // names (one English run, in order), then the abstracts under it.
    let (_, t, doc) = arabic_samples().into_iter().find(|(n, ..)| *n == "arabic-article").unwrap();
    assert!(t.base.is_none() && t.title_page.block_height_mm.is_some());
    let g = nawl_codegen::generate(&t, &doc);
    assert!(g.tex.contains("\\texloomfield{authors_en}{\\textenglish{Abdullah Al-Ahmadi\\texloomsepen Sarah Al-Qahtani}}"), "{}", g.tex);
    assert!(g.tex.contains("\\texloomfield{authors_affiliations}{"), "affiliations without repeats");
    assert!(g.tex.contains("\\texloomfield{authors_email}{\\textenglish{author@example.edu.sa}}"));
    assert!(g.cls.contains("\\begin{english}\\section*{\\textenglish{Abstract}}\\tl@f{abstract_en}\\par\\end{english}"), "{}", g.cls);
    // The thesis's English abstract page is set in English, listed in the contents.
    let cls = nawl_codegen::template_to_cls(&builtin("arabic-thesis"));
    assert!(cls.contains("\\addcontentsline{toc}{chapter}{\\textenglish{Abstract}}\\begin{english}\\chapter*{\\textenglish{Abstract}}"), "{cls}");
}

/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test arabic -- --ignored
#[test]
#[ignore]
fn arabic_templates_compile() {
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for (name, t, doc) in arabic_samples() {
        let g = nawl_codegen::generate(&t, &doc);
        let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
        let support = g.support_files();
        let files: Vec<nawl_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
        // Right-to-left slides need LuaLaTeX (beamer-rl): the installed one, if any.
        let out = if t.kind == nawl_core::DocumentKind::Presentation {
            let Some(lua) = nawl_compile::SystemLatex::find(nawl_compile::LatexProgram::LuaLatex) else {
                eprintln!("{name}: no LuaLaTeX installed, skipped");
                continue;
            };
            let g = nawl_codegen::generate_for(&t, &doc, nawl_codegen::Engine::Lua);
            let support = g.support_files();
            let files: Vec<nawl_compile::Asset> = support.iter().map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
            lua.compile(&g.tex, &files)
        } else {
            compiler.compile(&g.tex, &files)
        }
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        if let Ok(dir) = std::env::var("NAWL_ARABIC_OUT") {
            std::fs::write(std::path::Path::new(&dir).join(format!("{name}.pdf")), &out.pdf).unwrap();
        }
        let missing: Vec<&str> = out.log.lines().filter(|l| l.contains("Missing character")).collect();
        assert!(missing.is_empty(), "{name}: characters the fonts can't show: {missing:#?}");
    }
}

#[test]
fn decimal_numbers_stay_left_to_right() {
    let p = |text: &str| serde_json::json!({"type": "paragraph", "content": [{"type": "text", "text": text}]});
    let body = serde_json::json!({"type": "doc", "content": [
        p("بلغ الفرق 3.5 درجة في المجموعة التجريبية."),
        {"type": "tableFigure", "attrs": {"id": "t-1"}, "content": [
            {"type": "caption", "content": [{"type": "text", "text": "المتوسطات"}]},
            {"type": "table", "content": [
                {"type": "tableRow", "content": [{"type": "tableHeader", "content": [p("المتوسط")]}]},
                {"type": "tableRow", "content": [{"type": "tableCell", "content": [p("54.2")]}]}
            ]}
        ]}
    ]});
    let doc = Document { metadata: Default::default(), body: serde_json::from_value(body).unwrap(), bibliography: String::new() };
    let tex = nawl_codegen::document_to_tex(&builtin("arabic-article"), &doc);
    let line = tex.lines().find(|l| l.contains("54")).unwrap_or("");
    // A cell (paragraph) starting with the number: the paragraph starts first.
    assert!(line.contains(r"\leavevmode \textenglish{54.2}"), "{line}");
    assert!(tex.contains("بلغ الفرق \\textenglish{3.5} درجة"), "{tex}");
}

/// The Arabic example documents and the built-in template each is written for.
const EXAMPLES: [(&str, &str); 8] = [
    ("arabic-article.nawl", "arabic-article"),
    ("arabic-load-forecasting.nawl", "arabic-article"),
    ("arabic-article-two-column.nawl", "arabic-article-two-column"),
    ("arabic-article-two-column-arabic-only.nawl", "arabic-article-two-column-arabic-only"),
    ("arabic-thesis.nawl", "arabic-thesis"),
    ("arabic-book.nawl", "arabic-book"),
    ("arabic-presentation.nawl", "arabic-presentation"),
    ("arabic-admin-order.nawl", "arabic-admin-order"),
];

/// A system font may not be installed where the document is compiled (the
/// administrative order's Diwani title needs Microsoft Office's fonts): it falls back
/// to Amiri (Latin Modern for Latin text) instead of failing.
#[test]
fn missing_system_fonts_fall_back() {
    let t = builtin("arabic-admin-order");
    let cls = nawl_codegen::template_to_cls(&t);
    // An installed font's italic may have no Arabic letters (Segoe UI Italic), which
    // would print boxes: italic Arabic is the upright font, slanted.
    let shapes = "ItalicFont=*,ItalicFeatures={FakeSlant=0.15},BoldItalicFont=*,BoldItalicFeatures={FakeSlant=0.15,FakeBold=1.5}";
    assert!(
        cls.contains(&format!(
            "\\IfFontExistsTF{{Diwani Letter}}{{\\newfontfamily\\tlfontDiwaniLetter[Script=Arabic,Ligatures=TeX,{shapes}]{{Diwani Letter}}}}{{\\newfontfamily\\tlfontDiwaniLetter[Script=Arabic,Ligatures=TeX]{{Amiri}}}}"
        )),
        "{cls}"
    );
    // The Arabic body font too.
    let mut t = builtin("arabic-article");
    t.fonts.arabic = Some(nawl_core::FontChoice::System { family: "DecoType Naskh".into() });
    let doc: nawl_core::DocumentFile = serde_json::from_str(&example_file("arabic-article.nawl")).unwrap();
    let tex = nawl_codegen::document_to_tex(&t, &doc.document);
    assert!(tex.contains("\\IfFontExistsTF{DecoType Naskh}{\\def\\texloomarabicfont{DecoType Naskh}}{\\def\\texloomarabicfont{Amiri}}\n"), "{tex}");
    assert!(tex.contains(&format!("\\setmainfont[Script=Arabic,{shapes}]{{\\texloomarabicfont}}")), "{tex}");
    // Amiri and the library fonts keep their own shapes.
    assert!(!nawl_codegen::document_to_tex(&builtin("arabic-article"), &doc.document).contains("FakeSlant"));
}

fn example_file(name: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(name)).unwrap()
}

/// Each example carries a copy of its template, as a document saved by the app does,
/// so opening it switches to that template (without it, the active template stays:
/// English headings and labels). The copy must match the built-in template.
#[test]
fn arabic_examples_carry_their_templates() {
    for (file, id) in EXAMPLES {
        let value: serde_json::Value = serde_json::from_str(&example_file(file)).unwrap();
        let builtin_json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../templates/{id}/template.json"))).unwrap()).unwrap();
        assert_eq!(value["template"], builtin_json, "{file}: its template differs from templates/{id}; run node scripts/embed-example-templates.mjs");
        // Complete for its template: required fields and sections, citations, references.
        let doc: nawl_core::DocumentFile = serde_json::from_str(&example_file(file)).unwrap();
        let t = builtin(id);
        let keys = nawl_core::bib::keys(&doc.document.bibliography);
        let issues = t.check_document(&doc.document, keys.as_ref());
        assert!(issues.is_empty(), "{file}: {issues:?}");
        // Arabic headings throughout.
        for title in doc.document.body.top_level_titles() {
            assert!(title.chars().any(nawl_codegen::language::is_arabic), "{file}: heading {title:?}");
        }
    }
}

/// The Arabic-only two-column article has nothing in English: no English title,
/// names, abstract or keywords in the template, and nothing English in the output
/// but the contact e-mail.
#[test]
fn arabic_only_article_has_no_english() {
    let t = builtin("arabic-article-two-column-arabic-only");
    assert!(t.fields.iter().all(|f| !f.id.ends_with("_en")), "{:?}", t.fields);
    let doc: nawl_core::DocumentFile = serde_json::from_str(&example_file("arabic-article-two-column-arabic-only.nawl")).unwrap();
    let g = nawl_codegen::generate(&t, &doc.document);
    let english: Vec<&str> = g.tex.lines().filter(|l| l.contains("\\textenglish") || l.contains("\\begin{english}")).collect();
    assert_eq!(english, ["\\texloomfield{authors_email}{\\textenglish{author@example.edu.sa}}"]);
    for word in ["Abstract", "Keywords", "References", "Figure", "Table"] {
        assert!(!g.cls.contains(word) && !g.tex.contains(word), "{word}");
    }
}

/// Merged cells in a right-to-left table: rows span with \multirow, and a cell
/// spanning columns starts on the right like the others (`l` would put it on the left).
#[test]
fn merged_cells_in_right_to_left_tables() {
    let doc: nawl_core::DocumentFile = serde_json::from_str(&example_file("arabic-article-two-column-arabic-only.nawl")).unwrap();
    let tex = nawl_codegen::document_to_tex(&builtin("arabic-article-two-column-arabic-only"), &doc.document);
    assert!(tex.contains(r"\multirow{2}{=}{الجنس} & ذكر"), "{tex}");
    assert!(tex.contains(r"\multirow{3}{=}{العمر}"), "{tex}");
    assert!(tex.contains(r"\multicolumn{2}{|r|}{المجموع} & ٤٢٠"), "{tex}");
}

/// Arabic in a code block (strings, comments) is set in the Arabic font and direction:
/// in `verbatim`, the Latin code font would leave it blank.
#[test]
fn code_blocks_with_arabic() {
    let doc: nawl_core::DocumentFile = serde_json::from_str(&example_file("arabic-book.nawl")).unwrap();
    let tex = nawl_codegen::document_to_tex(&builtin("arabic-book"), &doc.document);
    assert!(tex.contains(r#"name~=~input("\textarabic{ما~اسمك؟}~")\par"#), "{tex}");
    assert!(tex.contains(r"principal~=~10000~~~~~~\#~\textarabic{المبلغ~بالريال}\par"), "indentation and comments: {tex}");
    assert!(tex.contains("\\mbox{}\\par\n"), "blank lines stay");
    // Code without Arabic stays verbatim.
    assert!(tex.contains("\\begin{verbatim}\nfor i in range(1, 11):\n    print(7, \"×\", i, \"=\", 7 * i)\n\\end{verbatim}"), "{tex}");
}

/// Run with: NAWL_TECTONIC=… cargo test -p nawl-codegen --test arabic -- --ignored
#[test]
#[ignore]
fn arabic_examples_compile() {
    let compiler = nawl_compile::Compiler::locate(&[]).expect("tectonic");
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for (file, id) in EXAMPLES {
        let doc: nawl_core::DocumentFile = serde_json::from_str(&example_file(file)).unwrap();
        let t = builtin(id);
        let presentation = t.kind == nawl_core::DocumentKind::Presentation;
        let g = if presentation { nawl_codegen::generate_for(&t, &doc.document, nawl_codegen::Engine::Lua) } else { nawl_codegen::generate(&t, &doc.document) };
        let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
        let support = g.support_files();
        let files: Vec<nawl_compile::Asset> = support.iter().chain(figures.iter()).map(|(p, b)| nawl_compile::Asset { path: p, bytes: b }).collect();
        let out = if presentation {
            // Right-to-left slides need LuaLaTeX (beamer-rl): the installed one, if any.
            let Some(lua) = nawl_compile::SystemLatex::find(nawl_compile::LatexProgram::LuaLatex) else {
                eprintln!("{file}: no LuaLaTeX installed, skipped");
                continue;
            };
            lua.compile(&g.tex, &files)
        } else {
            compiler.compile(&g.tex, &files)
        }
        .unwrap_or_else(|e| panic!("{file}: {e:?}"));
        if let Ok(dir) = std::env::var("NAWL_ARABIC_OUT") {
            std::fs::write(std::path::Path::new(&dir).join(file.replace(".nawl", ".pdf")), &out.pdf).unwrap();
        }
        let missing: Vec<&str> = out.log.lines().filter(|l| l.contains("Missing character")).collect();
        assert!(missing.is_empty(), "{file}: characters the fonts can't show: {missing:#?}");
    }
}
