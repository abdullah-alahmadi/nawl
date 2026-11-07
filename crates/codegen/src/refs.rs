//! Reference lists Texloom writes itself, for documents with Arabic.
//!
//! BibTeX can't set a right-to-left document's English entries (or an English
//! document's Arabic ones) in their own direction, so for these documents the list
//! is formatted here, each entry in its language with that language's words ("et
//! al." / "وآخرون"), in the template's style: IEEE, numeric or author-year. Other
//! documents keep BibTeX exactly as before. The Word export uses the same text.

use texloom_core::bib::{self, BibEntry};
use texloom_core::{walk_inlines, BibStyle, Document, Inline, Language, Template};

use crate::language::{self, Languages};
use crate::strings::{words, Words};

/// One formatted entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub key: String,
    /// natbib's `Author(Year)` label (author-year style).
    pub label: Option<String>,
    /// The entry as LaTeX, in its own language (not yet wrapped for direction).
    pub latex: String,
    /// The entry as plain text with `*…*` around italic parts (for Word).
    pub text: String,
    pub lang: Language,
}

/// Keys in the order they're first cited.
pub fn cited_keys(doc: &Document) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    walk_inlines(&doc.body.content, &mut |i| {
        if let Inline::Citation { attrs } = i {
            for k in &attrs.keys {
                if !keys.contains(k) {
                    keys.push(k.clone());
                }
            }
        }
    });
    keys
}

/// Whether Texloom writes the list itself: the document is Arabic, or a cited
/// entry is.
pub fn needed(doc: &Document, langs: &Languages) -> bool {
    if langs.english_only() || doc.bibliography.trim().is_empty() {
        return false;
    }
    if langs.main.rtl() {
        return true;
    }
    let cited = cited_keys(doc);
    bib::parse(&doc.bibliography).entries.iter().any(|e| cited.contains(&e.key) && entry_language(e) == Language::Arabic)
}

/// The style a template's citations and reference list use: the template's, except
/// presentations (numeric) and publisher classes with a title block of their own
/// (IEEE-like numbers; a template on a standard class with its own title block, like
/// the Arabic journal article, keeps its style and loads natbib for author-year).
pub fn style_of(t: &Template) -> BibStyle {
    match &t.base {
        _ if t.kind == texloom_core::DocumentKind::Presentation => BibStyle::Numeric,
        Some(base) if base.front_matter != texloom_core::FrontMatterStyle::Custom => BibStyle::Ieee,
        _ => t.bibliography.style,
    }
}

/// The cited entries, formatted and in list order. Sorted styles put the entries in
/// the document's language first, then the others, each by first author; `split`
/// (two lists) groups citation-ordered (IEEE) lists the same way.
pub fn references(doc: &Document, style: BibStyle, main: Language, split: bool) -> Vec<Reference> {
    let info = bib::parse(&doc.bibliography);
    let cited = cited_keys(doc);
    let mut entries: Vec<&BibEntry> = cited.iter().filter_map(|k| info.entries.iter().find(|e| &e.key == k)).collect();
    if style != BibStyle::Ieee {
        entries.sort_by_key(|e| (entry_language(e) != main, sort_key(&first_family(e)), e.year.clone().unwrap_or_default()));
    } else if split {
        // Stable: citation order within each list.
        entries.sort_by_key(|e| entry_language(e) != main);
    }
    entries.into_iter().map(|e| format(e, style)).collect()
}

/// A name as it sorts: Arabic without its article (ال), diacritics and tatweel, with
/// alef forms, taa marbuta and alef maqsura folded (أ إ آ ٱ → ا, ة → ه, ى → ي).
fn sort_key(name: &str) -> String {
    let name = name.trim();
    let name = name.strip_prefix("ال").filter(|rest| rest.chars().count() > 1).unwrap_or(name);
    name.chars()
        .filter(|c| !matches!(c, '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}'))
        .map(|c| match c {
            'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
            'ة' => 'ه',
            'ى' => 'ي',
            c => c,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

/// Splits list-ordered references where the document's language ends: the entries
/// in it, then the others (either may be empty).
pub fn split_by_language(refs: &[Reference], main: Language) -> (&[Reference], &[Reference]) {
    let at = refs.iter().position(|r| r.lang != main).unwrap_or(refs.len());
    refs.split_at(at)
}

/// The preamble lines that replace BibTeX with Texloom's list (empty when BibTeX
/// does the job).
pub fn preamble(t: &Template, doc: &Document, langs: &Languages) -> String {
    if !needed(doc, langs) {
        return String::new();
    }
    let style = style_of(t);
    let split = t.bibliography.split_by_language;
    let refs = references(doc, style, langs.main, split);
    // Numbers in citations follow the document's digits. LaTeX writes a cited
    // entry's number to the .aux in Western digits whatever the language, so with
    // Arabic-Indic digits each entry gets its number as its label.
    let indic = langs.local_digits("0") != "0";
    let item = |r: &Reference| {
        let label = match &r.label {
            // natbib reads "Author(Year)".
            Some(l) => {
                let (who, year) = l.rsplit_once('(').map(|(w, y)| (w, format!("({y}"))).unwrap_or((l.as_str(), String::new()));
                let who = if r.lang == langs.main {
                    language::text(who, langs.main, langs)
                } else {
                    // Authors in the other language are one run in it, from the
                    // first letter to the end ("Box et al." with its period).
                    format!("\\text{}{{{}}}", r.lang.polyglossia(), language::text(who, r.lang, langs))
                };
                format!("[{who}{}]", crate::latex::escape(&langs.local_digits(&year)))
            }
            None if indic => {
                let n = refs.iter().position(|x| x.key == r.key).unwrap_or(0) + 1;
                format!("[{}]", langs.local_digits(&n.to_string()))
            }
            None => String::new(),
        };
        let body = if r.lang == langs.main {
            r.latex.clone()
        } else if langs.main.rtl() && !r.lang.rtl() {
            // A left-to-right entry in a right-to-left list: its own paragraph
            // direction, so its lines start on the left with the hanging indent.
            format!("\\begin{{{}}}\\tl@ltrentry {}\\par\\end{{{}}}", r.lang.polyglossia(), r.latex, r.lang.polyglossia())
        } else {
            // An entry in the other language keeps its own direction.
            format!("\\text{}{{{}}}", r.lang.polyglossia(), r.latex)
        };
        format!("\\bibitem{label}{{{}}} {body}\n", r.key)
    };
    let mut o = String::from("% Reference list written by Texloom: BibTeX can't set entries in two directions\n\\makeatletter\n");
    if langs.main.rtl() {
        // bidi (XeTeX) keeps a right-to-left list's indents mirrored in a
        // left-to-right paragraph: the hang would be on the right and the first line
        // would stick out on the left. Shifting the lines by the list's margins puts
        // them back. (LuaTeX's paragraph directions place the lines themselves.)
        o.push_str("\\def\\tl@ltrentry{\\ifdefined\\XeTeXrevision\\leftskip\\dimexpr\\leftmargin-\\rightmargin\\relax\\rightskip-\\leftskip\\fi}\n");
    }
    let (own, other) = split_by_language(&refs, langs.main);
    if split && !own.is_empty() && !other.is_empty() {
        // Under the list's heading, a list for each language with its own heading;
        // numbers run on from the first list into the second.
        let words = words(langs.main);
        let chapters = t.base.is_none() && t.structure.top_level == texloom_core::TopLevel::Chapter;
        let (heading, group) = if chapters { ("chapter", "section") } else { ("section", "subsection") };
        o.push_str(&format!("\\def\\tl@reflist{{\\{heading}*{{\\refname}}\\@mkboth{{\\refname}}{{\\refname}}%\n"));
        o.push_str(&format!("\\def\\bibname{{\\refname}}\\let\\tl@nohead\\@gobbletwo\n"));
        for (i, (title, list)) in [(words.own_references, own), (words.other_references, other)].into_iter().enumerate() {
            o.push_str(&format!("\\{group}*{{{}}}\n", language::text(title, langs.main, langs)));
            // The lists' own headings are left out (the one above stands for both).
            o.push_str(&format!("\\begingroup\\let\\section\\tl@nohead\\let\\chapter\\tl@nohead\\begin{{thebibliography}}{{99}}\n"));
            if i == 1 {
                o.push_str(&format!("\\setcounter{{enumiv}}{{{}}}\n", own.len()));
            }
            for r in list {
                o.push_str(&item(r));
            }
            o.push_str("\\end{thebibliography}\\endgroup\n");
        }
        o.push_str("}\n");
    } else {
        o.push_str("\\def\\tl@reflist{\\begin{thebibliography}{99}\n");
        for r in &refs {
            o.push_str(&item(r));
        }
        o.push_str("\\end{thebibliography}}\n");
    }
    if style == BibStyle::AuthorYear && langs.main.rtl() {
        // Arabic punctuation between authors and years, and between citations.
        o.push_str("\\AtBeginDocument{\\setcitestyle{aysep={،},yysep={،},citesep={؛}}}\n");
    }
    // Wherever the class asks for BibTeX's list, this one comes instead.
    o.push_str("\\AtBeginDocument{\\renewcommand\\bibliography[1]{\\tl@reflist}\\renewcommand\\bibliographystyle[1]{}}\n\\makeatother\n");
    o
}

/// Arabic when the entry's authors or title are written in Arabic.
pub fn entry_language(e: &BibEntry) -> Language {
    let arabic = |s: &str| s.chars().any(language::is_arabic);
    if arabic(&e.title) || e.authors.iter().any(|a| arabic(a)) {
        Language::Arabic
    } else {
        Language::English
    }
}

fn first_family(e: &BibEntry) -> String {
    e.people.first().map(|p| clean(&p.family)).or_else(|| e.authors.first().map(|a| clean(a))).unwrap_or_default()
}

/// A BibTeX value as plain text: braces gone, escaped characters back, `--` a dash.
pub fn clean(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' | '}' => {}
            '~' => out.push(' '),
            '\\' => match chars.peek().copied() {
                Some(n) if "&%$#_{}".contains(n) => {
                    out.push(n);
                    chars.next();
                }
                // Other commands (\textit, \TeX…): the name goes, what follows stays.
                Some(n) if n.is_ascii_alphabetic() => {
                    let mut name = String::new();
                    while let Some(&n) = chars.peek().filter(|n| n.is_ascii_alphabetic()) {
                        name.push(n);
                        chars.next();
                    }
                    if name == "TeX" || name == "LaTeX" {
                        out.push_str(&name);
                    }
                }
                _ => {}
            },
            _ => out.push(c),
        }
    }
    let out = out.replace("---", "—").replace("--", "–");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Pieces of an entry, written once as LaTeX and once as text.
struct Out {
    latex: String,
    text: String,
    lang: Language,
}

impl Out {
    fn plain(&mut self, s: &str) {
        self.latex.push_str(&crate::latex::escape(s));
        self.text.push_str(s);
    }
    /// Text from the entry: words in another script keep their direction.
    fn value(&mut self, s: &str) {
        let langs = Languages { main: self.lang, others: vec![Language::English, Language::Arabic].into_iter().filter(|l| *l != self.lang).collect(), digits: Default::default() };
        self.latex.push_str(&language::text(s, self.lang, &langs));
        self.text.push_str(s);
    }
    fn italic(&mut self, s: &str) {
        let langs = Languages { main: self.lang, others: vec![Language::English, Language::Arabic].into_iter().filter(|l| *l != self.lang).collect(), digits: Default::default() };
        self.latex.push_str(&format!("\\emph{{{}}}", language::text(s, self.lang, &langs)));
        self.text.push_str(&format!("*{s}*"));
    }
    fn url(&mut self, u: &str) {
        self.latex.push_str(&format!("\\url{{{}}}", crate::latex::escape_url(u)));
        self.text.push_str(u);
    }
}

fn format(e: &BibEntry, style: BibStyle) -> Reference {
    let lang = entry_language(e);
    let w = words(lang);
    let f = |name: &str| e.fields.get(name).map(|v| clean(v)).filter(|v| !v.is_empty());
    let comma = if lang.rtl() { "، " } else { ", " };
    let names = names(e, style, lang, w);
    let year = e.year.clone().map(|y| clean(&y)).unwrap_or_else(|| w.no_date.to_string());
    let title = f("title").unwrap_or_default();
    let (open, close) = if lang.rtl() { ("«", "»") } else { ("\u{201C}", "\u{201D}") };
    let kind = e.kind.to_lowercase();
    let mut o = Out { latex: String::new(), text: String::new(), lang };
    // A list of parts joined with commas.
    let parts = |o: &mut Out, items: Vec<(bool, String)>| {
        for (i, (italic, s)) in items.into_iter().filter(|(_, s)| !s.is_empty()).enumerate() {
            if i > 0 {
                o.plain(comma);
            }
            if italic {
                o.italic(&s);
            } else {
                o.value(&s);
            }
        }
    };
    let pages = f("pages").map(|p| if p.contains('–') || p.contains('-') { format!("{} {p}", w.pages) } else { format!("{} {p}", w.page) });
    match style {
        BibStyle::Ieee => {
            if !names.is_empty() {
                o.value(&names);
                o.plain(comma);
            }
            let quoted = |o: &mut Out| {
                o.plain(open);
                o.value(&title);
                o.plain(&comma.trim_end().to_string());
                o.plain(close);
                o.plain(" ");
            };
            match kind.as_str() {
                "article" => {
                    quoted(&mut o);
                    parts(&mut o, vec![
                        (true, f("journal").unwrap_or_default()),
                        (false, f("volume").map(|v| format!("{} {v}", w.volume)).unwrap_or_default()),
                        (false, f("number").map(|v| format!("{} {v}", w.number)).unwrap_or_default()),
                        (false, pages.clone().unwrap_or_default()),
                        (false, year.clone()),
                    ]);
                }
                "inproceedings" | "conference" | "incollection" | "inbook" => {
                    quoted(&mut o);
                    o.plain(&format!("{} ", w.in_));
                    parts(&mut o, vec![
                        (true, f("booktitle").unwrap_or_default()),
                        (false, f("address").unwrap_or_default()),
                        (false, year.clone()),
                        (false, pages.clone().unwrap_or_default()),
                    ]);
                }
                "book" => {
                    o.italic(&title);
                    o.plain(comma);
                    parts(&mut o, vec![
                        (false, f("edition").map(|v| format!("{v} {}", w.edition)).unwrap_or_default()),
                        (false, [f("address"), f("publisher")].into_iter().flatten().collect::<Vec<_>>().join(": ")),
                        (false, year.clone()),
                    ]);
                }
                "phdthesis" | "mastersthesis" => {
                    quoted(&mut o);
                    parts(&mut o, vec![
                        (false, (if kind == "phdthesis" { w.phd_thesis } else { w.masters_thesis }).to_string()),
                        (false, f("school").unwrap_or_default()),
                        (false, f("address").unwrap_or_default()),
                        (false, year.clone()),
                    ]);
                }
                "techreport" => {
                    quoted(&mut o);
                    parts(&mut o, vec![
                        (false, f("institution").unwrap_or_default()),
                        (false, f("address").unwrap_or_default()),
                        (false, [Some(w.technical_report.to_string()), f("number")].into_iter().flatten().collect::<Vec<_>>().join(" ")),
                        (false, year.clone()),
                    ]);
                }
                _ => {
                    quoted(&mut o);
                    parts(&mut o, vec![(false, f("howpublished").unwrap_or_default()), (false, f("note").unwrap_or_default()), (false, year.clone())]);
                }
            }
            o.plain(".");
        }
        BibStyle::Numeric => {
            if !names.is_empty() {
                o.value(&names);
                o.plain(". ");
            }
            if kind == "book" {
                o.italic(&title);
            } else {
                o.value(&title);
            }
            o.plain(". ");
            let source = match kind.as_str() {
                "article" => {
                    let mut s = f("journal").unwrap_or_default();
                    if let Some(v) = f("volume") {
                        s.push_str(&format!("{comma}{v}"));
                        if let Some(n) = f("number") {
                            s.push_str(&format!("({n})"));
                        }
                    }
                    if let Some(p) = f("pages") {
                        s.push_str(&format!(":{p}"));
                    }
                    s
                }
                "inproceedings" | "conference" | "incollection" | "inbook" => {
                    format!("{} {}", w.in_, f("booktitle").unwrap_or_default())
                }
                "book" => [f("publisher"), f("address")].into_iter().flatten().collect::<Vec<_>>().join(comma),
                "phdthesis" | "mastersthesis" => {
                    format!("{}{comma}{}", if kind == "phdthesis" { w.phd_thesis } else { w.masters_thesis }, f("school").unwrap_or_default())
                }
                _ => f("howpublished").or_else(|| f("note")).unwrap_or_default(),
            };
            if !source.is_empty() {
                o.value(&source);
                o.plain(comma);
            }
            o.value(&year);
            o.plain(".");
        }
        BibStyle::AuthorYear => {
            if !names.is_empty() {
                o.value(&names);
                o.plain(" ");
            }
            o.plain("(");
            o.value(&year);
            o.plain("). ");
            match kind.as_str() {
                "article" => {
                    o.value(&title);
                    o.plain(". ");
                    if let Some(j) = f("journal") {
                        o.italic(&j);
                    }
                    if let Some(v) = f("volume") {
                        o.plain(comma);
                        o.italic(&v);
                        if let Some(n) = f("number") {
                            o.plain(&format!("({n})"));
                        }
                    }
                    if let Some(p) = f("pages") {
                        o.plain(comma);
                        o.value(&p);
                    }
                    o.plain(".");
                }
                "book" => {
                    o.italic(&title);
                    if let Some(ed) = f("edition") {
                        o.plain(&format!(" ({ed} {})", w.edition));
                    }
                    o.plain(". ");
                    if let Some(p) = f("publisher") {
                        o.value(&p);
                        o.plain(".");
                    }
                }
                "inproceedings" | "conference" | "incollection" | "inbook" => {
                    o.value(&title);
                    o.plain(&format!(". {} ", w.in_));
                    o.italic(&f("booktitle").unwrap_or_default());
                    if let Some(p) = f("pages") {
                        o.plain(&format!(" ({} {p})", w.pages));
                    }
                    o.plain(".");
                    if let Some(p) = f("publisher") {
                        o.plain(" ");
                        o.value(&p);
                        o.plain(".");
                    }
                }
                "phdthesis" | "mastersthesis" => {
                    o.italic(&title);
                    o.plain(" [");
                    o.value(&format!("{}{comma}{}", if kind == "phdthesis" { w.phd_thesis } else { w.masters_thesis }, f("school").unwrap_or_default()));
                    o.plain("].");
                }
                _ => {
                    o.italic(&title);
                    o.plain(".");
                    if let Some(h) = f("howpublished").or_else(|| f("note")) {
                        o.plain(" ");
                        o.value(&h);
                        o.plain(".");
                    }
                }
            }
        }
    }
    if let Some(doi) = f("doi") {
        o.plain(" ");
        o.url(&format!("https://doi.org/{}", doi.trim_start_matches("https://doi.org/")));
    } else if let Some(url) = e.fields.get("url").map(|u| u.trim().to_string()).filter(|u| !u.is_empty()) {
        o.plain(" ");
        o.url(&url);
    }
    let label = (style == BibStyle::AuthorYear).then(|| {
        let fam: Vec<String> = e.people.iter().map(|p| clean(&p.family)).collect();
        let who = match fam.len() {
            0 => clean(&title),
            1 => fam[0].clone(),
            2 if lang.rtl() => format!("{} {}{}", fam[0], w.and, fam[1]),
            2 => format!("{} {} {}", fam[0], w.and, fam[1]),
            _ => format!("{} {}", fam[0], w.et_al),
        };
        format!("{who}({})", e.year.clone().map(|y| clean(&y)).unwrap_or_else(|| w.no_date.to_string()))
    });
    Reference { key: e.key.clone(), label, latex: o.latex, text: o.text, lang }
}

/// The authors as the style writes them.
fn names(e: &BibEntry, style: BibStyle, lang: Language, w: &Words) -> String {
    let people: Vec<(String, String)> = e.people.iter().map(|p| (clean(&p.given), clean(&p.family))).collect();
    // Arabic names have no initials: always in full.
    let initials = |given: &str| {
        given
            .split_whitespace()
            .map(|g| g.split('-').map(|part| part.chars().next().map(|c| format!("{c}.")).unwrap_or_default()).collect::<Vec<_>>().join("-"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let one = |(given, family): &(String, String)| -> String {
        if given.is_empty() {
            return family.clone();
        }
        match (style, lang.rtl()) {
            (_, true) => format!("{given} {family}"),
            (BibStyle::Ieee, false) => format!("{} {family}", initials(given)),
            (BibStyle::AuthorYear, false) => format!("{family}, {}", initials(given)),
            (BibStyle::Numeric, false) => format!("{given} {family}"),
        }
    };
    let list: Vec<String> = people.iter().map(one).collect();
    let comma = if lang.rtl() { "، " } else { ", " };
    let and = match (style, lang.rtl()) {
        (_, true) => w.and.to_string(),
        (BibStyle::AuthorYear, false) => "&".into(),
        _ => w.and.to_string(),
    };
    // Arabic "و" joins the next name directly ("وأحمد").
    let join_last = |last: &str| if lang.rtl() { format!("{and}{last}") } else { format!("{and} {last}") };
    match list.len() {
        0 => String::new(),
        1 => list[0].clone(),
        n if style == BibStyle::Ieee && n > 6 => format!("{} {}", list[0], w.et_al),
        2 if style != BibStyle::AuthorYear => format!("{} {}", list[0], join_last(&list[1])),
        n => {
            let head = list[..n - 1].join(comma);
            let sep = if lang.rtl() { " " } else { comma };
            format!("{head}{sep}{}", join_last(&list[n - 1]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIB: &str = r#"
@article{smith2021, author = {Smith, John and Lee, Mary-Ann and Brown, Tom}, title = {Deep {L}earning for {IoT}}, journal = {IEEE Access}, volume = {9}, number = {2}, pages = {10--20}, year = {2021}, doi = {10.1109/x.2021}}
@book{ahmadi2020, author = {الأحمدي, عبدالله and علي, أحمد}, title = {مقدمة في الحوسبة}, publisher = {دار المعرفة}, address = {جدة}, year = {2020}}
@inproceedings{lora2019, author = {{LoRa Alliance}}, title = {LoRaWAN Specification}, booktitle = {Proc. IoT \& Systems}, pages = {5}, year = {2019}}
"#;

    fn doc() -> Document {
        let body = serde_json::json!({"type": "doc", "content": [{"type": "paragraph", "content": [
            {"type": "citation", "attrs": {"keys": ["smith2021", "ahmadi2020"]}},
            {"type": "citation", "attrs": {"keys": ["lora2019", "smith2021"]}}
        ]}]});
        Document { metadata: Default::default(), body: serde_json::from_value(body).unwrap(), bibliography: BIB.into() }
    }

    #[test]
    fn cleans_bibtex_values() {
        assert_eq!(clean("Deep {L}earning for {IoT} \\& \\textit{more}~now 10--20"), "Deep Learning for IoT & more now 10–20");
    }

    #[test]
    fn ieee_in_citation_order_each_entry_in_its_language() {
        let refs = references(&doc(), BibStyle::Ieee, Language::Arabic, false);
        assert_eq!(refs.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(), ["smith2021", "ahmadi2020", "lora2019"]);
        assert_eq!(
            refs[0].text,
            "J. Smith, M.-A. Lee, and T. Brown, \u{201C}Deep Learning for IoT,\u{201D} *IEEE Access*, vol. 9, no. 2, pp. 10–20, 2021. https://doi.org/10.1109/x.2021"
        );
        assert_eq!(refs[1].lang, Language::Arabic);
        assert_eq!(refs[1].text, "عبدالله الأحمدي وأحمد علي، *مقدمة في الحوسبة*، جدة: دار المعرفة، 2020.");
        assert_eq!(refs[2].text, "LoRa Alliance, \u{201C}LoRaWAN Specification,\u{201D} In *Proc. IoT & Systems*, 2019, p. 5.");
        assert!(refs[2].latex.contains("Proc. IoT \\& Systems"));
    }

    #[test]
    fn author_year_sorted_with_labels() {
        let refs = references(&doc(), BibStyle::AuthorYear, Language::Arabic, false);
        // Arabic entries first, then by family name.
        assert_eq!(refs.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(), ["ahmadi2020", "lora2019", "smith2021"]);
        assert_eq!(refs[0].label.as_deref(), Some("الأحمدي وعلي(2020)"));
        assert_eq!(refs[2].label.as_deref(), Some("Smith et al.(2021)"));
        assert!(refs[2].text.starts_with("Smith, J., Lee, M.-A., & Brown, T. (2021). Deep Learning for IoT. *IEEE Access*, *9*(2), 10–20."), "{}", refs[2].text);
    }

    #[test]
    fn written_only_for_documents_with_arabic() {
        let english = Languages { main: Language::English, others: vec![], digits: Default::default() };
        let mut d = doc();
        // English document citing an Arabic entry: Texloom writes the list.
        let mixed = Languages { main: Language::English, others: vec![Language::Arabic], digits: Default::default() };
        assert!(needed(&d, &mixed));
        assert!(!needed(&d, &english), "English only: BibTeX as before");
        d.bibliography = BIB.replace("@book{ahmadi2020", "@book{unused");
        assert!(!needed(&d, &mixed), "no Arabic entry cited");
        let t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
        let arabic = Languages { main: Language::Arabic, others: vec![Language::English], digits: Default::default() };
        let p = preamble(&t, &doc(), &arabic);
        assert!(p.contains("\\bibitem{smith2021} \\begin{english}\\tl@ltrentry J. Smith"), "{p}");
        assert!(p.contains("\\bibitem{ahmadi2020} عبدالله الأحمدي"));
        assert!(p.contains("\\renewcommand\\bibliography[1]{\\tl@reflist}"));
    }

    fn indic() -> Languages {
        Languages { main: Language::Arabic, others: vec![Language::English], digits: texloom_core::Digits::ArabicIndic }
    }

    #[test]
    fn author_year_labels_in_the_document_digits_with_whole_english_authors() {
        let t: Template = serde_json::from_str(include_str!("../../../templates/arabic-article/template.json")).unwrap();
        let p = preamble(&t, &doc(), &indic());
        // The year in Arabic-Indic digits; "et al." with its period inside the English run.
        assert!(p.contains("\\bibitem[\\textenglish{Smith et al.}(٢٠٢١)]{smith2021}"), "{p}");
        assert!(p.contains("\\bibitem[الأحمدي وعلي(٢٠٢٠)]{ahmadi2020}"), "{p}");
        // Western digits: years as written.
        let western = Languages { digits: texloom_core::Digits::Western, ..indic() };
        assert!(preamble(&t, &doc(), &western).contains("\\bibitem[\\textenglish{Smith et al.}(2021)]{smith2021}"));
    }

    #[test]
    fn english_entries_in_a_right_to_left_list_are_left_to_right_paragraphs() {
        let t: Template = serde_json::from_str(include_str!("../../../templates/arabic-article/template.json")).unwrap();
        let p = preamble(&t, &doc(), &indic());
        assert!(p.contains("{smith2021} \\begin{english}\\tl@ltrentry Smith, J., Lee, M.-A., \\& Brown, T. (2021)."), "{p}");
        assert!(p.contains("\\url{https://doi.org/10.1109/x.2021}\\par\\end{english}\n"), "the paragraph ends inside the environment: {p}");
        assert!(p.contains("\\def\\tl@ltrentry{\\ifdefined\\XeTeXrevision\\leftskip\\dimexpr\\leftmargin-\\rightmargin"));
        // Arabic entries stay as they are.
        assert!(p.contains("{ahmadi2020} عبدالله الأحمدي"), "{p}");
        // An English document's list is left to right: Arabic entries are runs in it.
        let mixed = Languages { main: Language::English, others: vec![Language::Arabic], digits: Default::default() };
        let p = preamble(&t, &doc(), &mixed);
        assert!(p.contains("{ahmadi2020} \\textarabic{"), "{p}");
        assert!(!p.contains("tl@ltrentry"), "{p}");
    }

    #[test]
    fn numbered_citations_in_the_document_digits() {
        let t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
        let p = preamble(&t, &doc(), &indic());
        // Each entry's number as its label, so \cite writes it in Arabic-Indic digits.
        assert!(p.contains("\\bibitem[١]{smith2021} \\begin{english}\\tl@ltrentry J. Smith"), "{p}");
        assert!(p.contains("\\bibitem[٢]{ahmadi2020} عبدالله"), "{p}");
        assert!(p.contains("\\bibitem[٣]{lora2019}"), "{p}");
        // Western digits: LaTeX numbers them itself.
        let western = Languages { digits: texloom_core::Digits::Western, ..indic() };
        assert!(preamble(&t, &doc(), &western).contains("\\bibitem{smith2021} "));
    }

    #[test]
    fn split_lists_number_on_in_the_document_digits() {
        let mut t: Template = serde_json::from_str(include_str!("../../../templates/senior-design/template.json")).unwrap();
        t.bibliography.split_by_language = true;
        let p = preamble(&t, &doc(), &indic());
        // Arabic entries first, then the others; numbers run on.
        assert!(p.contains("\\bibitem[١]{ahmadi2020}"), "{p}");
        assert!(p.contains("\\bibitem[٢]{smith2021}"), "{p}");
        assert!(p.contains("\\bibitem[٣]{lora2019}"), "{p}");
    }
}
