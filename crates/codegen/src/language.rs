//! Languages other than English, and Arabic in particular (written right to left).
//!
//! An Arabic document gets polyglossia (which loads bidi): the Arabic font as the
//! main font, English as the other language with the template's Latin fonts, and
//! Arabic-Indic or Western digits. English documents with some Arabic in them get
//! Arabic as their other language. Words in the other script are wrapped
//! (`\textenglish{…}` inside an Arabic paragraph, `\textarabic{…}` inside an
//! English one) so they keep their own direction: authors never mark them.
//!
//! The setup goes into the `.tex` preamble (not the class), because whether an
//! English template's document contains Arabic is only known from the document.

use texloom_core::{
    walk_inlines, BuiltinFont, Digits, Document, FieldValue, FontChoice, Inline, Language, Template, TextDir, TopLevel,
};

use crate::latex::escape;

thread_local! {
    static LR_NUMBERS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` writing numbers in right-to-left text in `\LR{…}`, for LuaLaTeX: unlike
/// XeTeX, LuaTeX doesn't order digits itself and sets them backwards there ("2021"
/// as "1202", "٣٣–٥٢" scrambled). XeTeX output is left as it is.
pub fn with_lr_numbers<T>(on: bool, f: impl FnOnce() -> T) -> T {
    let before = LR_NUMBERS.with(|c| c.replace(on));
    let out = f();
    LR_NUMBERS.with(|c| c.set(before));
    out
}

fn lr_numbers() -> bool {
    LR_NUMBERS.with(|c| c.get())
}

/// A digit, Western or Arabic-Indic (or Persian).
fn is_digit(c: char) -> bool {
    c.is_ascii_digit() || ('\u{0660}'..='\u{0669}').contains(&c) || ('\u{06F0}'..='\u{06F9}').contains(&c)
}

/// The languages a document uses: the template's, plus Arabic when an English
/// document contains Arabic text or right-to-left paragraphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Languages {
    pub main: Language,
    pub others: Vec<Language>,
    pub digits: Digits,
}

impl Languages {
    pub fn of(t: &Template, doc: &Document) -> Self {
        let settings = t.lang();
        let mut others = settings.others();
        if settings.main != Language::Arabic && !others.contains(&Language::Arabic) && has_arabic(doc) {
            others.push(Language::Arabic);
        }
        Self { main: settings.main, others, digits: settings.digits }
    }

    /// The template's own languages (for its fixed texts: labels, headers, titles).
    pub fn of_template(t: &Template) -> Self {
        let settings = t.lang();
        Self { main: settings.main, others: settings.others(), digits: settings.digits }
    }

    /// Arabic text can only be typeset by XeTeX and LuaTeX (not pdfLaTeX).
    pub fn need_unicode_engine(&self) -> bool {
        self.main == Language::Arabic || self.others.contains(&Language::Arabic)
    }

    /// A number Texloom writes as text (a citation's year or number) in the
    /// document's digits: Arabic-Indic in an Arabic document that uses them, as
    /// polyglossia writes page and heading numbers ("2016" → "٢٠١٦").
    pub fn local_digits(&self, s: &str) -> String {
        if self.main != Language::Arabic || self.digits != Digits::ArabicIndic {
            return s.to_string();
        }
        s.chars().map(|c| c.to_digit(10).filter(|_| c.is_ascii_digit()).and_then(|n| char::from_u32(0x0660 + n)).unwrap_or(c)).collect()
    }

    /// English only: nothing to set up, the output stays as it always was.
    pub fn english_only(&self) -> bool {
        self.main == Language::English && self.others.is_empty()
    }

    /// The language of a paragraph with the editor's direction `dir`, inside text in
    /// `current` (a paragraph without a direction of its own stays in it).
    pub fn for_dir(&self, dir: Option<TextDir>, current: Language) -> Language {
        let wanted_rtl = match dir {
            Some(TextDir::Rtl) => true,
            Some(TextDir::Ltr) => false,
            _ => return current,
        };
        if current.rtl() == wanted_rtl {
            return current;
        }
        std::iter::once(self.main).chain(self.others.iter().copied()).find(|l| l.rtl() == wanted_rtl).unwrap_or(current)
    }

    /// The language a script's words are set in, if the document has it.
    fn of_script(&self, script: Script) -> Option<Language> {
        let lang = match script {
            Script::Latin => Language::English,
            Script::Arabic => Language::Arabic,
        };
        (self.main == lang || self.others.contains(&lang)).then_some(lang)
    }
}

/// Arabic letters anywhere in the document's text or metadata, or a paragraph set
/// right to left.
fn has_arabic(doc: &Document) -> bool {
    let mut found = false;
    walk_inlines(&doc.body.content, &mut |i| match i {
        Inline::Text { text, .. } => found |= text.chars().any(is_arabic),
        // Arabic in a formula needs the Arabic font too.
        Inline::InlineMath { attrs } => found |= attrs.latex.chars().any(is_arabic),
        _ => {}
    });
    texloom_core::walk_blocks(&doc.body.content, &mut |b| match b {
        texloom_core::Block::Paragraph { attrs, .. } => found |= attrs.dir == Some(TextDir::Rtl),
        texloom_core::Block::Heading { attrs, .. } => found |= attrs.dir == Some(TextDir::Rtl),
        texloom_core::Block::Equation { attrs } => found |= attrs.latex.chars().any(is_arabic),
        _ => {}
    });
    found
        || doc.metadata.values().any(|v| match v {
            FieldValue::Text(s) => s.chars().any(is_arabic),
            FieldValue::List(l) => l.iter().any(|s| s.chars().any(is_arabic)),
            FieldValue::Authors(a) => a.iter().any(|a| a.name.chars().any(is_arabic) || a.affiliations.iter().any(|s| s.chars().any(is_arabic))),
        })
        // A cited reference in Arabic: its entry in the list needs the Arabic font and
        // direction too (without them it would print as missing characters).
        || (!doc.bibliography.trim().is_empty() && {
            let cited = crate::refs::cited_keys(doc);
            texloom_core::bib::parse(&doc.bibliography).entries.iter().any(|e| cited.contains(&e.key) && crate::refs::entry_language(e) == Language::Arabic)
        })
}

/// A formula's LaTeX made ready for Arabic: the math fonts have no Arabic letters
/// (TeX drops them with only a warning), so Arabic runs in math are set as text in
/// the Arabic font and direction (`\text{\textarabic{…}}`). With Arabic-Indic digits,
/// numbers in math are written with them too (`3.5` → `٣٫٥`), as the document's own
/// numbers are. Text already in `\text{…}`, and arguments that are lengths, labels or
/// names (`\hspace{2pt}`, `\label{…}`, `\begin{array}`), are left alone.
pub fn math(latex: &str, langs: &Languages) -> String {
    const TEXT: [&str; 10] = ["text", "textrm", "textbf", "textit", "textsf", "texttt", "textnormal", "mbox", "textarabic", "textenglish"];
    const RAW: [&str; 16] = [
        "hspace", "vspace", "kern", "mkern", "hskip", "mskip", "rule", "raisebox", "label", "ref", "eqref", "begin", "end", "color", "mathcolor", "tag",
    ];
    #[derive(Clone, Copy, PartialEq)]
    enum Group {
        Math,
        Text,
        Raw,
    }
    let indic = langs.digits == texloom_core::Digits::ArabicIndic && langs.main == Language::Arabic;
    let chars: Vec<char> = latex.chars().collect();
    let mut out = String::with_capacity(latex.len());
    let mut stack = vec![Group::Math];
    // The kind of group the next `{` opens (after \text, \hspace…).
    let mut next = Group::Math;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let here = *stack.last().unwrap_or(&Group::Math);
        match c {
            '\\' => {
                // A command: \name, or a control symbol (\{, \\).
                let start = i;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                if i == start + 1 && i < chars.len() {
                    i += 1;
                }
                let name: String = chars[start + 1..i].iter().collect();
                out.extend(&chars[start..i]);
                next = if TEXT.contains(&name.as_str()) {
                    Group::Text
                } else if RAW.contains(&name.as_str()) {
                    Group::Raw
                } else {
                    Group::Math
                };
                // \\[2pt]: the optional length.
                if name == "\\" && chars.get(i) == Some(&'[') {
                    while i < chars.len() && chars[i] != ']' {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                continue;
            }
            '{' => {
                stack.push(if here == Group::Math { next } else { here });
                next = Group::Math;
                out.push(c);
            }
            '}' => {
                if stack.len() > 1 {
                    stack.pop();
                }
                out.push(c);
            }
            c if here == Group::Math && is_arabic(c) => {
                // The whole run, spaces between Arabic words included.
                let start = i;
                while i < chars.len() && (is_arabic(chars[i]) || (chars[i] == ' ' && chars.get(i + 1).is_some_and(|n| is_arabic(*n)))) {
                    i += 1;
                }
                let run: String = chars[start..i].iter().collect();
                // A word (not a one-letter variable) gets spaces, which math ignores when typed.
                if run.chars().filter(|c| is_arabic(*c)).count() > 1 {
                    out.push_str(&format!("\\text{{ \\textarabic{{{run}}} }}"));
                } else {
                    out.push_str(&format!("\\text{{\\textarabic{{{run}}}}}"));
                }
                next = Group::Math;
                continue;
            }
            // Arabic in \text{…} of a document whose font has no Arabic: the Arabic font.
            c if here == Group::Text && langs.main != Language::Arabic && is_arabic(c) => {
                let start = i;
                while i < chars.len() && (is_arabic(chars[i]) || (chars[i] == ' ' && chars.get(i + 1).is_some_and(|n| is_arabic(*n)))) {
                    i += 1;
                }
                let run: String = chars[start..i].iter().collect();
                out.push_str(&format!("\\textarabic{{{run}}}"));
                continue;
            }
            c if here == Group::Math && indic && c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || (chars[i] == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit))) {
                    i += 1;
                }
                let digits: String = chars[start..i]
                    .iter()
                    .map(|&d| if d == '.' { '\u{066B}' } else { char::from_u32(0x0660 + d.to_digit(10).unwrap_or(0)).unwrap_or(d) })
                    .collect();
                // Braced, so it stays one argument (x^2, \frac{1}{2}).
                out.push_str(&format!("{{\\text{{{digits}}}}}"));
                next = Group::Math;
                continue;
            }
            c => {
                if !c.is_whitespace() {
                    next = Group::Math;
                }
                out.push(c);
            }
        }
        i += 1;
    }
    out
}

/// Abjad numerals, as polyglossia prints LaTeX's letter counters (\alph, \Alph) in
/// Arabic: ا ب ج د ه و ز ح ط ي, then يا (11), ك (20)…
pub fn abjad(n: usize) -> String {
    const VALUES: [(usize, char); 28] = [
        (1000, 'غ'), (900, 'ظ'), (800, 'ض'), (700, 'ذ'), (600, 'خ'), (500, 'ث'), (400, 'ت'), (300, 'ش'), (200, 'ر'), (100, 'ق'),
        (90, 'ص'), (80, 'ف'), (70, 'ع'), (60, 'س'), (50, 'ن'), (40, 'م'), (30, 'ل'), (20, 'ك'), (10, 'ي'),
        (9, 'ط'), (8, 'ح'), (7, 'ز'), (6, 'و'), (5, 'ه'), (4, 'د'), (3, 'ج'), (2, 'ب'), (1, 'ا'),
    ];
    let mut rest = n;
    let mut out = String::new();
    for (value, letter) in VALUES {
        while rest >= value {
            out.push(letter);
            rest -= value;
        }
    }
    out
}

pub fn is_arabic(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Script {
    Latin,
    Arabic,
}

/// How a character takes part in direction: a letter of a script, a digit (which
/// follows the letters before it, as in Unicode's bidi rules), or neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Strong(Script),
    Digit,
    Neutral,
}

fn class(c: char) -> Class {
    if is_arabic(c) {
        // Arabic-Indic digits (U+0660–0669, U+06F0–06F9) count as Arabic letters here.
        Class::Strong(Script::Arabic)
    } else if c.is_ascii_digit() {
        Class::Digit
    } else if c.is_alphabetic() {
        Class::Strong(Script::Latin)
    } else {
        Class::Neutral
    }
}

/// A piece of a paragraph in one script: text of inline node `node`, or LaTeX
/// already written (formulas, citations), which is neutral.
pub(crate) struct Piece {
    pub node: usize,
    pub text: String,
    pub latex: Option<String>,
    pub script: Option<Script>,
    /// Ends a run of words in another script: numbers TeX writes itself (page,
    /// citation, cross-reference numbers) follow the document's digits, which the
    /// other language's font may not have (Arabic-Indic digits in a Latin font).
    pub fence: bool,
    /// A number with separators ("54.2", "45–70", "1/1/2025"): read left to right as
    /// one unit, also in right-to-left text (as Unicode's bidi rules do; XeTeX
    /// doesn't apply them, and "54.2" would come out "2.54").
    pub number: bool,
}

impl Piece {
    /// LaTeX that writes numbers of its own: stays outside runs in another script.
    pub fn latex(node: usize, latex: String) -> Self {
        Self { node, text: String::new(), latex: Some(latex), script: None, fence: true, number: false }
    }

    /// LaTeX that can go inside a run (a formula: its digits are the maths font's).
    pub fn neutral(node: usize, latex: String) -> Self {
        Self { node, text: String::new(), latex: Some(latex), script: None, fence: false, number: false }
    }
}

/// Splits the text of inline node `node` into pieces of one script (spaces and
/// punctuation are neutral; digits follow the letters before them, as in Unicode's
/// bidi rules: "Windows 11").
pub(crate) fn split(node: usize, text: &str, last: &mut Option<Script>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    let mut current = String::new();
    let mut current_script: Option<Option<Script>> = None;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // A number with separators, not after Latin letters: one piece of its own.
        if c.is_ascii_digit() && *last != Some(Script::Latin) {
            let mut end = i;
            let mut separated = false;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
                if end + 1 < chars.len() && ".,:/-–".contains(chars[end]) && chars[end + 1].is_ascii_digit() {
                    separated = true;
                    end += 1;
                }
            }
            if separated {
                if !current.is_empty() {
                    out.push(Piece { node, text: std::mem::take(&mut current), latex: None, script: current_script.flatten(), fence: false, number: false });
                }
                current_script = None;
                out.push(Piece { node, text: chars[i..end].iter().collect(), latex: None, script: None, fence: false, number: true });
                i = end;
                continue;
            }
        }
        i += 1;
        let script = match class(c) {
            Class::Strong(s) => {
                *last = Some(s);
                Some(s)
            }
            // A digit after Latin letters belongs to them ("Windows 11"); otherwise
            // it stays neutral and follows the paragraph.
            Class::Digit if *last == Some(Script::Latin) => Some(Script::Latin),
            Class::Digit | Class::Neutral => None,
        };
        if current_script.is_some_and(|s| s != script) {
            out.push(Piece { node, text: std::mem::take(&mut current), latex: None, script: current_script.flatten(), fence: false, number: false });
        }
        current_script = Some(script);
        current.push(c);
    }
    if !current.is_empty() {
        out.push(Piece { node, text: current, latex: None, script: current_script.flatten(), fence: false, number: false });
    }
    out
}

/// LaTeX for consecutive pieces: the text of each inline node rendered once
/// (`render(node, text)` escapes it and applies its bold/italic marks). In
/// right-to-left text, numbers with separators are kept left to right (set as
/// English: bidi's `\LR` doesn't keep "54.2" together, a language switch does).
fn render_pieces(pieces: &[Piece], rtl: bool, render: &dyn Fn(usize, &str) -> String) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < pieces.len() {
        if let Some(latex) = &pieces[i].latex {
            out.push_str(latex);
            i += 1;
            continue;
        }
        if pieces[i].number && rtl {
            out.push_str(&format!("\\textenglish{{{}}}", render(pieces[i].node, &pieces[i].text)));
            i += 1;
            continue;
        }
        let node = pieces[i].node;
        let mut text = String::new();
        while i < pieces.len() && pieces[i].latex.is_none() && pieces[i].node == node && !(pieces[i].number && rtl) {
            text.push_str(&pieces[i].text);
            i += 1;
        }
        if rtl && lr_numbers() {
            for (digits, part) in digit_runs(&text) {
                if digits {
                    out.push_str(&format!("\\LR{{{}}}", render(node, part)));
                } else {
                    out.push_str(&render(node, part));
                }
            }
        } else {
            out.push_str(&render(node, &text));
        }
    }
    out
}

/// `text` cut into numbers and the rest: (is a number, run). A number takes in the
/// separators between its digits ("٢٫٣١", "١٨(٢):٣٣–٥٢", "1/5/2025"), so it's one
/// left-to-right unit, with a closing bracket that balances one in it.
fn digit_runs(text: &str) -> Vec<(bool, &str)> {
    const SEPARATORS: &str = "٫٬.,:/-–()[]";
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut start = 0;
    let mut k = 0;
    while k < chars.len() {
        if !is_digit(chars[k].1) {
            k += 1;
            continue;
        }
        let from = chars[k].0;
        let mut end = k + 1;
        loop {
            while end < chars.len() && is_digit(chars[end].1) {
                end += 1;
            }
            // Separators count when another digit follows them.
            let mut next = end;
            while next < chars.len() && SEPARATORS.contains(chars[next].1) {
                next += 1;
            }
            if next > end && next < chars.len() && is_digit(chars[next].1) {
                end = next;
            } else {
                break;
            }
        }
        let run: String = chars[k..end].iter().map(|c| c.1).collect();
        let open = |o: char, c: char| run.matches(o).count() > run.matches(c).count();
        if end < chars.len() && ((chars[end].1 == ')' && open('(', ')')) || (chars[end].1 == ']' && open('[', ']'))) {
            end += 1;
        }
        let to = chars.get(end).map_or(text.len(), |c| c.0);
        if from > start {
            out.push((false, &text[start..from]));
        }
        out.push((true, &text[from..to]));
        start = to;
        k = end;
    }
    if start < text.len() {
        out.push((false, &text[start..]));
    }
    out
}

/// Joins the pieces of a paragraph written in `lang`, wrapping each run of words in
/// another script, from its first letter to its last (with the spaces, digits and
/// punctuation between them), in that language's command.
pub(crate) fn join(pieces: &[Piece], lang: Language, langs: &Languages, render: &dyn Fn(usize, &str) -> String) -> String {
    let foreign = |p: &Piece| p.script.and_then(|s| langs.of_script(s)).filter(|l| *l != lang);
    let mut out = String::new();
    let mut i = 0;
    while i < pieces.len() {
        let Some(other) = foreign(&pieces[i]) else {
            // Up to the next piece in another language, as one stretch.
            let start = i;
            while i < pieces.len() && foreign(&pieces[i]).is_none() {
                i += 1;
            }
            out.push_str(&render_pieces(&pieces[start..i], lang.rtl(), render));
            continue;
        };
        // The run ends at its last piece in that language, before anything in another.
        let mut end = i;
        let mut j = i + 1;
        while j < pieces.len() {
            if pieces[j].fence {
                break;
            }
            match pieces[j].script {
                None => {}
                Some(_) if foreign(&pieces[j]) == Some(other) => end = j,
                Some(_) => break,
            }
            j += 1;
        }
        let inner = render_pieces(&pieces[i..=end], other.rtl(), render);
        out.push_str(&format!("\\text{}{{{inner}}}", other.polyglossia()));
        i = end + 1;
    }
    out
}

/// Plain text (a field value) in a paragraph of `lang`, escaped, with words in
/// another script wrapped. Text wholly in the other language (an English abstract)
/// is one run from its first character to its last: its punctuation belongs to it
/// ("…in Ramadan." keeps its period after "Ramadan" in right-to-left text).
pub fn text(s: &str, lang: Language, langs: &Languages) -> String {
    if langs.english_only() {
        return escape(s);
    }
    let mut last = None;
    let pieces = split(0, s, &mut last);
    let mut written: Vec<Language> = pieces.iter().filter_map(|p| p.script).filter_map(|s| langs.of_script(s)).collect();
    written.dedup();
    if let [other] = written[..] {
        if other != lang {
            let inner = render_pieces(&pieces, other.rtl(), &|_, t| escape(t));
            return format!("\\text{}{{{inner}}}", other.polyglossia());
        }
    }
    join(&pieces, lang, langs, &|_, t| escape(t))
}

/// The preamble lines that set up the languages and their fonts; empty for English
/// documents. Goes right after `\documentclass`, and wins over the class's fonts.
pub fn preamble(t: &Template, langs: &Languages) -> String {
    if langs.english_only() {
        return String::new();
    }
    let (mut arabic, ar) = arabic_font(t);
    let mut o = String::new();
    let names: Vec<&str> = std::iter::once(langs.main).chain(langs.others.iter().copied()).map(Language::polyglossia).collect();
    o.push_str(&format!("% Languages: {} (compile with XeLaTeX or LuaLaTeX)\n", names.join(", ")));
    // Publisher classes and beamer don't load it themselves.
    o.push_str("\\usepackage{fontspec}\n");
    if let Some(FontChoice::System { family }) = &t.fonts.arabic
        && !family.trim().is_empty()
    {
        // A system font may not be installed where the document is compiled (Office's
        // Diwani fonts, say): Amiri, which every TeX distribution has, rather than an error.
        o.push_str(&format!("\\IfFontExistsTF{{{arabic}}}{{\\def\\texloomarabicfont{{{arabic}}}}}{{\\def\\texloomarabicfont{{Amiri}}}}\n"));
        arabic = "\\texloomarabicfont".into();
    }
    let numerals = match langs.digits {
        Digits::ArabicIndic => "numerals=mashriq",
        Digits::Western => "numerals=maghrib",
    };
    if langs.main == Language::Arabic {
        // The Arabic font is the main font: headings and \normalfont use it too.
        // English text switches to the template's Latin fonts.
        o.push_str(&format!("\\setmainfont[Script=Arabic{ar}]{{{arabic}}}\n"));
        // Not quite the same options as the main font: identical ones make one font
        // family, and polyglossia would then take body text for sans (English
        // words would come out in the sans font).
        o.push_str(&format!("\\setsansfont[Script=Arabic,WordSpace=1{ar}]{{{arabic}}}\n"));
        for (cmd, choice, role) in [("englishfont", &t.fonts.main, Role::Main), ("englishfontsf", &t.fonts.sans, Role::Sans), ("englishfonttt", &t.fonts.mono, Role::Mono)] {
            let line = format!("\\newfontfamily\\{cmd}{}", latin_spec(choice, role));
            match choice {
                // Not installed where the document is compiled: Latin Modern instead.
                FontChoice::System { family } => {
                    let lm = latin_spec(&FontChoice::Builtin { font: BuiltinFont::LatinModern }, role);
                    o.push_str(&format!("\\IfFontExistsTF{{{}}}{{{line}}}{{\\newfontfamily\\{cmd}{lm}}}\n", escape(family)));
                }
                _ => o.push_str(&format!("{line}\n")),
            }
        }
        // Arabic in code font (newer polyglossia insists every style can show Arabic).
        o.push_str(&format!("\\newfontfamily\\arabicfonttt[Script=Arabic,Ligatures=TeX{ar}]{{{arabic}}}\n"));
    } else {
        o.push_str(&format!("\\newfontfamily\\arabicfont[Script=Arabic,Ligatures=TeX{ar}]{{{arabic}}}\n"));
        o.push_str(&format!("\\newfontfamily\\arabicfontsf[Script=Arabic,Ligatures=TeX{ar}]{{{arabic}}}\n"));
        o.push_str(&format!("\\newfontfamily\\arabicfonttt[Script=Arabic,Ligatures=TeX{ar}]{{{arabic}}}\n"));
    }
    o.push_str("\\usepackage{polyglossia}\n");
    if lr_numbers() {
        // Numbers in right-to-left text go in \LR (luabidi's), so LuaTeX keeps their
        // digits in order.
        o.push_str("\\AtBeginDocument{\\providecommand\\LR[1]{#1}}\n");
    }
    let options = |l: Language| if l == Language::Arabic { format!("[{numerals}]") } else { String::new() };
    o.push_str(&format!("\\setdefaultlanguage{}{{{}}}\n", options(langs.main), langs.main.polyglossia()));
    for l in &langs.others {
        o.push_str(&format!("\\setotherlanguage{}{{{}}}\n", options(*l), l.polyglossia()));
    }
    o.push_str(&arabic_symbol_fallbacks());
    // Arabic as the main language: polyglossia's Arabic sets \theequation to the bare
    // count whenever it becomes the main language (at the start too), so equations
    // read "(١)" next to "شكل ١.١". With per-chapter numbering, the chapter goes back
    // in front, now and each time polyglossia sets it again.
    if langs.main == Language::Arabic && t.structure.top_level == TopLevel::Chapter && t.structure.per_chapter_numbering {
        o.push_str("\\makeatletter\n\\providecommand\\@SepMark{.}\n");
        o.push_str("\\def\\tl@chapterequation{\\@ifundefined{c@chapter}{}{\\renewcommand\\theequation{\\ifnum\\c@chapter>\\z@\\thechapter\\@SepMark\\fi\\@arabic\\c@equation}}}\n");
        o.push_str("\\ifdefined\\arabic@globalnumbers\\gappto\\arabic@globalnumbers{\\tl@chapterequation}\\fi\n");
        o.push_str("\\tl@chapterequation\\AtBeginDocument{\\tl@chapterequation}\n\\makeatother\n");
    }
    // polyglossia sets its own "Figure"/"شكل" when the document starts; the
    // template's labels win.
    let labels = format!(
        "\\renewcommand\\figurename{{{}}}\\renewcommand\\tablename{{{}}}",
        escape(&t.captions.figure_label),
        escape(&t.captions.table_label)
    );
    o.push_str(&format!("\\gappto\\captions{}{{{labels}}}\n", langs.main.polyglossia()));
    // bidi (loaded by polyglossia, after the class) brings back the standard chapter
    // and section headings; the template's go on top again.
    if t.base.is_none() && t.kind != texloom_core::DocumentKind::Presentation {
        o.push_str("\\makeatletter\n");
        o.push_str(&crate::cls::heading_formats(t, langs));
        o.push_str("\\makeatother\n");
    }
    o
}

/// Some Arabic fonts have no Arabic decimal or thousands separator or percent sign
/// (Aref Ruqaa, Reem Kufi, Tajawal…), and TeX would drop them: ٢٫٥ would print "٢٥".
/// Each prints itself where the current font has it, else a stand-in every font
/// has: the Arabic comma for the decimal separator (also used for decimals), a thin
/// space between thousands, "%" for the percent sign.
fn arabic_symbol_fallbacks() -> String {
    let mut o = String::from("% Arabic separators and percent sign, with stand-ins for fonts that lack them\n");
    for (code, fallback) in [("066B", "\\char\"060C "), ("066C", "\\,"), ("066A", "\\char\"25 ")] {
        o.push_str(&format!(
            "\\begingroup\\lccode`\\~=\"{code}\\lowercase{{\\endgroup\\protected\\def~}}{{\\iffontchar\\font\"{code} \\char\"{code} \\else{fallback}\\fi}}\\catcode\"{code}=13\n"
        ));
    }
    o
}

/// The font for Arabic text, as fontspec takes it: the name, and options to add.
fn arabic_font(t: &Template) -> (String, String) {
    match &t.fonts.arabic {
        Some(FontChoice::System { family }) if !family.trim().is_empty() => (escape(family.trim()), String::new()),
        // From Texloom's font library: by file, from the build folder's fonts/.
        Some(FontChoice::Library { id }) if texloom_core::fonts::find(id).is_some() => {
            let f = texloom_core::fonts::find(id).expect("checked");
            (f.regular_file(), format!(",{}", crate::cls::library_options(f)))
        }
        // Amiri comes with every TeX distribution (Tectonic's bundle too).
        _ => ("Amiri".into(), String::new()),
    }
}

#[derive(Clone, Copy)]
enum Role {
    Main,
    Sans,
    Mono,
}

/// `{name}[options]` for a Latin font, loaded by file name when it's built in (no
/// system fonts needed, as for the class's own fonts).
fn latin_spec(choice: &FontChoice, role: Role) -> String {
    let files = |stem: &str| format!("{{{stem}}}[Extension=.otf,Ligatures=TeX,UprightFont=*-regular,BoldFont=*-bold,ItalicFont=*-italic,BoldItalicFont=*-bolditalic]");
    match choice {
        FontChoice::System { family } => format!("{{{}}}[Ligatures=TeX]", escape(family)),
        FontChoice::Library { id } => match texloom_core::fonts::find(id) {
            Some(f) => format!("{{{}}}[Ligatures=TeX,{}]", f.regular_file(), crate::cls::library_options(f)),
            None => "{Amiri}[Ligatures=TeX]".into(),
        },
        FontChoice::Builtin { font: BuiltinFont::Amiri } => "{Amiri}[Ligatures=TeX]".into(),
        FontChoice::Builtin { font: BuiltinFont::LatinModern } => match role {
            Role::Main => files("lmroman10"),
            Role::Sans => "{lmsans10}[Extension=.otf,Ligatures=TeX,UprightFont=*-regular,BoldFont=*-bold,ItalicFont=*-oblique,BoldItalicFont=*-boldoblique]".into(),
            Role::Mono => "{lmmono10}[Extension=.otf,UprightFont=*-regular,ItalicFont=*-italic]".into(),
        },
        FontChoice::Builtin { font } => files(crate::cls::gyre_file(*font).unwrap_or("texgyretermes")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_and_digits_in_math() {
        let indic = Languages { main: Language::Arabic, others: vec![Language::English], digits: texloom_core::Digits::ArabicIndic };
        let western = Languages { digits: texloom_core::Digits::Western, ..indic.clone() };
        // Arabic letters become text in the Arabic font; words stay together.
        assert_eq!(math("س + ص = ع", &western), "\\text{\\textarabic{س}} + \\text{\\textarabic{ص}} = \\text{\\textarabic{ع}}");
        assert_eq!(math("x > 0 إذا كان", &western), "x > 0 \\text{ \\textarabic{إذا كان} }");
        // Already text: left alone in an Arabic document; given the Arabic font in an English one.
        assert_eq!(math("\\text{حيث} x", &western), "\\text{حيث} x");
        let mixed = Languages { main: Language::English, others: vec![Language::Arabic], digits: Default::default() };
        assert_eq!(math("\\text{حيث} x", &mixed), "\\text{\\textarabic{حيث}} x");
        // Digits: Arabic-Indic, braced so they stay one argument; lengths and labels untouched.
        assert_eq!(math("x^2 + 3.5", &indic), "x^{\\text{٢}} + {\\text{٣٫٥}}");
        assert_eq!(math("\\frac{1}{2}\\hspace{2pt}\\label{e1}", &indic), "\\frac{{\\text{١}}}{{\\text{٢}}}\\hspace{2pt}\\label{e1}");
        assert_eq!(math("a \\\\[4pt] b_{10}", &indic), "a \\\\[4pt] b_{{\\text{١٠}}}");
        assert_eq!(math("x^2 + 3.5", &western), "x^2 + 3.5", "Western digits: unchanged");
        let english = Languages { main: Language::English, others: vec![], digits: Default::default() };
        assert_eq!(math("\\int_0^1 f(x)\\,dx", &english), "\\int_0^1 f(x)\\,dx");
    }

    #[test]
    fn numbers_in_the_document_digits() {
        assert_eq!(arabic_doc().local_digits("(2016a)"), "(٢٠١٦a)");
        let western = Languages { digits: Digits::Western, ..arabic_doc() };
        assert_eq!(western.local_digits("2016"), "2016");
        let english = Languages { main: Language::English, others: vec![Language::Arabic], digits: Digits::ArabicIndic };
        assert_eq!(english.local_digits("2016"), "2016", "English documents keep Western digits");
    }

    #[test]
    fn abjad_numerals() {
        let first: Vec<String> = (1..=10).map(abjad).collect();
        assert_eq!(first.join(" "), "ا ب ج د ه و ز ح ط ي");
        assert_eq!(abjad(11), "يا");
        assert_eq!(abjad(20), "ك");
        assert_eq!(abjad(1447), "غتمز");
    }

    fn arabic_doc() -> Languages {
        Languages { main: Language::Arabic, others: vec![Language::English], digits: Digits::ArabicIndic }
    }

    #[test]
    fn latin_words_in_arabic_are_wrapped_whole() {
        let l = arabic_doc();
        assert_eq!(text("نموذج deep neural networks جديد", Language::Arabic, &l), "نموذج \\textenglish{deep neural networks} جديد");
        // Digits after Latin letters belong to them; digits after Arabic don't.
        assert_eq!(text("نظام Windows 11 وعام 2024", Language::Arabic, &l), "نظام \\textenglish{Windows 11} وعام 2024");
        // Punctuation between English words stays inside, at the edges outside.
        assert_eq!(text("(انظر: Smith, J. and Lee) هنا", Language::Arabic, &l), "(انظر: \\textenglish{Smith, J. and Lee}) هنا");
        // An English paragraph with Arabic words: the other way round.
        assert_eq!(text("The word كتاب means book.", Language::English, &l), "The word \\textarabic{كتاب} means book.");
        assert_eq!(text("نص عربي فقط", Language::Arabic, &l), "نص عربي فقط");
        // Special characters are still escaped.
        assert_eq!(text("نسبة 50% من R&D", Language::Arabic, &l), "نسبة 50\\% من \\textenglish{R\\&D}");
        // Numbers with separators stay one left-to-right unit in Arabic text.
        assert_eq!(text("المتوسط 54.2 في الصفحات 45–70.", Language::Arabic, &l), "المتوسط \\textenglish{54.2} في الصفحات \\textenglish{45--70}.");
        assert_eq!(text("The mean was 54.2.", Language::English, &l), "The mean was 54.2.");
    }

    #[test]
    fn text_wholly_in_the_other_language_is_one_run() {
        let l = arabic_doc();
        // An English abstract in an Arabic document: its final period and brackets
        // are in the English run.
        assert_eq!(text("It fell by 37% in Ramadan.", Language::Arabic, &l), "\\textenglish{It fell by 37\\% in Ramadan.}");
        assert_eq!(text("(LSTM)", Language::Arabic, &l), "\\textenglish{(LSTM)}");
        // Arabic text ending in an English word keeps its own period.
        assert_eq!(text("نموذج LSTM.", Language::Arabic, &l), "نموذج \\textenglish{LSTM}.");
        // Arabic text in an English document, the other way round.
        assert_eq!(text("«مقدمة».", Language::English, &l), "\\textarabic{\\guillemotleft{}مقدمة\\guillemotright{}.}");
    }

    #[test]
    fn english_documents_are_untouched() {
        let l = Languages { main: Language::English, others: vec![], digits: Digits::Western };
        assert_eq!(text("plain & simple", Language::English, &l), "plain \\& simple");
        let t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
        assert_eq!(preamble(&t, &l), "");
    }

    #[test]
    fn arabic_preamble() {
        let mut t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
        t.captions.figure_label = "شكل".into();
        let p = preamble(&t, &arabic_doc());
        assert!(p.contains("\\setmainfont[Script=Arabic]{Amiri}"));
        assert!(p.contains("\\newfontfamily\\englishfont{lmroman10}") || p.contains("\\newfontfamily\\englishfont{texgyre"), "{p}");
        assert!(p.contains("\\setdefaultlanguage[numerals=mashriq]{arabic}\n\\setotherlanguage{english}"));
        assert!(p.contains("\\gappto\\captionsarabic{\\renewcommand\\figurename{شكل}"));
        // Stand-ins for fonts without Arabic separators or percent sign.
        assert!(
            p.contains("\\begingroup\\lccode`\\~=\"066B\\lowercase{\\endgroup\\protected\\def~}{\\iffontchar\\font\"066B \\char\"066B \\else\\char\"060C \\fi}\\catcode\"066B=13\n"),
            "{p}"
        );
        assert!(p.contains("\\iffontchar\\font\"066A \\char\"066A \\else\\char\"25 \\fi"));
        // English main with Arabic: Arabic gets its own font and the digit option.
        let mixed = Languages { main: Language::English, others: vec![Language::Arabic], digits: Digits::Western };
        let p = preamble(&t, &mixed);
        assert!(p.contains("\\newfontfamily\\arabicfont[Script=Arabic,Ligatures=TeX]{Amiri}"));
        assert!(p.contains("\\setotherlanguage[numerals=maghrib]{arabic}"));
        assert!(!p.contains("\\setmainfont"));
    }

    #[test]
    fn paragraph_directions() {
        let l = arabic_doc();
        assert_eq!(l.for_dir(Some(TextDir::Ltr), Language::Arabic), Language::English);
        assert_eq!(l.for_dir(Some(TextDir::Rtl), Language::English), Language::Arabic);
        assert_eq!(l.for_dir(None, Language::English), Language::English, "inherits");
        let english = Languages { main: Language::English, others: vec![], digits: Digits::Western };
        assert_eq!(english.for_dir(Some(TextDir::Rtl), Language::English), Language::English, "no Arabic set up: stays English");
    }
}
