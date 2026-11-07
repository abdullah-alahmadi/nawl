use super::*;
use std::io::Read;

fn template(name: &str) -> Template {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../templates/{name}/template.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn example(name: &str) -> Document {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"));
    let f: texloom_core::DocumentFile = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    f.document
}

fn figure(path: &str) -> Option<Vec<u8>> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(path)).ok()
}

/// The parts of a .docx, by name.
fn unzip(bytes: &[u8]) -> HashMap<String, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut out = HashMap::new();
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).unwrap();
        let mut s = Vec::new();
        f.read_to_end(&mut s).unwrap();
        out.insert(f.name().unwrap().to_string(), String::from_utf8_lossy(&s).into_owned());
    }
    out
}

/// Every XML part parses (Word refuses files with malformed XML).
fn assert_well_formed(parts: &HashMap<String, String>) {
    for (name, xml) in parts.iter().filter(|(n, _)| n.ends_with(".xml") || n.ends_with(".rels")) {
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut depth = 0i32;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(_)) => depth += 1,
                Ok(quick_xml::events::Event::End(_)) => depth -= 1,
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => panic!("{name}: {e}"),
                _ => {}
            }
        }
        assert_eq!(depth, 0, "{name}: unbalanced elements");
    }
}

#[test]
fn arabic_article_exports() {
    let t = template("arabic-article");
    let doc = example("arabic-article.loom");
    let mathml: HashMap<String, String> = [(
        "g = \\frac{post - pre}{100 - pre}".to_string(),
        "<math><mi>g</mi><mo>=</mo><mfrac><mrow><mi>post</mi><mo>-</mo><mi>pre</mi></mrow><mrow><mn>100</mn><mo>-</mo><mi>pre</mi></mrow></mfrac></math>".to_string(),
    )]
    .into();
    let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml }).unwrap();
    let parts = unzip(&bytes);
    assert_well_formed(&parts);
    assert!(parts["word/document.xml"].contains("Abdullah Al-Ahmadi, Sarah Al-Qahtani"), "English names under the English title");
    for p in ["[Content_Types].xml", "_rels/.rels", "word/document.xml", "word/styles.xml", "word/numbering.xml", "word/settings.xml", "word/footer1.xml", "word/media/image1.png"] {
        assert!(parts.contains_key(p), "{p} missing");
    }
    let d = &parts["word/document.xml"];
    // Right to left: the section, Arabic paragraphs and runs; the English paragraph not.
    assert!(d.contains("<w:bidi/>") && d.contains("<w:rtl/>"));
    assert!(d.contains("<w:bidi w:val=\"0\"/></w:pPr><w:r><w:rPr></w:rPr><w:t xml:space=\"preserve\">As Siemens notes"), "the English paragraph is left to right");
    // The title block, the equation as a Word equation, the figure, the table.
    assert!(d.contains("أثر التغذية الراجعة الفورية"));
    assert!(d.contains("<m:oMath><m:r>") && d.contains("<m:f><m:num>"));
    assert!(d.contains("<a:blip r:embed=\"rIdImage1\"/>"));
    assert!(d.contains("<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:bidiVisual/>"));
    // Numbers as the PDF shows them: Arabic-Indic, with the template's labels.
    assert!(d.contains("شكل ١: "), "figure caption");
    assert!(d.contains("جدول ١: "), "table caption");
    // Citations and the reference list (author-year, Arabic punctuation).
    assert!(d.contains("(عبدالله الأحمدي") || d.contains("(الأحمدي"), "author-year citation");
    assert!(d.contains("Siemens, G. (2013)."));
    assert!(parts["word/styles.xml"].contains("w:cs=\"Traditional Arabic\""));
    assert!(parts["word/settings.xml"].contains("w:bidi=\"ar-SA\""));
}

#[test]
fn english_report_with_merged_cells_and_crop() {
    let t = template("senior-design");
    let mut doc = example("senior-design-report.loom");
    // Crop the figure and merge two cells, as the editor can.
    let mut json = serde_json::to_value(&doc.body).unwrap();
    fn visit(v: &mut serde_json::Value) {
        if v.get("type").and_then(|t| t.as_str()) == Some("figure") {
            v["attrs"]["crop"] = serde_json::json!({"left": 10.0, "right": 0.0, "top": 20.0, "bottom": 0.0});
        }
        if v.get("type").and_then(|t| t.as_str()) == Some("tableHeader") {
            v["attrs"]["rowspan"] = serde_json::json!(1);
        }
        if let Some(children) = v.get_mut("content").and_then(|c| c.as_array_mut()) {
            for c in children {
                visit(c);
            }
        }
    }
    visit(&mut json);
    doc.body = serde_json::from_value(json).unwrap();
    let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new() }).unwrap();
    let parts = unzip(&bytes);
    assert_well_formed(&parts);
    let d = &parts["word/document.xml"];
    assert!(!d.contains("<w:rtl/>") && !d.contains("<w:bidi/>"), "an English document has nothing right to left");
    assert!(d.contains("<a:srcRect l=\"10000\" t=\"20000\" r=\"0\" b=\"0\"/>"), "the crop");
    assert!(d.contains("<w:pStyle w:val=\"Heading1\"/>") && d.contains("CHAPTER 1") || d.contains("Chapter 1"), "chapter label");
    assert!(d.contains(" TOC \\o \"1-3\" \\h \\z \\u "), "table of contents field");
    assert!(d.contains("w:fmt=\"lowerRoman\""), "front matter numbered i, ii…");
    // Formulas without MathML fall back to their LaTeX, still as a Word equation.
    assert!(d.contains("<m:oMath><m:r><m:rPr><m:sty m:val=\"p\"/>"));
}

#[test]
fn slides_are_not_exported() {
    let t = template("beamer-presentation");
    let doc = example("presentation.loom");
    assert!(write(&Input { template: &t, doc: &doc, image: &|_| None, asset: &|_| None, mathml: &HashMap::new() }).is_err());
}

/// An article's title block and abstract share the first page with the text; in two
/// columns they span both, in a section of their own that the columns continue.
#[test]
fn articles_start_on_the_first_page() {
    let doc = example("article.loom");
    let one = unzip(&write(&Input { template: &template("general-article"), doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new() }).unwrap());
    assert_well_formed(&one);
    let d = &one["word/document.xml"];
    assert!(!d.contains("w:type=\"page\""), "no page breaks before the text");
    assert_eq!(d.matches("<w:sectPr>").count(), 1, "one section, numbered from 1");
    assert!(!d.contains("<w:cols"));

    let two = unzip(&write(&Input { template: &template("article-two-column"), doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new() }).unwrap());
    assert_well_formed(&two);
    let d = &two["word/document.xml"];
    assert!(!d.contains("w:type=\"page\""));
    assert_eq!(d.matches("<w:sectPr>").count(), 2, "the spanning title block, then the columns");
    let last = &d[d.rfind("<w:sectPr>").unwrap()..];
    assert!(last.contains("<w:type w:val=\"continuous\"/>") && last.contains("<w:cols w:num=\"2\""), "{last}");
    assert!(!last.contains("w:start="), "page numbers go on from the title block");
}

/// The Arabic thesis: the defence date in both calendars, and Arabic references
/// before the foreign ones, each list under its heading, numbered on.
#[test]
fn thesis_dates_and_reference_lists() {
    let t = template("arabic-thesis");
    let mut doc = example("arabic-article.loom");
    doc.metadata.insert("date".into(), FieldValue::Text("2026-05-14".into()));
    let parts = unzip(&write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new() }).unwrap());
    assert_well_formed(&parts);
    let d = &parts["word/document.xml"];
    assert!(d.contains("٢٧ ذو القعدة ١٤٤٧هـ") && d.contains("١٤ مايو ٢٠٢٦"), "both calendars");
    let arabic = d.find("المراجع العربية").expect("Arabic list heading");
    let foreign = d.find("المراجع الأجنبية").expect("foreign list heading");
    assert!(arabic < foreign);
    let ahmadi = d.find("الأحمدي").map(|_| d.rfind("الأحمدي").unwrap()).unwrap();
    assert!(arabic < ahmadi && ahmadi < foreign, "the Arabic entry is in the Arabic list");
    // Sub-lists in abjad letters, as polyglossia prints them: (ا) (ب) (ج).
    assert!(parts["word/numbering.xml"].contains("<w:numFmt w:val=\"arabicAbjad\"/><w:lvlText w:val=\"(%2)\"/>"));
}

#[test]
fn number_ranges_hold_together() {
    assert_eq!(hold_ranges("ص 45–70."), "ص 45\u{200E}–\u{200E}70.");
    assert_eq!(hold_ranges("a – b, 3–"), "a – b, 3–", "only dashes between digits");
}

#[test]
fn image_sizes() {
    let png = figure("figures/system-diagram.png").unwrap();
    let (w, h) = image_size(&png).unwrap();
    assert!(w > 0 && h > 0);
    // A minimal JPEG header with a start-of-frame of 40×30.
    let jpeg = [0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x1E, 0x00, 0x28, 0x03, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(image_size(&jpeg), Some((40, 30)));
    assert_eq!(image_size(b"%PDF-1.4"), None);
}

/// Word refuses property elements out of the schema's order: check every
/// `w:pPr`, `w:rPr` and `w:sectPr` in the parts against it.
fn assert_schema_order(parts: &HashMap<String, String>) {
    const PPR: &[&str] = &["pStyle", "keepNext", "keepLines", "pageBreakBefore", "framePr", "widowControl", "numPr", "suppressLineNumbers", "pBdr", "shd", "tabs", "suppressAutoHyphens", "kinsoku", "wordWrap", "overflowPunct", "topLinePunct", "autoSpaceDE", "autoSpaceDN", "bidi", "adjustRightInd", "snapToGrid", "spacing", "ind", "contextualSpacing", "mirrorIndents", "suppressOverlap", "jc", "textDirection", "textAlignment", "textboxTightWrap", "outlineLvl", "divId", "cnfStyle", "rPr", "sectPr", "pPrChange"];
    const RPR: &[&str] = &["rStyle", "rFonts", "b", "bCs", "i", "iCs", "caps", "smallCaps", "strike", "dstrike", "outline", "shadow", "emboss", "imprint", "noProof", "snapToGrid", "vanish", "webHidden", "color", "spacing", "w", "kern", "position", "sz", "szCs", "highlight", "u", "effect", "bdr", "shd", "fitText", "vertAlign", "rtl", "cs", "em", "lang", "eastAsianLayout", "specVanish", "oMath"];
    const SECT: &[&str] = &["headerReference", "footerReference", "footnotePr", "endnotePr", "type", "pgSz", "pgMar", "paperSrc", "pgBorders", "lnNumType", "pgNumType", "cols", "formProt", "vAlign", "noEndnote", "titlePg", "textDirection", "bidi", "rtlGutter", "docGrid", "printerSettings"];
    for (name, xml) in parts.iter().filter(|(n, _)| n.ends_with(".xml")) {
        let mut reader = quick_xml::Reader::from_str(xml);
        // For each open properties element: its order list and the last index seen.
        let mut stack: Vec<(String, Option<(&[&str], usize)>)> = Vec::new();
        loop {
            let (local, empty) = match reader.read_event() {
                Ok(quick_xml::events::Event::Start(e)) => (String::from_utf8_lossy(e.local_name().as_ref().as_bytes()).into_owned(), false),
                Ok(quick_xml::events::Event::Empty(e)) => (String::from_utf8_lossy(e.local_name().as_ref().as_bytes()).into_owned(), true),
                Ok(quick_xml::events::Event::End(_)) => {
                    stack.pop();
                    continue;
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => panic!("{name}: {e}"),
                _ => continue,
            };
            if let Some((parent, Some((order, last)))) = stack.last_mut() {
                if let Some(i) = order.iter().position(|o| *o == local) {
                    assert!(i >= *last, "{name}: <w:{local}> after a later element in <w:{parent}>");
                    *last = i;
                }
            }
            if !empty {
                let order: Option<&[&str]> = match local.as_str() {
                    "pPr" => Some(PPR),
                    "rPr" => Some(RPR),
                    "sectPr" => Some(SECT),
                    _ => None,
                };
                stack.push((local, order.map(|o| (o, 0))));
            }
        }
    }
}

#[test]
fn properties_are_in_schema_order() {
    for (tpl, ex) in [("arabic-article", "arabic-article.loom"), ("senior-design", "senior-design-report.loom")] {
        let t = template(tpl);
        let doc = example(ex);
        let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new() }).unwrap();
        assert_schema_order(&unzip(&bytes));
    }
    let t = template("arabic-thesis");
    let doc = example("arabic-article.loom");
    let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new() }).unwrap();
    assert_schema_order(&unzip(&bytes));
}

/// Writes the exports to $TEXLOOM_DOCX_OUT, to open in Word.
#[test]
#[ignore]
fn write_samples() {
    let out = std::path::PathBuf::from(std::env::var("TEXLOOM_DOCX_OUT").expect("TEXLOOM_DOCX_OUT"));
    let mathml: HashMap<String, String> = [(
        r"g = \frac{post - pre}{100 - pre}".to_string(),
        "<math><mi>g</mi><mo>=</mo><mfrac><mrow><mi>post</mi><mo>-</mo><mi>pre</mi></mrow><mrow><mn>100</mn><mo>-</mo><mi>pre</mi></mrow></mfrac></math>".to_string(),
    )]
    .into();
    for (tpl, ex, name) in [
        ("arabic-article", "arabic-article.loom", "arabic-article.docx"),
        ("arabic-thesis", "arabic-article.loom", "arabic-thesis.docx"),
        ("senior-design", "senior-design-report.loom", "senior-design.docx"),
        ("general-article", "article.loom", "general-article.docx"),
        ("article-two-column", "article.loom", "article-two-column.docx"),
    ] {
        let t = template(tpl);
        let doc = example(ex);
        let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml }).unwrap();
        std::fs::write(out.join(name), bytes).unwrap();
    }
    // Formulas as MathLive converts them (a JSON map of LaTeX → MathML), one per equation.
    if let Ok(path) = std::env::var("TEXLOOM_MATHML") {
        let mathml: HashMap<String, String> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut blocks: Vec<serde_json::Value> = mathml.keys().map(|l| serde_json::json!({"type": "equation", "attrs": {"latex": l}})).collect();
        blocks.insert(0, serde_json::json!({"type": "paragraph", "content": [{"type": "text", "text": "Inline "}, {"type": "inlineMath", "attrs": {"latex": mathml.keys().next().unwrap()}}]}));
        let doc = Document {
            metadata: Default::default(),
            body: serde_json::from_value(serde_json::json!({"type": "doc", "content": blocks})).unwrap(),
            bibliography: String::new(),
        };
        let t = template("senior-design");
        let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml }).unwrap();
        std::fs::write(out.join("formulas.docx"), bytes).unwrap();
        // The same formulas in an Arabic document with Arabic-Indic digits.
        let t = template("arabic-thesis");
        let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml }).unwrap();
        std::fs::write(out.join("formulas-ar.docx"), bytes).unwrap();
    }
}
