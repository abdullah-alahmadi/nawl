//! Exports a Texloom document to Microsoft Word (.docx), for the cases where
//! someone still asks for a Word file. Texloom never reads Word files.
//!
//! The Word file follows the template as closely as Word allows: page size and
//! margins, fonts (a Latin font and a font for Arabic), heading styles with their
//! numbers written out as the editor shows them, captions, lists, tables (merged
//! cells included), figures (cropped), equations as editable Word equations, the
//! reference list Texloom formats itself, a table of contents field and page
//! numbers. Arabic documents are right to left throughout; English passages and
//! words in them keep their direction.

mod math;
mod props;
mod xml;

use std::collections::HashMap;
use std::io::Write;

use texloom_codegen::language::{is_arabic, Languages};
use texloom_codegen::refs;
use texloom_core::{
    Align, BibStyle, Block, BuiltinFont, Digits, Document, FieldKind, FieldValue, FontChoice, FrontMatterItem, Inline, Language, LineSpacing,
    Mark, PageNumberStyle, TableFigurePart, TableRow, Template, TextDir, TitleContent, TitleItem, TopLevel,
};

use props::{Pp, Rp};
use xml::{emu, esc, half_points, pt_twips, twips};

/// What the export needs besides the template and the document.
pub struct Input<'a> {
    pub template: &'a Template,
    pub doc: &'a Document,
    /// An image of the document by its path (`figures/plot.png`).
    pub image: &'a dyn Fn(&str) -> Option<Vec<u8>>,
    /// An image of the template by its asset path (title-page logos).
    pub asset: &'a dyn Fn(&str) -> Option<Vec<u8>>,
    /// MathML for each formula's LaTeX (from the editor); others are shown as LaTeX.
    pub mathml: &'a HashMap<String, String>,
}

/// The `.docx` file's bytes.
pub fn write(input: &Input) -> Result<Vec<u8>, String> {
    if input.template.kind == texloom_core::DocumentKind::Presentation {
        return Err("Word export is for documents; export slides as PDF.".into());
    }
    let mut w = Writer::new(input);
    let body = w.body();
    w.package(&body)
}

// ---------------------------------------------------------------- numbering

/// Numbers as the editor and the PDF show them, worked out before writing.
#[derive(Default)]
struct Numbers {
    /// The label of each heading, in order ("الفصل ١", "2.3"); `None` for
    /// unnumbered ones.
    headings: Vec<Option<String>>,
    figures: Vec<String>,
    tables: Vec<String>,
    equations: Vec<String>,
    /// What a cross-reference to an id shows.
    refs: HashMap<String, String>,
}

fn letters(n: usize) -> String {
    let mut s = String::new();
    let mut x = n;
    while x > 0 {
        s.insert(0, (b'A' + ((x - 1) % 26) as u8) as char);
        x = (x - 1) / 26;
    }
    s
}

/// How the main text follows the front matter.
enum Front {
    /// No front matter of Texloom's own (a publisher class's title block, or none).
    None,
    /// Pages of its own, in a section with its own page numbers.
    OwnSection,
    /// Only an article's title block (and its abstract), on the first page.
    FirstPage,
    /// The same over two columns: the title block spans both.
    FirstPageSpanning,
}

fn local_digits(s: &str, digits: Digits) -> String {
    if digits != Digits::ArabicIndic {
        return s.to_string();
    }
    s.chars().map(|c| if c.is_ascii_digit() { char::from_u32(0x0660 + (c as u32 - '0' as u32)).unwrap_or(c) } else { c }).collect()
}

/// Keeps number ranges ("45–70") together in right-to-left text. Unlike a dot
/// or a hyphen, an en or em dash between digits is a neutral character, so the
/// bidi algorithm reorders it to "70–45" or breaks the line there. Left-to-right
/// marks around the dash hold the range left to right.
fn hold_ranges(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        let dash = matches!(c, '–' | '—');
        let between_digits = i > 0 && chars[i - 1].is_ascii_digit() && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit());
        if dash && between_digits {
            out.push('\u{200E}');
            out.push(c);
            out.push('\u{200E}');
        } else {
            out.push(c);
        }
    }
    out
}

fn heading_word(t: &Template, level: usize, appendix: bool) -> String {
    let label = if appendix && level == 1 { t.structure.appendix_label.as_str() } else { t.headings.levels.get(level - 1).map(|h| h.label.as_str()).unwrap_or("") };
    let word: String = label.replace("{n}", "").trim_matches(|c: char| !c.is_alphanumeric()).to_string();
    if !word.is_empty() {
        return word;
    }
    let english = !t.lang().main.rtl();
    match (level, appendix, t.structure.top_level == TopLevel::Chapter) {
        (1, true, _) => if english { "Appendix" } else { "الملحق" }.into(),
        (1, _, true) => if english { "Chapter" } else { "الفصل" }.into(),
        _ => if english { "Section" } else { "القسم" }.into(),
    }
}

fn number_document(t: &Template, doc: &Document) -> Numbers {
    let mut n = Numbers::default();
    let digits = t.lang().digits;
    let mut counters = [0usize; 4];
    let (mut figure, mut table, mut equation) = (0usize, 0usize, 0usize);
    let mut appendix = false;
    let chapters = t.structure.top_level == TopLevel::Chapter;
    let per_chapter = chapters && t.structure.per_chapter_numbering;
    // Appendices are lettered: abjad letters in Arabic, as polyglossia prints them.
    let arabic = texloom_codegen::language::Languages::of(t, doc).main == Language::Arabic;
    let top = |appendix: bool, c: usize| match (appendix, arabic) {
        (false, _) => c.to_string(),
        (true, true) => texloom_codegen::language::abjad(c),
        (true, false) => letters(c),
    };
    texloom_core::walk_blocks(&doc.body.content, &mut |b| match b {
        Block::AppendixStart => {
            appendix = true;
            counters = [0; 4];
        }
        Block::Heading { attrs, .. } => {
            let level = usize::from(attrs.level.clamp(1, 4));
            if !attrs.numbered {
                n.headings.push(None);
                return;
            }
            counters[level - 1] += 1;
            for c in counters.iter_mut().skip(level) {
                *c = 0;
            }
            if level == 1 && per_chapter {
                figure = 0;
                table = 0;
                equation = 0;
            }
            let numbered = level <= usize::from(t.structure.numbered_depth) || (appendix && level == 1);
            if !numbered {
                n.headings.push(None);
                return;
            }
            let mut parts = vec![top(appendix, counters[0])];
            parts.extend(counters[1..level].iter().map(|c| c.to_string()));
            let number = local_digits(&parts.join("."), digits);
            let label = if appendix && level == 1 { t.structure.appendix_label.clone() } else { t.headings.levels.get(level - 1).map(|h| h.label.clone()).unwrap_or_default() };
            n.headings.push(Some(if label.trim().is_empty() { number.clone() } else { label.replace("{n}", &number) }));
            if let Some(id) = &attrs.id {
                n.refs.insert(id.clone(), format!("{} {number}", heading_word(t, level, appendix)));
            }
        }
        Block::Figure { attrs, .. } => {
            figure += 1;
            let num = local_digits(&if per_chapter { format!("{}.{figure}", top(appendix, counters[0])) } else { figure.to_string() }, digits);
            if let Some(id) = &attrs.id {
                n.refs.insert(id.clone(), format!("{} {num}", t.captions.figure_label));
            }
            n.figures.push(num);
        }
        Block::TableFigure { attrs, .. } => {
            table += 1;
            let num = local_digits(&if per_chapter { format!("{}.{table}", top(appendix, counters[0])) } else { table.to_string() }, digits);
            if let Some(id) = &attrs.id {
                n.refs.insert(id.clone(), format!("{} {num}", t.captions.table_label));
            }
            n.tables.push(num);
        }
        Block::Equation { attrs } if attrs.numbered => {
            equation += 1;
            let num = local_digits(&if per_chapter { format!("{}.{equation}", top(appendix, counters[0])) } else { equation.to_string() }, digits);
            if let Some(id) = &attrs.id {
                n.refs.insert(id.clone(), format!("({num})"));
            }
            n.equations.push(num);
        }
        _ => {}
    });
    n
}

// ---------------------------------------------------------------- the writer

struct Writer<'a> {
    input: &'a Input<'a>,
    t: &'a Template,
    langs: Languages,
    rtl: bool,
    numbers: Numbers,
    /// How many headings, figures, tables and equations have been written.
    next: (usize, usize, usize, usize),
    citations: HashMap<String, String>,
    references: Vec<refs::Reference>,
    /// Images: (relationship id, file name, bytes).
    media: Vec<(String, String, Vec<u8>)>,
    /// External links: (relationship id, URL).
    links: Vec<(String, String)>,
    /// Numbered-list instances: (numId, start).
    lists: Vec<(usize, u32)>,
    drawing_id: usize,
    bibliography_done: bool,
    /// The style of plain paragraphs being written (inside a quote or a table cell).
    context_style: Option<&'static str>,
    /// Table header cells: their text is bold.
    bold_text: bool,
}

const TEXT_WIDTH_FALLBACK_MM: f32 = 160.0;

impl<'a> Writer<'a> {
    fn new(input: &'a Input<'a>) -> Self {
        let t = input.template;
        let doc = input.doc;
        let langs = Languages::of(t, doc);
        let style = refs::style_of(t);
        let references = refs::references(doc, style, langs.main, t.bibliography.split_by_language);
        let mut citations = HashMap::new();
        for (i, r) in references.iter().enumerate() {
            let label = match (&r.label, style) {
                (Some(l), BibStyle::AuthorYear) => {
                    let (who, year) = l.rsplit_once('(').map(|(w, y)| (w.to_string(), y.trim_end_matches(')').to_string())).unwrap_or((l.clone(), String::new()));
                    let sep = if langs.main.rtl() { "، " } else { ", " };
                    format!("{who}{sep}{year}")
                }
                _ => local_digits(&(i + 1).to_string(), langs.digits),
            };
            citations.insert(r.key.clone(), label);
        }
        Self {
            input,
            t,
            rtl: langs.main.rtl(),
            langs,
            numbers: number_document(t, doc),
            next: (0, 0, 0, 0),
            citations,
            references,
            media: Vec::new(),
            links: Vec::new(),
            lists: Vec::new(),
            drawing_id: 1,
            bibliography_done: false,
            context_style: None,
            bold_text: false,
        }
    }

    fn text_width_mm(&self) -> f32 {
        let (w, _) = self.t.page.paper.size_mm();
        let m = &self.t.page.margins;
        let width = w - m.inner_mm - m.outer_mm - self.t.page.binding_offset_mm;
        if width > 20.0 { width } else { TEXT_WIDTH_FALLBACK_MM }
    }

    /// The width of a line of the main text: a column's in two columns (with the
    /// LaTeX class's 6 mm gap).
    fn line_width_mm(&self) -> f32 {
        if self.t.page.columns == 2 { (self.text_width_mm() - 6.0) / 2.0 } else { self.text_width_mm() }
    }

    /// The paragraph direction to write: only where it differs from the styles'.
    fn bidi(&self, rtl: bool) -> Option<bool> {
        match (rtl, self.rtl) {
            (true, _) => Some(true),
            (false, true) => Some(false),
            (false, false) => None,
        }
    }

    // ---------------------------------------------------------------- text runs

    /// Text as runs: Arabic stretches are right to left (`w:rtl`), each run with
    /// the properties `rp`.
    fn text_runs(&self, text: &str, rp: &Rp) -> String {
        let mut out = String::new();
        let mut current = String::new();
        let mut current_rtl: Option<bool> = None;
        let bold = self.bold_text;
        let flush = |out: &mut String, s: &mut String, rtl: Option<bool>| {
            if s.is_empty() {
                return;
            }
            let props = Rp { rtl: rtl == Some(true), bold: rp.bold || bold, ..rp.clone() };
            out.push_str(&format!("<w:r>{}<w:t xml:space=\"preserve\">{}</w:t></w:r>", props.xml(), esc(s)));
            s.clear();
        };
        let text = hold_ranges(text);
        for c in text.chars() {
            let script = if is_arabic(c) { Some(true) } else if c.is_alphabetic() { Some(false) } else { None };
            if let Some(rtl) = script {
                if current_rtl.is_some_and(|r| r != rtl) {
                    flush(&mut out, &mut current, current_rtl);
                }
                current_rtl = Some(rtl);
            }
            current.push(c);
        }
        flush(&mut out, &mut current, current_rtl);
        out
    }

    fn mark_props(marks: &[Mark], mono: &str) -> Rp {
        let mut p = Rp::default();
        for m in marks {
            match m {
                Mark::Bold => p.bold = true,
                Mark::Italic => p.italic = true,
                Mark::Underline => p.underline = true,
                Mark::Strike => p.strike = true,
                Mark::Code => p.font = Some(mono.to_string()),
                Mark::Link { .. } => p.style = Some("Hyperlink"),
                Mark::Unknown => {}
            }
        }
        p
    }

    fn inlines(&mut self, content: &[Inline]) -> String {
        let mono = word_font(&self.t.fonts.mono, Role::Mono);
        let mut out = String::new();
        for i in content {
            match i {
                Inline::Text { text, marks } => {
                    let runs = self.text_runs(text, &Self::mark_props(marks, &mono));
                    match marks.iter().find_map(|m| if let Mark::Link { attrs } = m { Some(attrs.href.clone()) } else { None }) {
                        Some(href) if !href.trim().is_empty() => {
                            let id = format!("rIdLink{}", self.links.len() + 1);
                            self.links.push((id.clone(), href));
                            out.push_str(&format!("<w:hyperlink r:id=\"{id}\">{runs}</w:hyperlink>"));
                        }
                        _ => out.push_str(&runs),
                    }
                }
                Inline::HardBreak => out.push_str("<w:r><w:br/></w:r>"),
                Inline::InlineMath { attrs } => out.push_str(&self.formula(&attrs.latex)),
                Inline::Ref { attrs } => {
                    let text = self.numbers.refs.get(&attrs.target).cloned().unwrap_or_else(|| "??".into());
                    out.push_str(&self.text_runs(&text, &Rp::default()));
                }
                Inline::Citation { attrs } => {
                    let parts: Vec<String> = attrs.keys.iter().map(|k| self.citations.get(k).cloned().unwrap_or_else(|| "?".into())).collect();
                    let text = if refs::style_of(self.t) == BibStyle::AuthorYear {
                        format!("({})", parts.join(if self.rtl { "؛ " } else { "; " }))
                    } else {
                        format!("[{}]", parts.join(if self.rtl { "، " } else { ", " }))
                    };
                    out.push_str(&self.text_runs(&text, &Rp::default()));
                }
                Inline::Unknown => {}
            }
        }
        out
    }

    fn formula(&self, latex: &str) -> String {
        let latex = latex.trim();
        if latex.is_empty() {
            return String::new();
        }
        self.input.mathml.get(latex).and_then(|m| math::omml(m)).unwrap_or_else(|| math::fallback(latex))
    }

    /// A paragraph; plain ones take the surrounding style (quote, table cell).
    fn para(&self, mut pp: Pp, runs: &str) -> String {
        if pp.style.is_none() {
            pp.style = self.context_style.map(str::to_string);
        }
        format!("<w:p>{}{runs}</w:p>", pp.xml())
    }

    /// A paragraph in the given direction.
    fn para_dir(&self, style: Option<&str>, rtl: bool, runs: &str) -> String {
        self.para(Pp { style: style.map(str::to_string), bidi: self.bidi(rtl), ..Default::default() }, runs)
    }

    fn page_break(&self) -> String {
        "<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>".into()
    }

    /// The direction of text: that of its first letter.
    fn text_is_rtl(&self, s: &str) -> bool {
        s.chars().find(|c| c.is_alphabetic()).map_or(self.rtl, is_arabic)
    }

    // ---------------------------------------------------------------- body

    fn body(&mut self) -> String {
        let mut o = String::new();
        let doc = self.input.doc;
        let mut main_section = self.section(false);
        match self.front_matter(&mut o) {
            Front::None | Front::FirstPage => {}
            // The front matter's own section: its page numbers (i, ii…) and no
            // number on the title page.
            Front::OwnSection => o.push_str(&self.para(Pp { sect: Some(self.section(true)), ..Default::default() }, "")),
            // An article's title block over two columns: a one-column section for it,
            // then the columns on the same page, numbered on from it.
            Front::FirstPageSpanning => {
                o.push_str(&self.para(Pp { sect: Some(self.section_with(false, 1, false, true)), ..Default::default() }, ""));
                main_section = self.section_with(false, 2, true, false);
            }
        }
        let blocks = doc.body.content.clone();
        for b in &blocks {
            self.block(b, &mut o, self.rtl, 0);
        }
        self.bibliography(&mut o);
        o.push_str(&main_section);
        o
    }

    fn bibliography(&mut self, o: &mut String) {
        if self.bibliography_done || self.references.is_empty() {
            return;
        }
        self.bibliography_done = true;
        let title = self.t.bibliography.title.clone();
        let style = if self.t.bibliography.in_toc { "Heading1" } else { "FrontHeading" };
        o.push_str(&self.para_dir(Some(style), self.text_is_rtl(&title), &self.text_runs(&title, &Rp::default())));
        let numbered = refs::style_of(self.t) != BibStyle::AuthorYear;
        let references = self.references.clone();
        // Two lists (Arabic theses): entries in the document's language, then the
        // others, each under its own heading; numbers run on.
        let (own, other) = refs::split_by_language(&references, self.langs.main);
        let split = self.t.bibliography.split_by_language && !own.is_empty() && !other.is_empty();
        let words = texloom_codegen::strings::words(self.langs.main);
        for (i, r) in references.iter().enumerate() {
            if split && (i == 0 || i == own.len()) {
                let heading = if i == 0 { words.own_references } else { words.other_references };
                o.push_str(&self.para_dir(Some("Heading2"), self.rtl, &self.text_runs(heading, &Rp::default())));
            }
            let mut runs = String::new();
            if numbered {
                runs.push_str(&self.text_runs(&format!("[{}] ", local_digits(&(i + 1).to_string(), self.langs.digits)), &Rp::default()));
            }
            // `*…*` marks the italic parts.
            for (k, part) in r.text.split('*').enumerate() {
                runs.push_str(&self.text_runs(part, &Rp { italic: k % 2 == 1, ..Default::default() }));
            }
            o.push_str(&self.para_dir(Some("Reference"), r.lang.rtl(), &runs));
        }
    }

    fn para_rtl(dir: Option<TextDir>, rtl: bool) -> bool {
        match dir {
            Some(TextDir::Rtl) => true,
            Some(TextDir::Ltr) => false,
            _ => rtl,
        }
    }

    fn block(&mut self, b: &Block, o: &mut String, rtl: bool, depth: u8) {
        match b {
            Block::Paragraph { attrs, content } => {
                if content.is_empty() {
                    return;
                }
                let runs = self.inlines(content);
                o.push_str(&self.para_dir(None, Self::para_rtl(attrs.dir, rtl), &runs));
            }
            Block::Heading { attrs, content } => {
                let level = usize::from(attrs.level.clamp(1, 4));
                self.next.0 += 1;
                let label = self.numbers.headings.get(self.next.0 - 1).cloned().flatten();
                let text = self.inlines(content);
                let display = self.t.headings.levels.get(level - 1).is_some_and(|h| h.layout == texloom_core::HeadingLayout::Display);
                let runs = match label {
                    Some(l) if display => format!("{}<w:r><w:br/></w:r>{text}", self.text_runs(&l, &Rp::default())),
                    Some(l) => format!("{}{text}", self.text_runs(&format!("{l}  "), &Rp::default())),
                    None => text,
                };
                o.push_str(&self.para_dir(Some(&format!("Heading{level}")), Self::para_rtl(attrs.dir, rtl), &runs));
            }
            Block::BulletList { content } => self.list(content, None, o, rtl, depth),
            Block::OrderedList { attrs, content } => self.list(content, Some(attrs.start), o, rtl, depth),
            Block::Blockquote { content } => {
                let saved = self.context_style.replace("Quote");
                for b in content {
                    self.block(b, o, rtl, depth);
                }
                self.context_style = saved;
            }
            Block::CodeBlock { content } => {
                let code: String = content.iter().filter_map(|i| if let Inline::Text { text, .. } = i { Some(text.as_str()) } else { None }).collect();
                for line in code.split('\n') {
                    o.push_str(&self.para_dir(Some("Code"), false, &self.text_runs(line, &Rp::default())));
                }
            }
            Block::HorizontalRule => o.push_str(&self.para(Pp { border_bottom: Some(6), ..Default::default() }, "")),
            Block::Equation { attrs } => {
                let formula = self.formula(&attrs.latex);
                let width = twips(self.line_width_mm());
                let number = if attrs.numbered {
                    self.next.3 += 1;
                    self.numbers.equations.get(self.next.3 - 1).map(|n| format!("<w:r><w:tab/></w:r>{}", self.text_runs(&format!("({n})"), &Rp::default()))).unwrap_or_default()
                } else {
                    String::new()
                };
                // Left to right, the formula centred and its number at the right edge
                // (as in the PDF).
                let pp = Pp {
                    tabs: vec![("center", width / 2), ("right", width)],
                    bidi: self.bidi(false),
                    ind: Some("w:firstLine=\"0\"".into()),
                    ..Default::default()
                };
                o.push_str(&self.para(pp, &format!("<w:r><w:tab/></w:r>{formula}{number}")));
            }
            Block::Figure { attrs, content } => {
                self.next.1 += 1;
                let width = self.line_width_mm() * attrs.width_percent.clamp(5.0, 100.0) / 100.0;
                let crop = attrs.crop.and_then(|c| c.trim());
                let picture = (self.input.image)(&attrs.src).and_then(|bytes| self.picture(&attrs.src, bytes, width, crop));
                let runs = picture.unwrap_or_else(|| self.text_runs(&format!("[{}]", attrs.src), &Rp::italic()));
                let pp = Pp { keep_next: true, ind: Some("w:firstLine=\"0\"".into()), jc: Some("center"), ..Default::default() };
                o.push_str(&self.para(pp, &runs));
                let label = format!("{} {}", self.t.captions.figure_label, self.numbers.figures.get(self.next.1 - 1).cloned().unwrap_or_default());
                let caption: Vec<Inline> = content.iter().flat_map(|c| c.content.clone()).collect();
                let runs = format!("{}{}", self.text_runs(&format!("{label}{}", caption_sep(self.t)), &Rp::bold()), self.inlines(&caption));
                o.push_str(&self.para_dir(Some("Caption"), rtl, &runs));
            }
            Block::TableFigure { content, .. } => {
                self.next.2 += 1;
                let label = format!("{} {}", self.t.captions.table_label, self.numbers.tables.get(self.next.2 - 1).cloned().unwrap_or_default());
                for part in content {
                    match part {
                        TableFigurePart::Caption { content } => {
                            let runs = format!("{}{}", self.text_runs(&format!("{label}{}", caption_sep(self.t)), &Rp::bold()), self.inlines(content));
                            let pp = Pp { style: Some("Caption".into()), keep_next: true, bidi: self.bidi(rtl), ..Default::default() };
                            o.push_str(&self.para(pp, &runs));
                        }
                        TableFigurePart::Table { content } => self.table(content, o, rtl),
                        _ => {}
                    }
                }
            }
            Block::Table { content } => self.table(content, o, rtl),
            Block::AppendixStart => {
                // The reference list comes before the appendices, as in the PDF.
                self.bibliography(o);
                o.push_str(&self.page_break());
            }
            Block::RawLatex { attrs } => {
                let gray = Rp { color: Some("888888".into()), ..Default::default() };
                for line in attrs.latex.lines() {
                    o.push_str(&self.para_dir(Some("Code"), false, &self.text_runs(line, &gray)));
                }
            }
            Block::Columns { content } | Block::Slide { content, .. } | Block::Column { content, .. } | Block::SlideBox { content, .. } => {
                for b in content {
                    self.block(b, o, rtl, depth);
                }
            }
            Block::SlideTitle { content } => {
                let runs = self.inlines(content);
                o.push_str(&self.para_dir(Some("Heading2"), rtl, &runs));
            }
            Block::SlidePause | Block::Unknown => {}
        }
    }

    fn list(&mut self, items: &[texloom_core::ListItem], start: Option<u32>, o: &mut String, rtl: bool, depth: u8) {
        let num_id = match start {
            None => 1,
            Some(s) => {
                // Each numbered list counts from its own start.
                let id = 2 + self.lists.len();
                self.lists.push((id, s));
                id
            }
        };
        for item in items {
            let mut first = true;
            for b in &item.content {
                match b {
                    Block::Paragraph { attrs, content } if first => {
                        let runs = self.inlines(content);
                        let pp = Pp {
                            style: Some("ListParagraph".into()),
                            num: Some((depth.min(8), num_id)),
                            bidi: self.bidi(Self::para_rtl(attrs.dir, rtl)),
                            ..Default::default()
                        };
                        o.push_str(&self.para(pp, &runs));
                        first = false;
                    }
                    Block::BulletList { content } => self.list(content, None, o, rtl, depth + 1),
                    Block::OrderedList { attrs, content } => self.list(content, Some(attrs.start), o, rtl, depth + 1),
                    other => self.block(other, o, rtl, depth + 1),
                }
            }
        }
    }

    /// A table: equal columns across the text width, merged cells as in the editor.
    fn table(&mut self, rows: &[TableRow], o: &mut String, rtl: bool) {
        // Grid: which cell covers each row and column (cells spanning rows take
        // their columns in the rows below).
        let mut slots: Vec<Vec<Option<(usize, usize)>>> = vec![Vec::new(); rows.len()];
        for (r, row) in rows.iter().enumerate() {
            let mut col = 0;
            for (i, cell) in row.content.iter().enumerate() {
                while slots[r].get(col).is_some_and(Option::is_some) {
                    col += 1;
                }
                let cs = cell.attrs.colspan.max(1) as usize;
                let rs = (cell.attrs.rowspan.max(1) as usize).min(rows.len() - r);
                for line in &mut slots[r..r + rs] {
                    if line.len() < col + cs {
                        line.resize(col + cs, None);
                    }
                    for s in &mut line[col..col + cs] {
                        *s = Some((r, i));
                    }
                }
                col += cs;
            }
        }
        let columns = slots.iter().map(Vec::len).max().unwrap_or(0);
        if columns == 0 {
            return;
        }
        let width = twips(self.line_width_mm());
        let col_w = width / columns as i64;
        let bidi = if rtl { "<w:bidiVisual/>" } else { "" };
        o.push_str(&format!("<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/>{bidi}<w:tblW w:w=\"{width}\" w:type=\"dxa\"/><w:tblLayout w:type=\"fixed\"/></w:tblPr><w:tblGrid>"));
        for _ in 0..columns {
            o.push_str(&format!("<w:gridCol w:w=\"{col_w}\"/>"));
        }
        o.push_str("</w:tblGrid>");
        let saved = self.context_style.replace("TableText");
        for r in 0..rows.len() {
            o.push_str("<w:tr>");
            let mut col = 0;
            while col < columns {
                let Some((cr, ci)) = slots[r].get(col).copied().flatten() else {
                    o.push_str(&format!("<w:tc><w:tcPr><w:tcW w:w=\"{col_w}\" w:type=\"dxa\"/></w:tcPr><w:p/></w:tc>"));
                    col += 1;
                    continue;
                };
                let cell = &rows[cr].content[ci];
                let cs = cell.attrs.colspan.max(1) as usize;
                let span = if cs > 1 { format!("<w:gridSpan w:val=\"{cs}\"/>") } else { String::new() };
                let tc_w = col_w * cs as i64;
                if cr < r {
                    // Covered by a cell from above.
                    o.push_str(&format!("<w:tc><w:tcPr><w:tcW w:w=\"{tc_w}\" w:type=\"dxa\"/>{span}<w:vMerge/></w:tcPr><w:p/></w:tc>"));
                } else {
                    let merge = if cell.attrs.rowspan > 1 { "<w:vMerge w:val=\"restart\"/>" } else { "" };
                    let mut inner = String::new();
                    self.bold_text = cell.is_header();
                    for b in &cell.content {
                        self.block(b, &mut inner, rtl, 0);
                    }
                    self.bold_text = false;
                    if inner.is_empty() {
                        inner = "<w:p/>".into();
                    }
                    o.push_str(&format!("<w:tc><w:tcPr><w:tcW w:w=\"{tc_w}\" w:type=\"dxa\"/>{span}{merge}</w:tcPr>{inner}</w:tc>"));
                }
                col += cs;
            }
            o.push_str("</w:tr>");
        }
        self.context_style = saved;
        o.push_str("</w:tbl><w:p/>");
    }

    /// An inline picture `width_mm` wide, cut by `crop` (left, bottom, right, top
    /// fractions), or `None` for files Word can't show (PDF).
    fn picture(&mut self, name: &str, bytes: Vec<u8>, width_mm: f32, crop: Option<[f32; 4]>) -> Option<String> {
        let (px_w, px_h) = image_size(&bytes)?;
        let ext = if bytes.starts_with(b"\x89PNG") { "png" } else { "jpeg" };
        let [l, b, r, t] = crop.unwrap_or([0.0; 4]);
        let visible_w = px_w as f32 * (1.0 - l - r);
        let visible_h = px_h as f32 * (1.0 - t - b);
        let height_mm = width_mm * visible_h / visible_w.max(1.0);
        let n = self.media.len() + 1;
        let rid = format!("rIdImage{n}");
        let file = format!("image{n}.{ext}");
        self.media.push((rid.clone(), file.clone(), bytes));
        let id = self.drawing_id;
        self.drawing_id += 1;
        let (cx, cy) = (emu(width_mm), emu(height_mm));
        let rect = if crop.is_some() {
            let k = |f: f32| (f * 100_000.0).round() as i64;
            format!("<a:srcRect l=\"{}\" t=\"{}\" r=\"{}\" b=\"{}\"/>", k(l), k(t), k(r), k(b))
        } else {
            String::new()
        };
        Some(format!(
            "<w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{id}\" name=\"{}\"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect=\"1\"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"{id}\" name=\"{file}\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"{rid}\"/>{rect}<a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>",
            esc(name)
        ))
    }

    // ---------------------------------------------------------------- front matter

    /// The value of a field as text (lists one per line).
    fn field(&self, id: &str) -> Option<Vec<String>> {
        let date = |id: &str| {
            let f = self.t.field(id).filter(|f| f.kind == FieldKind::Date)?;
            match self.input.doc.metadata.get(&f.id)? {
                FieldValue::Text(v) if !v.trim().is_empty() => Some(v.clone()),
                _ => None,
            }
        };
        // A date's Hijri form ({defense_date_hijri}), and dates in the document's language.
        if let Some(base) = id.strip_suffix(texloom_core::date::HIJRI_SUFFIX)
            && let Some(v) = date(base)
        {
            return texloom_core::date::format_hijri(&v, self.langs.main, self.langs.digits).map(|h| vec![h]);
        }
        if let Some(v) = date(id) {
            return Some(vec![texloom_core::date::format(&v, self.langs.main, self.langs.digits)]);
        }
        let v = self.input.doc.metadata.get(id).filter(|v| !v.is_empty())?;
        Some(match v {
            FieldValue::Text(s) => vec![s.clone()],
            FieldValue::List(l) => l.clone(),
            FieldValue::Authors(a) => a.iter().map(|a| a.name.clone()).collect(),
        })
    }

    /// Template text with its placeholders filled ("Advisor: {advisor}").
    /// Placeholders filled in; list entries joined by `sep` (a line each on title
    /// pages, a run of names in an article's title block).
    fn fill(&self, text: &str, sep: &str) -> String {
        let mut out = String::new();
        for piece in texloom_core::placeholder::parse(text) {
            match piece {
                texloom_core::placeholder::Piece::Literal(l) => out.push_str(&l),
                texloom_core::placeholder::Piece::Name(n) => out.push_str(&self.field(n).map(|v| v.join(sep)).unwrap_or_default()),
            }
        }
        out
    }

    /// Writes the front matter; says how the main text follows it.
    fn front_matter(&mut self, o: &mut String) -> Front {
        if self.t.base.is_some() {
            self.title_block(o);
            return Front::None;
        }
        let items = self.t.front_matter.clone();
        let block = self.t.title_page.block_height_mm;
        for item in &items {
            match item {
                FrontMatterItem::TitlePage => {
                    let items = self.t.title_page.items.clone();
                    self.placed_page(&items, block, o);
                }
                FrontMatterItem::FixedPage { items, .. } => self.placed_page(items, None, o),
                FrontMatterItem::FieldPage { title, field, in_toc, language, inline } => {
                    let Some(values) = self.field(field) else { continue };
                    let rtl = language.map_or(self.rtl, Language::rtl);
                    let style = if *in_toc { "Heading1" } else { "FrontHeading" };
                    o.push_str(&self.para_dir(Some(style), rtl, &self.text_runs(title, &Rp::default())));
                    for p in values.join("\n\n").split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
                        o.push_str(&self.para_dir(None, rtl, &self.text_runs(p, &Rp::default())));
                    }
                    if !*inline {
                        o.push_str(&self.page_break());
                    }
                }
                FrontMatterItem::TableOfContents { title } => {
                    o.push_str(&self.para_dir(Some("FrontHeading"), self.rtl, &self.text_runs(title, &Rp::default())));
                    let hint = if self.rtl { "حدّث الحقل لعرض المحتويات (F9)" } else { "Update this field to show the contents (F9)." };
                    let pp = Pp { bidi: self.bidi(self.rtl), ..Default::default() };
                    o.push_str(&format!(
                        "<w:p>{}<w:r><w:fldChar w:fldCharType=\"begin\" w:dirty=\"true\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> TOC \\o \"1-3\" \\h \\z \\u </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>{}<w:r><w:fldChar w:fldCharType=\"end\"/></w:r></w:p>",
                        pp.xml(),
                        self.text_runs(hint, &Rp::default())
                    ));
                    o.push_str(&self.page_break());
                }
                // Word builds these from captions it numbers itself; the figure and
                // table numbers here are written out, so they're left out.
                FrontMatterItem::ListOfFigures { .. } | FrontMatterItem::ListOfTables { .. } => {}
            }
        }
        // As in the LaTeX class: a title block and the items under it, alone, share
        // the first page with the main text.
        let on_first_page = |item: &FrontMatterItem| match item {
            FrontMatterItem::TitlePage => block.is_some(),
            FrontMatterItem::FieldPage { inline, .. } => *inline,
            _ => false,
        };
        if items.is_empty() {
            Front::None
        } else if !items.iter().all(on_first_page) {
            Front::OwnSection
        } else if self.t.page.columns == 2 {
            Front::FirstPageSpanning
        } else {
            Front::FirstPage
        }
    }

    /// A page laid out at exact positions (title page, approval page), as
    /// paragraphs from top to bottom. With `block` (an article's title block) the
    /// text follows on the same page, starting that far below the paper's top edge.
    fn placed_page(&mut self, items: &[TitleItem], block: Option<f32>, o: &mut String) {
        let mut sorted: Vec<&TitleItem> = items.iter().collect();
        sorted.sort_by(|a, b| a.y_mm.total_cmp(&b.y_mm));
        let mut y = self.t.page.margins.top_mm;
        for item in sorted {
            let before = pt_twips((item.y_mm - y).max(0.0) * 72.0 / 25.4);
            match &item.content {
                TitleContent::Text { text, style, .. } => {
                    let sep = match block {
                        None => "\n",
                        Some(_) if self.rtl => "، ",
                        Some(_) => ", ",
                    };
                    let filled = self.fill(text, sep);
                    if filled.trim().is_empty() {
                        continue;
                    }
                    let rtl = self.text_is_rtl(&filled);
                    let rp = Rp {
                        size: Some(half_points(style.size_pt)),
                        bold: style.bold,
                        italic: style.italic,
                        small_caps: style.small_caps,
                        caps: style.uppercase,
                        ..Default::default()
                    };
                    let mut runs = String::new();
                    for (i, line) in filled.split('\n').enumerate() {
                        if i > 0 {
                            runs.push_str("<w:r><w:br/></w:r>");
                        }
                        runs.push_str(&self.text_runs(line, &rp));
                    }
                    let pp = Pp {
                        bidi: self.bidi(rtl),
                        before: Some(before),
                        after: Some(0),
                        ind: Some("w:firstLine=\"0\"".into()),
                        jc: Some(jc(style.align, rtl)),
                        ..Default::default()
                    };
                    o.push_str(&self.para(pp, &runs));
                    let lines = filled.split('\n').count() as f32;
                    y = item.y_mm + lines * style.size_pt * 1.25 * 25.4 / 72.0;
                }
                TitleContent::Image { asset, width_mm } => {
                    if let Some(pic) = (self.input.asset)(asset).and_then(|b| self.picture(asset, b, *width_mm, None)) {
                        let pp = Pp { before: Some(before), after: Some(0), ind: Some("w:firstLine=\"0\"".into()), jc: Some("center"), ..Default::default() };
                        o.push_str(&self.para(pp, &pic));
                        y = item.y_mm + width_mm * 0.6;
                    }
                }
                TitleContent::Rule { length_mm, thickness_pt } => {
                    let side = twips(((self.text_width_mm() - length_mm) / 2.0).max(0.0));
                    let pp = Pp {
                        border_bottom: Some((thickness_pt * 8.0).round().max(2.0) as i64),
                        before: Some(before),
                        after: Some(0),
                        line: Some(120),
                        line_exact: true,
                        ind: Some(format!("w:left=\"{side}\" w:right=\"{side}\" w:firstLine=\"0\"")),
                        ..Default::default()
                    };
                    o.push_str(&self.para(pp, ""));
                    y = item.y_mm + 2.0;
                }
            }
        }
        match block {
            Some(height) => {
                let before = pt_twips((height - y).max(0.0) * 72.0 / 25.4);
                let pp = Pp { before: Some(before), after: Some(0), line: Some(20), line_exact: true, ..Default::default() };
                o.push_str(&self.para(pp, ""));
            }
            None => o.push_str(&self.page_break()),
        }
    }

    /// The title block of templates built on a publisher class: the fields in order.
    fn title_block(&mut self, o: &mut String) {
        let fields = self.t.fields.clone();
        for f in &fields {
            if f.kind == FieldKind::Image {
                continue;
            }
            let Some(values) = self.field(&f.id) else { continue };
            let joined = values.join(if self.rtl { "، " } else { ", " });
            let rtl = self.text_is_rtl(&joined);
            if f.id.starts_with("title") {
                o.push_str(&self.para_dir(Some("Title"), rtl, &self.text_runs(&joined, &Rp::default())));
                continue;
            }
            match f.kind {
                FieldKind::Authors => {
                    o.push_str(&self.para_dir(Some("Subtitle"), rtl, &self.text_runs(&joined, &Rp::default())));
                    if let Some(FieldValue::Authors(a)) = self.input.doc.metadata.get(&f.id) {
                        let mut affs: Vec<String> = Vec::new();
                        for author in a {
                            for aff in &author.affiliations {
                                if !affs.contains(aff) {
                                    affs.push(aff.clone());
                                }
                            }
                        }
                        for aff in affs {
                            let small = Rp { size: Some(20), ..Default::default() };
                            o.push_str(&self.para_dir(Some("Subtitle"), self.text_is_rtl(&aff), &self.text_runs(&aff, &small)));
                        }
                    }
                }
                FieldKind::Paragraphs => {
                    let rtl = self.text_is_rtl(&values.join(" "));
                    // "Abstract (English)" → "Abstract": the label without its note.
                    let label = if rtl { f.label.split(" (").next().unwrap_or(&f.label).to_string() } else { f.label.split('(').nth(1).map(|s| s.trim_end_matches(')').to_string()).unwrap_or_else(|| f.label.clone()) };
                    o.push_str(&self.para_dir(Some("FrontHeading"), rtl, &self.text_runs(&label, &Rp::default())));
                    for p in values.join("\n\n").split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
                        o.push_str(&self.para_dir(Some("Abstract"), rtl, &self.text_runs(p, &Rp::default())));
                    }
                }
                _ => {
                    let label = if rtl { f.label.split(" (").next().unwrap_or(&f.label).to_string() } else { f.label.split('(').nth(1).map(|s| s.trim_end_matches(')').to_string()).unwrap_or_else(|| f.label.clone()) };
                    let runs = format!("{}{}", self.text_runs(&format!("{label}: "), &Rp::bold()), self.text_runs(&joined, &Rp::default()));
                    o.push_str(&self.para_dir(None, rtl, &runs));
                }
            }
        }
    }

    // ---------------------------------------------------------------- page setup

    /// Section properties: the front matter's (`front`) or the main text's.
    fn section(&self, front: bool) -> String {
        self.section_with(front, if front { 1 } else { self.t.page.columns }, false, true)
    }

    /// Section properties: `continuous` starts the section on the same page as the
    /// one before, `restart` numbers its pages from 1.
    fn section_with(&self, front: bool, columns: u8, continuous: bool, restart: bool) -> String {
        let (w, h) = self.t.page.paper.size_mm();
        let m = &self.t.page.margins;
        let inner = m.inner_mm + self.t.page.binding_offset_mm;
        // The binding margin is on the right of right-to-left single pages; two-sided
        // documents mirror the margins (with the gutter on the right when RTL).
        let (left, right) = if self.rtl && !self.t.page.two_sided { (m.outer_mm, inner) } else { (inner, m.outer_mm) };
        let style = if front { self.t.structure.front_matter_page_numbers } else { self.t.structure.main_page_numbers };
        let fmt = match style {
            // Word shows Arabic-Indic digits in Arabic text by its own setting
            // ("hindiNumbers" would be Devanagari digits).
            PageNumberStyle::Arabic => "decimal",
            PageNumberStyle::LowerRoman => "lowerRoman",
            PageNumberStyle::UpperRoman => "upperRoman",
        };
        let header = if self.has_running_header() && !front { "<w:headerReference w:type=\"default\" r:id=\"rIdHeader\"/>" } else { "" };
        let title_pg = if front { "<w:titlePg/>" } else { "" };
        let bidi = if self.rtl { "<w:bidi/>" } else { "" };
        let gutter = if self.rtl && self.t.page.two_sided { "<w:rtlGutter/>" } else { "" };
        let kind = if continuous { "<w:type w:val=\"continuous\"/>" } else { "" };
        let start = if restart { " w:start=\"1\"" } else { "" };
        // The same 6 mm gap as the LaTeX class.
        let cols = if columns == 2 { format!("<w:cols w:num=\"2\" w:space=\"{}\"/>", twips(6.0)) } else { String::new() };
        format!(
            "<w:sectPr>{header}<w:footerReference w:type=\"default\" r:id=\"rIdFooter\"/>{kind}<w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" w:header=\"567\" w:footer=\"567\" w:gutter=\"0\"/><w:pgNumType w:fmt=\"{fmt}\"{start}/>{cols}{title_pg}{bidi}{gutter}</w:sectPr>",
            twips(w),
            twips(h),
            twips(m.top_mm),
            twips(right),
            twips(m.bottom_mm),
            twips(left)
        )
    }

    fn has_running_header(&self) -> bool {
        let h = &self.t.header_footer.main.header;
        self.t.base.is_none() && [&h.left, &h.center, &h.right].iter().any(|s| s.contains("{chapter}"))
    }

    // ---------------------------------------------------------------- the package

    fn package(&self, body: &str) -> Result<Vec<u8>, String> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let deflate = zip::write::SimpleFileOptions::default();
        let mut put = |name: &str, data: &[u8]| -> Result<(), String> {
            zip.start_file(name, deflate).map_err(|e| e.to_string())?;
            zip.write_all(data).map_err(|e| e.to_string())
        };
        let has_png = self.media.iter().any(|(_, f, _)| f.ends_with(".png"));
        let has_jpeg = self.media.iter().any(|(_, f, _)| f.ends_with(".jpeg"));
        let mut types = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>"#);
        if has_png {
            types.push_str(r#"<Default Extension="png" ContentType="image/png"/>"#);
        }
        if has_jpeg {
            types.push_str(r#"<Default Extension="jpeg" ContentType="image/jpeg"/>"#);
        }
        for (part, ct) in [
            ("/word/document.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"),
            ("/word/styles.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"),
            ("/word/settings.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"),
            ("/word/numbering.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"),
            ("/word/footer1.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"),
            ("/word/header1.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"),
        ] {
            types.push_str(&format!(r#"<Override PartName="{part}" ContentType="{ct}"/>"#));
        }
        types.push_str("</Types>");
        put("[Content_Types].xml", types.as_bytes())?;
        put(
            "_rels/.rels",
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        )?;
        let rel = |id: &str, kind: &str, target: &str| format!(r#"<Relationship Id="{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}" Target="{}"/>"#, esc(target));
        let mut rels = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
        rels.push_str(&rel("rIdStyles", "styles", "styles.xml"));
        rels.push_str(&rel("rIdSettings", "settings", "settings.xml"));
        rels.push_str(&rel("rIdNumbering", "numbering", "numbering.xml"));
        rels.push_str(&rel("rIdFooter", "footer", "footer1.xml"));
        rels.push_str(&rel("rIdHeader", "header", "header1.xml"));
        for (id, file, _) in &self.media {
            rels.push_str(&rel(id, "image", &format!("media/{file}")));
        }
        for (id, url) in &self.links {
            rels.push_str(&format!(r#"<Relationship Id="{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="{}" TargetMode="External"/>"#, esc(url)));
        }
        rels.push_str("</Relationships>");
        put("word/_rels/document.xml.rels", rels.as_bytes())?;
        let document = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {}><w:body>{body}</w:body></w:document>"#, xml::DOCUMENT_NS);
        put("word/document.xml", document.as_bytes())?;
        put("word/styles.xml", self.styles().as_bytes())?;
        put("word/settings.xml", self.settings().as_bytes())?;
        put("word/numbering.xml", self.numbering().as_bytes())?;
        put("word/footer1.xml", self.footer().as_bytes())?;
        put("word/header1.xml", self.header().as_bytes())?;
        for (_, file, bytes) in &self.media {
            put(&format!("word/media/{file}"), bytes)?;
        }
        let cursor = zip.finish().map_err(|e| e.to_string())?;
        Ok(cursor.into_inner())
    }

    fn field_paragraph(&self, instr: &str) -> String {
        let pp = Pp { bidi: self.bidi(self.rtl), ind: Some("w:firstLine=\"0\"".into()), jc: Some("center"), ..Default::default() };
        format!(
            "<w:p>{}<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> {instr} </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType=\"end\"/></w:r></w:p>",
            pp.xml()
        )
    }

    fn footer(&self) -> String {
        format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:ftr {}>{}</w:ftr>"#, xml::DOCUMENT_NS, self.field_paragraph("PAGE"))
    }

    fn header(&self) -> String {
        // The chapter's title, from its heading (Word's STYLEREF field). Without a
        // top-level heading the field would print Word's "Error! No text…".
        let has_heading = self.input.doc.body.content.iter().any(|b| matches!(b, Block::Heading { attrs, .. } if attrs.level == 1));
        let content = if has_heading { self.field_paragraph("STYLEREF \"heading 1\"") } else { "<w:p/>".to_string() };
        format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr {}>{content}</w:hdr>"#, xml::DOCUMENT_NS)
    }

    fn settings(&self) -> String {
        let mirror = if self.t.page.two_sided { "<w:mirrorMargins/>" } else { "" };
        // In the schema's order.
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">{mirror}<w:defaultTabStop w:val="720"/><w:characterSpacingControl w:val="doNotCompress"/><w:updateFields w:val="true"/><m:mathPr><m:mathFont m:val="Cambria Math"/></m:mathPr><w:themeFontLang w:val="en-US" w:bidi="ar-SA"/></w:settings>"#
        )
    }

    fn numbering(&self) -> String {
        let level = |i: usize, fmt: &str, text: &str| {
            let indent = 360 * (i as i64 + 1);
            format!("<w:lvl w:ilvl=\"{i}\"><w:start w:val=\"1\"/><w:numFmt w:val=\"{fmt}\"/><w:lvlText w:val=\"{text}\"/><w:lvlJc w:val=\"start\"/><w:pPr><w:ind w:start=\"{indent}\" w:hanging=\"360\"/></w:pPr></w:lvl>")
        };
        let bullets: String = (0..9).map(|i| level(i, "bullet", ["•", "◦", "▪"][i % 3])).collect();
        // As LaTeX's enumerate: 1. (a) i. A., with abjad letters in Arabic (ا ب ج).
        let numbers: String = (0..9)
            .map(|i| match i {
                1 if self.rtl => level(i, "arabicAbjad", "(%2)"),
                1 => level(i, "lowerLetter", "(%2)"),
                2 => level(i, "lowerRoman", "%3."),
                3 => level(i, "upperLetter", "%4."),
                _ => level(i, "decimal", &format!("%{}.", i + 1)),
            })
            .collect();
        let mut o = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="hybridMultilevel"/>{bullets}</w:abstractNum><w:abstractNum w:abstractNumId="1"><w:multiLevelType w:val="hybridMultilevel"/>{numbers}</w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>"#
        );
        for (id, start) in &self.lists {
            o.push_str(&format!("<w:num w:numId=\"{id}\"><w:abstractNumId w:val=\"1\"/><w:lvlOverride w:ilvl=\"0\"><w:startOverride w:val=\"{start}\"/></w:lvlOverride></w:num>"));
        }
        o.push_str("</w:numbering>");
        o
    }

    fn styles(&self) -> String {
        let t = self.t;
        let latin = esc(&word_font(&t.fonts.main, Role::Main));
        let mono = word_font(&t.fonts.mono, Role::Mono);
        let arabic = esc(&match &t.fonts.arabic {
            Some(FontChoice::System { family }) if !family.trim().is_empty() => family.clone(),
            // Amiri isn't on most Word users' machines; Traditional Arabic is
            // Windows' classic naskh.
            _ => "Traditional Arabic".into(),
        });
        let size = half_points(f32::from(t.fonts.base_size_pt));
        let line = match t.page.line_spacing {
            LineSpacing::Single => 240,
            LineSpacing::OneHalf => 360,
            LineSpacing::Double => 480,
            LineSpacing::Custom { stretch } => (240.0 * stretch).round() as i64,
        };
        let defaults = Pp { bidi: self.rtl.then_some(true), after: Some(pt_twips(t.page.paragraph.spacing_pt.max(0.0))), line: Some(line), ..Default::default() };
        let mut o = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="{latin}" w:hAnsi="{latin}" w:eastAsia="{latin}" w:cs="{arabic}"/><w:sz w:val="{size}"/><w:szCs w:val="{size}"/><w:lang w:val="en-US" w:eastAsia="en-US" w:bidi="ar-SA"/></w:rPr></w:rPrDefault><w:pPrDefault>{}</w:pPrDefault></w:docDefaults>"#,
            defaults.xml()
        );
        let style = |o: &mut String, id: &str, name: &str, based: bool, pp: Pp, rp: Rp| {
            let based = if based { "<w:basedOn w:val=\"Normal\"/>" } else { "" };
            let next = if id.starts_with("Heading") { "<w:next w:val=\"Normal\"/>" } else { "" };
            o.push_str(&format!(r#"<w:style w:type="paragraph" w:styleId="{id}"><w:name w:val="{name}"/>{based}{next}<w:qFormat/>{}{}</w:style>"#, pp.xml(), rp.xml()));
        };
        let ind0 = || Some("w:firstLine=\"0\"".to_string());
        o.push_str(&format!(
            r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/>{}</w:style>"#,
            Pp { ind: Some(format!("w:firstLine=\"{}\"", twips(t.page.paragraph.indent_mm.max(0.0)))), jc: Some("both"), ..Default::default() }.xml()
        ));
        for (i, h) in t.headings.levels.iter().take(4).enumerate() {
            let s = &h.text;
            let pp = Pp {
                keep_next: true,
                page_break_before: i == 0 && t.structure.top_level == TopLevel::Chapter,
                before: Some(pt_twips(h.space_before_pt)),
                after: Some(pt_twips(h.space_after_pt)),
                ind: ind0(),
                jc: Some(jc(s.align, self.rtl)),
                outline: Some(i),
                ..Default::default()
            };
            let rp = Rp {
                bold: s.bold,
                italic: s.italic,
                caps: s.uppercase,
                color: s.color.as_ref().map(|c| c.trim_start_matches('#').to_string()),
                size: Some(half_points(s.size_pt)),
                ..Default::default()
            };
            style(&mut o, &format!("Heading{}", i + 1), &format!("heading {}", i + 1), true, pp, rp);
        }
        let top = t.headings.levels.first().map(|h| half_points(h.text.size_pt)).unwrap_or(size + 8);
        let cap_size = match t.captions.size {
            texloom_core::CaptionSize::Footnote => size - 4,
            texloom_core::CaptionSize::Small => size - 2,
            texloom_core::CaptionSize::Normal => size,
        };
        let cap_jc = match t.captions.align {
            texloom_core::CaptionAlign::Center => "center",
            texloom_core::CaptionAlign::Left => "start",
            texloom_core::CaptionAlign::Justified => "both",
        };
        let centred = |before: i64, after: i64| Pp { before: Some(before), after: Some(after), ind: ind0(), jc: Some("center"), ..Default::default() };
        style(&mut o, "Title", "Title", true, centred(0, 240), Rp { bold: true, size: Some(top + 4), ..Default::default() });
        style(&mut o, "Subtitle", "Subtitle", true, centred(0, 60), Rp::default());
        style(&mut o, "FrontHeading", "Front Matter Heading", true, Pp { keep_next: true, ..centred(240, 240) }, Rp { bold: true, size: Some(top), ..Default::default() });
        style(&mut o, "Abstract", "Abstract", true, Pp { ind: Some("w:left=\"567\" w:right=\"567\" w:firstLine=\"0\"".into()), ..Default::default() }, Rp::default());
        style(&mut o, "Caption", "caption", true, Pp { before: Some(60), after: Some(200), ind: ind0(), jc: Some(cap_jc), ..Default::default() }, Rp { size: Some(cap_size), ..Default::default() });
        style(&mut o, "Quote", "Quote", true, Pp { ind: Some("w:left=\"720\" w:right=\"720\" w:firstLine=\"0\"".into()), ..Default::default() }, Rp::italic());
        style(&mut o, "Code", "Code", true, Pp { after: Some(0), line: Some(240), ind: ind0(), jc: Some("left"), ..Default::default() }, Rp { font: Some(mono), size: Some(size - 2), ..Default::default() });
        style(&mut o, "ListParagraph", "List Paragraph", true, Pp { after: Some(60), ..Default::default() }, Rp::default());
        style(&mut o, "TableText", "Table Text", true, Pp { after: Some(0), line: Some(240), ind: ind0(), jc: Some("start"), ..Default::default() }, Rp::default());
        style(&mut o, "Reference", "Reference", true, Pp { after: Some(120), ind: Some("w:start=\"567\" w:hanging=\"567\"".into()), jc: Some("start"), ..Default::default() }, Rp::default());
        o.push_str(r#"<w:style w:type="character" w:styleId="Hyperlink"><w:name w:val="Hyperlink"/><w:rPr><w:color w:val="0563C1"/><w:u w:val="single"/></w:rPr></w:style><w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders><w:tblCellMar><w:left w:w="108" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style>"#);
        o.push_str("</w:styles>");
        o
    }
}

fn caption_sep(t: &Template) -> &'static str {
    match t.captions.separator {
        texloom_core::CaptionSeparator::Colon => ": ",
        texloom_core::CaptionSeparator::Period => ". ",
        texloom_core::CaptionSeparator::Dash => " — ",
        texloom_core::CaptionSeparator::Newline => " ",
    }
}

/// Word's paragraph alignment for a physical alignment. In right-to-left
/// paragraphs Word mirrors "left" and "right", so they're swapped there.
fn jc(align: Align, rtl: bool) -> &'static str {
    match (align, rtl) {
        (Align::Center, _) => "center",
        (Align::Left, false) | (Align::Right, true) => "left",
        (Align::Right, false) | (Align::Left, true) => "right",
    }
}

#[derive(Clone, Copy)]
enum Role {
    Main,
    Mono,
}

/// The font Word uses for a template font: TeX's fonts by their usual look-alikes.
fn word_font(f: &FontChoice, role: Role) -> String {
    match f {
        FontChoice::System { family } => family.clone(),
        FontChoice::Builtin { font } => match font {
            BuiltinFont::LatinModern => match role {
                Role::Main => "Cambria",
                Role::Mono => "Consolas",
            },
            BuiltinFont::Termes => "Times New Roman",
            BuiltinFont::Pagella => "Palatino Linotype",
            BuiltinFont::Bonum => "Bookman Old Style",
            BuiltinFont::Schola => "Century Schoolbook",
            BuiltinFont::Heros => "Arial",
            BuiltinFont::Adventor => "Century Gothic",
            BuiltinFont::Cursor => "Courier New",
            BuiltinFont::Amiri => "Traditional Arabic",
        }
        .into(),
    }
}

/// Pixel size of a PNG or JPEG.
fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.starts_with(b"\x89PNG") {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let reader = decoder.read_info().ok()?;
        let info = reader.info();
        return Some((info.width, info.height));
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        // JPEG: the size is in the first start-of-frame marker.
        let mut i = 2;
        while i + 9 < bytes.len() {
            if bytes[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = bytes[i + 1];
            let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
            if (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker) {
                let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                return (w > 0 && h > 0).then_some((w, h));
            }
            i += 2 + len;
        }
    }
    None
}

#[cfg(test)]
mod tests;
