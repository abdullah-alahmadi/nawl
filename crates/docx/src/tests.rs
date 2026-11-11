use super::*;
use std::io::Read;

fn template(name: &str) -> Template {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../templates/{name}/template.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn example(name: &str) -> Document {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"));
    let f: nawl_core::DocumentFile = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
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
    let doc = example("arabic-article.nawl");
    let mathml: HashMap<String, String> = [(
        "I = \\frac{\\left(\\theta_{fc} - \\theta\\right) D}{\\eta}".to_string(),
        "<math><mi>I</mi><mo>=</mo><mfrac><mrow><mo>(</mo><msub><mi>θ</mi><mi>fc</mi></msub><mo>-</mo><mi>θ</mi><mo>)</mo><mi>D</mi></mrow><mi>η</mi></mfrac></math>".to_string(),
    )]
    .into();
    let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml, font: &|_| None }).unwrap();
    let parts = unzip(&bytes);
    assert_well_formed(&parts);
    assert!(parts["word/document.xml"].contains("Abdullah Al-Ahmadi, Sarah Al-Qahtani"), "English names under the English title");
    for p in ["[Content_Types].xml", "_rels/.rels", "word/document.xml", "word/styles.xml", "word/numbering.xml", "word/settings.xml", "word/footer1.xml", "word/media/image1.png"] {
        assert!(parts.contains_key(p), "{p} missing");
    }
    let d = &parts["word/document.xml"];
    // Right to left: the section, Arabic paragraphs and runs; the English paragraph not.
    assert!(d.contains("<w:bidi/>") && d.contains("<w:rtl/>"));
    assert!(d.contains("<w:bidi w:val=\"0\"/></w:pPr><w:r><w:rPr></w:rPr><w:t xml:space=\"preserve\">In short, the controller watered"), "the English paragraph is left to right");
    // The title block, the equation as a Word equation, the figure, the table.
    assert!(d.contains("نظام ري ذكي منخفض التكلفة"));
    assert!(d.contains("<m:oMath><m:r>") && d.contains("<m:f><m:num>"));
    assert!(d.contains("<a:blip r:embed=\"rIdImage1\"/>"));
    assert!(d.contains("<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:bidiVisual/>"));
    // Numbers as the PDF shows them: Arabic-Indic, with the template's labels.
    assert!(d.contains("شكل ١: "), "figure caption");
    assert!(d.contains("جدول ١: "), "table caption");
    // Citations and the reference list (author-year, Arabic punctuation).
    assert!(d.contains("(عبدالله الأحمدي") || d.contains("(الأحمدي"), "author-year citation");
    assert!(d.contains("Goap, A., Sharma, D."));
    assert!(parts["word/styles.xml"].contains("w:cs=\"Traditional Arabic\""));
    assert!(parts["word/settings.xml"].contains("w:bidi=\"ar-SA\""));
}

#[test]
fn author_year_citations_in_arabic() {
    let t = template("arabic-article");
    let doc = example("arabic-load-forecasting.nawl");
    let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap();
    let d = &unzip(&bytes)["word/document.xml"];
    // Years in the document's Arabic-Indic digits, as the numbered styles' numbers.
    assert!(d.contains("، ٢٠١٦)"), "Hong and Fan, 2016");
    assert!(!d.contains("، 2016)"));
    // A left-to-right mark keeps the period of "et al." after "al".
    assert!(d.contains("(Box et al.\u{200E}"), "Box et al.");
    // English entries are left-to-right paragraphs.
    assert!(d.contains("<w:bidi w:val=\"0\"/></w:pPr><w:r><w:rPr></w:rPr><w:t xml:space=\"preserve\">Hochreiter, S."), "Hochreiter");
    assert!(d.contains("(1997)") && d.contains("1735\u{200E}–\u{200E}1780"), "English entries keep Western digits");
    // Arabic entries in Arabic-Indic digits too (years, volumes, page ranges).
    assert!(d.contains("(٢٠٢١)") && d.contains("٧٧–٩٨"), "Arabic entries");
    assert!(!d.contains("77–98"));
}

/// A paragraph's own alignment: Word's "left" is the start of the line, so the
/// other way round in right-to-left paragraphs.
#[test]
fn aligned_paragraphs() {
    let p = |text: &str, align: &str, dir: Option<&str>| serde_json::json!({"type": "paragraph", "attrs": {"align": align, "dir": dir}, "content": [{"type": "text", "text": text}]});
    let body = serde_json::json!({"type": "doc", "content": [p("يسار", "left", None), p("وسط", "center", None), p("Right", "right", Some("ltr"))]});
    let doc = Document { metadata: Default::default(), body: serde_json::from_value(body).unwrap(), bibliography: String::new() };
    let t = template("arabic-article");
    let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap();
    let parts = unzip(&bytes);
    assert_well_formed(&parts);
    assert_schema_order(&parts);
    let d = &parts["word/document.xml"];
    let jc = |text: &str| {
        let end = d.find(text).unwrap_or_else(|| panic!("{text}"));
        let start = d[..end].rfind("<w:pPr>").unwrap();
        d[start..end].split("<w:jc w:val=\"").nth(1).and_then(|s| s.split('"').next()).unwrap_or("").to_string()
    };
    assert_eq!((jc("يسار").as_str(), jc("وسط").as_str(), jc("Right").as_str()), ("right", "center", "right"), "{d}");
}

/// Arabic formulas (right to left) go in as pictures of how the PDF sets them, on the
/// text's baseline; without a picture, or written the international way, they're
/// Word equations as before.
#[test]
fn arabic_formulas_are_pictures() {
    let mut t = template("arabic-article");
    t.language.as_mut().unwrap().math = nawl_core::MathStyle::Arabic;
    let body = serde_json::json!({"type": "doc", "content": [
        {"type": "paragraph", "content": [{"type": "text", "text": "حيث "}, {"type": "inlineMath", "attrs": {"latex": "س^2"}}, {"type": "text", "text": " مربع."}]},
        {"type": "equation", "attrs": {"latex": "س^2", "numbered": true}},
        {"type": "equation", "attrs": {"latex": "ص+1", "numbered": false}},
        {"type": "equation", "attrs": {"latex": "E=mc^2", "numbered": false, "style": "international"}},
    ]});
    let doc = Document { metadata: Default::default(), body: serde_json::from_value(body).unwrap(), bibliography: String::new() };
    let png = figure("figures/system-diagram.png").unwrap();
    let pictures: HashMap<String, FormulaPicture> = [
        (formula_key("س^2", false), FormulaPicture { png: png.clone(), width_pt: 20.0, height_pt: 14.0, depth_pt: 3.0 }),
        (formula_key(" س^2 ", true), FormulaPicture { png: png.clone(), width_pt: 30.0, height_pt: 20.0, depth_pt: 0.0 }),
        // Not used: the formula is written the international way.
        (formula_key("E=mc^2", true), FormulaPicture { png, width_pt: 1.0, height_pt: 1.0, depth_pt: 0.0 }),
    ]
    .into();
    let mathml: HashMap<String, String> = [("E=mc^2".to_string(), "<math><mi>E</mi><mo>=</mo><mi>m</mi></math>".to_string())].into();
    let input = Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml, font: &|_| None };
    let parts = unzip(&write_with(&input, &pictures).unwrap());
    assert_well_formed(&parts);
    assert_schema_order(&parts);
    let d = &parts["word/document.xml"];
    assert_eq!(d.matches("<w:drawing>").count(), 2, "{d}");
    assert!(parts.contains_key("word/media/image1.png") && parts.contains_key("word/media/image2.png") && !parts.contains_key("word/media/image3.png"));
    // 20 × 14 TeX points, lowered by 3pt (6 half-points); the display one isn't lowered.
    assert!(d.contains("<w:r><w:rPr><w:position w:val=\"-6\"/></w:rPr><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"253051\" cy=\"177136\"/>"), "{d}");
    assert!(d.contains("<w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"379577\" cy=\"253051\"/>"), "{d}");
    // No picture for ص+1: shown some other way, not dropped; E=mc² is a Word equation.
    assert!(d.contains("ص") && d.contains("<m:oMath>"), "{d}");
    // Without pictures nothing changes.
    let plain = unzip(&write(&input).unwrap());
    assert!(!plain["word/document.xml"].contains("<w:drawing>"));
}

#[test]
fn english_report_with_merged_cells_and_crop() {
    let t = template("senior-design");
    let mut doc = example("senior-design-report.nawl");
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
    let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap();
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
    let doc = example("presentation.nawl");
    assert!(write(&Input { template: &t, doc: &doc, image: &|_| None, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).is_err());
}

/// An article's title block and abstract share the first page with the text; in two
/// columns they span both, in a section of their own that the columns continue.
#[test]
fn articles_start_on_the_first_page() {
    let doc = example("article.nawl");
    let one = unzip(&write(&Input { template: &template("general-article"), doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap());
    assert_well_formed(&one);
    let d = &one["word/document.xml"];
    assert!(!d.contains("w:type=\"page\""), "no page breaks before the text");
    assert_eq!(d.matches("<w:sectPr>").count(), 1, "one section, numbered from 1");
    assert!(!d.contains("<w:cols"));

    let two = unzip(&write(&Input { template: &template("article-two-column"), doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap());
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
    let mut doc = example("arabic-article.nawl");
    doc.metadata.insert("date".into(), FieldValue::Text("2026-05-14".into()));
    let parts = unzip(&write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap());
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

/// A library font is embedded: listed in the font table with its key, stored
/// obfuscated (XORing again gives the file back), and Word told to use embedded fonts.
#[test]
fn library_fonts_are_embedded() {
    let mut t = template("arabic-article");
    t.fonts.arabic = Some(nawl_core::FontChoice::Library { id: "cairo".into() });
    let regular = b"OTTO regular font bytes, longer than thirty-two bytes".to_vec();
    let font = |rel: &str| (rel == "fonts/cairo-Regular.ttf").then(|| regular.clone());
    let parts = unzip(&write(&Input { template: &t, doc: &example("arabic-article.nawl"), image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &font }).unwrap());
    assert_well_formed(&parts);
    let table = &parts["word/fontTable.xml"];
    assert!(table.contains("<w:font w:name=\"Cairo\">") && table.contains("<w:embedRegular r:id=\"rIdF1\" w:fontKey=\"{"), "{table}");
    assert!(!table.contains("embedBold"), "only files that are there");
    assert!(parts["word/settings.xml"].contains("<w:embedTrueTypeFonts/>"));
    assert!(parts["word/_rels/document.xml.rels"].contains("Target=\"fontTable.xml\""));
    assert!(parts["[Content_Types].xml"].contains("Extension=\"odttf\""));
    let key = table.split("w:fontKey=\"").nth(1).unwrap().split('"').next().unwrap();
    assert_eq!(obfuscate(&obfuscate(&regular, key), key), regular);
    assert_ne!(obfuscate(&regular, key)[..32], regular[..32]);
    // Without library fonts, nothing of this.
    let plain = unzip(&write(&Input { template: &template("arabic-article"), doc: &example("arabic-article.nawl"), image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &font }).unwrap());
    assert!(!plain.contains_key("word/fontTable.xml") && !plain["word/settings.xml"].contains("embedTrueType"));
}

/// A heading level's own font is its Word style's font (every script), and a library
/// font used only there is embedded too.
#[test]
fn heading_fonts_reach_word() {
    let mut t = template("arabic-thesis");
    t.headings.levels[0].text.font = Some(nawl_core::FontChoice::Library { id: "reem-kufi".into() });
    let font = |rel: &str| (rel == "fonts/reem-kufi-Regular.ttf").then(|| b"font bytes, at least thirty-two of them".to_vec());
    let parts = unzip(&write(&Input { template: &t, doc: &example("arabic-article.nawl"), image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &font }).unwrap());
    let styles = &parts["word/styles.xml"];
    let heading = &styles[styles.find("w:styleId=\"Heading1\"").unwrap()..];
    let heading = &heading[..heading.find("</w:style>").unwrap()];
    assert!(heading.contains("<w:rFonts w:ascii=\"Reem Kufi\" w:hAnsi=\"Reem Kufi\" w:cs=\"Reem Kufi\"/>"), "{heading}");
    assert!(parts["word/fontTable.xml"].contains("<w:font w:name=\"Reem Kufi\">"));
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
    for (tpl, ex) in [("arabic-article", "arabic-article.nawl"), ("senior-design", "senior-design-report.nawl")] {
        let t = template(tpl);
        let doc = example(ex);
        let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap();
        assert_schema_order(&unzip(&bytes));
    }
    let t = template("arabic-thesis");
    let doc = example("arabic-article.nawl");
    let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap();
    assert_schema_order(&unzip(&bytes));
}

/// Writes the exports to $NAWL_DOCX_OUT, to open in Word.
#[test]
#[ignore]
fn write_samples() {
    let out = std::path::PathBuf::from(std::env::var("NAWL_DOCX_OUT").expect("NAWL_DOCX_OUT"));
    let mathml: HashMap<String, String> = [(
        r"g = \frac{post - pre}{100 - pre}".to_string(),
        "<math><mi>g</mi><mo>=</mo><mfrac><mrow><mi>post</mi><mo>-</mo><mi>pre</mi></mrow><mrow><mn>100</mn><mo>-</mo><mi>pre</mi></mrow></mfrac></math>".to_string(),
    )]
    .into();
    for (tpl, ex, name) in [
        ("arabic-article", "arabic-article.nawl", "arabic-article.docx"),
        ("arabic-thesis", "arabic-article.nawl", "arabic-thesis.docx"),
        ("senior-design", "senior-design-report.nawl", "senior-design.docx"),
        ("general-article", "article.nawl", "general-article.docx"),
        ("arabic-article-cairo", "arabic-article.nawl", "arabic-article-cairo.docx"),
        ("article-two-column", "article.nawl", "article-two-column.docx"),
        // Merged cells (rows and columns) in a right-to-left table.
        ("arabic-article-two-column-arabic-only", "arabic-article-two-column-arabic-only.nawl", "arabic-merged-cells.docx"),
    ] {
        // "-cairo": the Arabic article with a library font, embedded (the fonts fetched
        // by scripts/fetch-fonts.mjs).
        let t = match tpl.strip_suffix("-cairo") {
            Some(base) => {
                let mut t = template(base);
                t.fonts.arabic = Some(nawl_core::FontChoice::Library { id: "cairo".into() });
                t
            }
            None => template(tpl),
        };
        let doc = example(ex);
        let fonts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri");
        let font = |rel: &str| std::fs::read(fonts_dir.join(rel)).ok();
        let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml, font: &font }).unwrap();
        std::fs::write(out.join(name), bytes).unwrap();
    }
    // Formulas as MathLive converts them (a JSON map of LaTeX → MathML), one per equation.
    if let Ok(path) = std::env::var("NAWL_MATHML") {
        let mathml: HashMap<String, String> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut blocks: Vec<serde_json::Value> = mathml.keys().map(|l| serde_json::json!({"type": "equation", "attrs": {"latex": l}})).collect();
        blocks.insert(0, serde_json::json!({"type": "paragraph", "content": [{"type": "text", "text": "Inline "}, {"type": "inlineMath", "attrs": {"latex": mathml.keys().next().unwrap()}}]}));
        let doc = Document {
            metadata: Default::default(),
            body: serde_json::from_value(serde_json::json!({"type": "doc", "content": blocks})).unwrap(),
            bibliography: String::new(),
        };
        let t = template("senior-design");
        let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml, font: &|_| None }).unwrap();
        std::fs::write(out.join("formulas.docx"), bytes).unwrap();
        // The same formulas in an Arabic document with Arabic-Indic digits.
        let t = template("arabic-thesis");
        let bytes = write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &mathml, font: &|_| None }).unwrap();
        std::fs::write(out.join("formulas-ar.docx"), bytes).unwrap();
    }
}

#[test]
fn university_report_has_cover_pages_and_logo() {
    let t = template("engineering-report");
    let doc = example("engineering-report.nawl");
    let parts = unzip(&write(&Input { template: &t, doc: &doc, image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap());
    assert_well_formed(&parts);
    let body = &parts["word/document.xml"];
    // Five chapters, the references and the appendices each open on a cover: the
    // heading lower down the page, then a page break.
    let cover = format!("<w:spacing w:before=\"{}\"/><w:jc w:val=\"center\"/>", xml::twips(85.0));
    assert_eq!(body.matches(&cover).count(), 8, "covers");
    assert!(body.contains("Appendices"));
    // The logo the author picked (an image field) is on the cover.
    assert!(parts.keys().any(|k| k.starts_with("word/media/")), "no logo picture");
}

#[test]
fn authors_with_different_affiliations_are_numbered() {
    // The sample's two authors are at different universities.
    let parts = unzip(&write(&Input { template: &template("arabic-article"), doc: &example("arabic-load-forecasting.nawl"), image: &figure, asset: &|_| None, mathml: &HashMap::new(), font: &|_| None }).unwrap());
    assert_well_formed(&parts);
    let body = &parts["word/document.xml"];
    let sup = "<w:vertAlign w:val=\"superscript\"/>";
    // Two Arabic names, two Arabic affiliations, two English names.
    assert!(body.matches(sup).count() >= 6, "{}", body.matches(sup).count());
    assert!(body.contains(&format!("{sup}<w:rtl/></w:rPr><w:t xml:space=\"preserve\">١</w:t>")) || body.contains("١</w:t>"));
    assert!(!body.contains('\u{E000}') && !body.contains('\u{E001}'), "markers left in the text");
}
