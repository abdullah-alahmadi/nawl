//! MathML (from the editor's MathLive) → Office Math (OMML), so equations stay
//! editable in Word. Covers what the editor produces: identifiers, numbers,
//! operators, fractions, scripts, roots, big operators, accents, matrices.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::xml::esc;

thread_local! {
    /// Digits in formulas written Arabic-Indic (an Arabic document using ١٢٣), for
    /// the conversion running on this thread.
    static INDIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
    text: String,
}

impl Node {
    fn attr(&self, k: &str) -> Option<&str> {
        self.attrs.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }
    /// The element's own text plus its children's (for token elements).
    fn all_text(&self) -> String {
        let mut s = self.text.clone();
        for c in &self.children {
            s.push_str(&c.all_text());
        }
        s
    }
}

fn parse(mathml: &str) -> Option<Node> {
    let mut reader = Reader::from_str(mathml);
    let mut stack: Vec<Node> = vec![Node { name: "math".into(), ..Default::default() }];
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) => {
                let mut n = Node { name: local(e.name().as_ref().as_bytes()), ..Default::default() };
                for a in e.attributes().flatten() {
                    n.attrs.push((local(a.key.as_ref().as_bytes()), a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok()?.into_owned()));
                }
                stack.push(n);
            }
            Event::Empty(e) => {
                let mut n = Node { name: local(e.name().as_ref().as_bytes()), ..Default::default() };
                for a in e.attributes().flatten() {
                    n.attrs.push((local(a.key.as_ref().as_bytes()), a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok()?.into_owned()));
                }
                stack.last_mut()?.children.push(n);
            }
            Event::Text(t) => {
                let s = t.xml10_content().into_owned();
                stack.last_mut()?.text.push_str(&s);
            }
            Event::GeneralRef(r) => {
                // Character references (&#x2212;) inside token elements.
                let name: String = r.as_ref().to_string();
                let c = if let Some(hex) = name.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                } else if let Some(dec) = name.strip_prefix('#') {
                    dec.parse().ok().and_then(char::from_u32)
                } else {
                    match name.as_str() {
                        "lt" => Some('<'),
                        "gt" => Some('>'),
                        "amp" => Some('&'),
                        "quot" => Some('"'),
                        "apos" => Some('\''),
                        _ => None,
                    }
                };
                if let Some(c) = c {
                    stack.last_mut()?.text.push(c);
                }
            }
            Event::End(_) => {
                let n = stack.pop()?;
                stack.last_mut()?.children.push(n);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    stack.pop()
}

fn local(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name);
    s.rsplit(':').next().unwrap_or(&s).to_string()
}

/// OMML for a MathML formula (`<m:oMath>…</m:oMath>`), or `None` if it doesn't parse.
pub fn omml(mathml: &str) -> Option<String> {
    omml_with(mathml, false)
}

/// The same, with numbers in Arabic-Indic digits (`3.5` → `٣٫٥`) when `indic`.
pub fn omml_with(mathml: &str, indic: bool) -> Option<String> {
    INDIC.with(|c| c.set(indic));
    let root = parse(mathml)?;
    let mut out = String::from("<m:oMath>");
    out.push_str(&children(&root));
    out.push_str("</m:oMath>");
    Some(out)
}

/// A formula Word can show when there's no MathML (LaTeX source, upright).
pub fn fallback(latex: &str) -> String {
    format!("<m:oMath>{}</m:oMath>", run(latex, true))
}

fn children(n: &Node) -> String {
    let mut out = String::new();
    let mut rest = n.children.iter().peekable();
    while let Some(c) = rest.next() {
        let s = node(c);
        match s.strip_suffix("<m:e/></m:nary>") {
            // ∑ and ∫ take what follows them as their body, up to a relation or a
            // + or − (as Word's own equation editor does).
            Some(head) => {
                let mut body = String::new();
                while let Some(next) = rest.next_if(|x| !(x.name == "mo" && "=<>≤≥≈≠≡+−-,".contains(x.all_text().trim()) && !x.all_text().trim().is_empty())) {
                    body.push_str(&node(next));
                }
                out.push_str(&format!("{head}<m:e>{body}</m:e></m:nary>"));
            }
            None => out.push_str(&s),
        }
    }
    out
}

/// A math run: italic letters unless `plain`. Arabic is set as normal right-to-left
/// text in the document's Arabic font (math fonts have no Arabic letters).
fn run(text: &str, plain: bool) -> String {
    if text.is_empty() {
        return String::new();
    }
    if text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)) {
        return format!("<m:r><m:rPr><m:nor/></m:rPr><w:rPr><w:rtl/></w:rPr><m:t xml:space=\"preserve\">{}</m:t></m:r>", esc(text));
    }
    let sty = if plain { "<m:rPr><m:sty m:val=\"p\"/></m:rPr>" } else { "" };
    format!(
        "<m:r>{sty}<w:rPr><w:rFonts w:ascii=\"Cambria Math\" w:hAnsi=\"Cambria Math\"/></w:rPr><m:t xml:space=\"preserve\">{}</m:t></m:r>",
        esc(text)
    )
}

/// Argument element (`<m:e>`, `<m:num>`…) holding a node.
fn arg(tag: &str, n: Option<&Node>) -> String {
    format!("<m:{tag}>{}</m:{tag}>", n.map(node).unwrap_or_default())
}

const BIG_OPERATORS: &str = "∑∏∐∫∬∭∮⋃⋂⋁⋀⨁⨂";

fn is_big_operator(n: Option<&Node>) -> bool {
    n.is_some_and(|b| {
        let t = b.all_text();
        b.name == "mo" && t.chars().count() == 1 && BIG_OPERATORS.contains(t.as_str())
    })
}

/// An n-ary operator (∑, ∫) with its limits; the body follows in the row.
fn nary(chr: &str, sub: Option<&Node>, sup: Option<&Node>, place: &str) -> String {
    let hide = |h: bool, tag: &str| if h { format!("<m:{tag} m:val=\"1\"/>") } else { String::new() };
    format!(
        "<m:nary><m:naryPr><m:chr m:val=\"{}\"/><m:limLoc m:val=\"{place}\"/>{}{}</m:naryPr>{}{}<m:e/></m:nary>",
        esc(chr),
        hide(sub.is_none(), "subHide"),
        hide(sup.is_none(), "supHide"),
        arg("sub", sub),
        arg("sup", sup)
    )
}

/// Word draws accents from combining characters; MathML often has the spacing ones.
fn combining(chr: &str) -> String {
    match chr {
        "^" | "ˆ" => "\u{0302}",
        "¯" | "‾" | "_" => "\u{0305}",
        "~" | "˜" => "\u{0303}",
        "˙" => "\u{0307}",
        "¨" => "\u{0308}",
        "ˇ" => "\u{030C}",
        "→" => "\u{20D7}",
        other => other,
    }
    .to_string()
}

/// The brackets of a row that opens and closes with matching fences.
fn fenced(n: &Node) -> Option<(String, String)> {
    let c = &n.children;
    if c.len() < 2 {
        return None;
    }
    let (first, last) = (&c[0], &c[c.len() - 1]);
    let (open, close) = (first.all_text(), last.all_text());
    let opens = ["(", "[", "{", "|", "‖", "⟨"].contains(&open.as_str());
    let closes = [")", "]", "}", "|", "‖", "⟩"].contains(&close.as_str());
    (first.name == "mo" && last.name == "mo" && opens && closes).then_some((open, close))
}

fn node(n: &Node) -> String {
    let c = &n.children;
    match n.name.as_str() {
        "mi" => {
            let t = n.all_text();
            // Single letters are italic (variables); names like "sin" are upright.
            let plain = t.chars().count() > 1 || n.attr("mathvariant") == Some("normal");
            run(&t, plain)
        }
        // MathLive marks implied products and function application with invisible
        // operators; Word needs none.
        "mo" if n.all_text().chars().all(|c| ('\u{2061}'..='\u{2064}').contains(&c)) => String::new(),
        "msub" | "msup" | "msubsup" if is_big_operator(c.first()) => {
            let (sub, sup) = match n.name.as_str() {
                "msub" => (c.get(1), None),
                "msup" => (None, c.get(1)),
                _ => (c.get(1), c.get(2)),
            };
            nary(&c[0].all_text(), sub, sup, "subSup")
        }
        "mn" if INDIC.with(|c| c.get()) => {
            let digits: String = n
                .all_text()
                .chars()
                .map(|c| match c {
                    '0'..='9' => char::from_u32(0x0660 + (c as u32 - '0' as u32)).unwrap_or(c),
                    '.' => '\u{066B}',
                    c => c,
                })
                .collect();
            run(&digits, true)
        }
        "mn" | "mo" | "mtext" | "ms" => run(&n.all_text(), true),
        "mspace" => run(" ", true),
        "mfrac" => format!("<m:f>{}{}</m:f>", arg("num", c.first()), arg("den", c.get(1))),
        "msup" => format!("<m:sSup>{}{}</m:sSup>", arg("e", c.first()), arg("sup", c.get(1))),
        "msub" => format!("<m:sSub>{}{}</m:sSub>", arg("e", c.first()), arg("sub", c.get(1))),
        "msubsup" => format!("<m:sSubSup>{}{}{}</m:sSubSup>", arg("e", c.first()), arg("sub", c.get(1)), arg("sup", c.get(2))),
        "msqrt" => format!("<m:rad><m:radPr><m:degHide m:val=\"1\"/></m:radPr><m:deg/><m:e>{}</m:e></m:rad>", children(n)),
        "mroot" => format!("<m:rad>{}{}</m:rad>", arg("deg", c.get(1)), arg("e", c.first())),
        "munder" | "mover" | "munderover" => {
            let base = c.first();
            let base_text = base.map(|b| b.all_text()).unwrap_or_default();
            if is_big_operator(base) {
                // ∑ with limits above and below.
                let (sub, sup) = match n.name.as_str() {
                    "munder" => (c.get(1), None),
                    "mover" => (None, c.get(1)),
                    _ => (c.get(1), c.get(2)),
                };
                return nary(&base_text, sub, sup, "undOvr");
            }
            match n.name.as_str() {
                // An accent (x̂, x̄, x⃗) or something over the base.
                "mover" if n.attr("accent") == Some("true") || c.get(1).is_some_and(|a| a.name == "mo") => {
                    let chr = combining(&c.get(1).map(|a| a.all_text()).unwrap_or_default());
                    format!("<m:acc><m:accPr><m:chr m:val=\"{}\"/></m:accPr>{}</m:acc>", esc(&chr), arg("e", base))
                }
                "mover" => format!("<m:limUpp>{}{}</m:limUpp>", arg("e", base), arg("lim", c.get(1))),
                "munder" => format!("<m:limLow>{}{}</m:limLow>", arg("e", base), arg("lim", c.get(1))),
                _ => format!(
                    "<m:limUpp><m:e><m:limLow>{}{}</m:limLow></m:e>{}</m:limUpp>",
                    arg("e", base),
                    arg("lim", c.get(1)),
                    arg("lim", c.get(2))
                ),
            }
        }
        "mtable" => {
            let rows: String = c
                .iter()
                .filter(|r| r.name == "mtr" || r.name == "mlabeledtr")
                .map(|r| format!("<m:mr>{}</m:mr>", r.children.iter().map(|cell| format!("<m:e>{}</m:e>", children(cell))).collect::<String>()))
                .collect();
            format!("<m:m>{rows}</m:m>")
        }
        // A labelled formula's annotation (the LaTeX source) isn't shown.
        "semantics" => c.first().map(node).unwrap_or_default(),
        "annotation" | "annotation-xml" => String::new(),
        "mphantom" => String::new(),
        "mrow" if fenced(n).is_some() => {
            // (…), [ … ], |…|: a Word delimiter, which grows with its contents.
            let (open, close) = fenced(n).unwrap_or_default();
            let inner: String = c[1..c.len() - 1].iter().map(node).collect();
            format!(
                "<m:d><m:dPr><m:begChr m:val=\"{}\"/><m:endChr m:val=\"{}\"/></m:dPr><m:e>{inner}</m:e></m:d>",
                esc(&open),
                esc(&close)
            )
        }
        _ => {
            // mrow, mstyle, mpadded, math…: their children in a row, plus loose text.
            let mut s = run(n.text.trim(), true);
            s.push_str(&children(n));
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_common_formulas() {
        let f = omml("<math><mi>g</mi><mo>=</mo><mfrac><mrow><mi>a</mi><mo>&#x2212;</mo><mn>1</mn></mrow><mn>2</mn></mfrac></math>").unwrap();
        assert!(f.starts_with("<m:oMath>") && f.ends_with("</m:oMath>"));
        assert!(f.contains("<m:f><m:num>"), "{f}");
        assert!(f.contains("<m:t xml:space=\"preserve\">−</m:t>"), "character references decoded: {f}");
        let s = omml("<math><msup><mi>x</mi><mn>2</mn></msup><msqrt><mi>y</mi></msqrt></math>").unwrap();
        assert!(s.contains("<m:sSup><m:e>") && s.contains("<m:rad><m:radPr><m:degHide"), "{s}");
        let sum = omml("<math><munderover><mo>∑</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover><mi>i</mi></math>").unwrap();
        assert!(sum.contains("<m:nary><m:naryPr><m:chr m:val=\"∑\"/>"), "{sum}");
        assert!(omml("<math><mi>sin</mi></math>").unwrap().contains("<m:sty m:val=\"p\"/>"), "function names upright");
        assert!(omml("<math><mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr></mtable></math>").unwrap().contains("<m:m><m:mr><m:e>"));
        // As MathLive writes them: invisible operators, ∑ as msubsup, spacing
        // accents, fences.
        let ml = omml("<math><mrow><mi>f</mi><mo>&#x2061;</mo><mrow><mo>(</mo><mi>x</mi><mo>)</mo></mrow><mi>d</mi><mo>&#8290;</mo><mi>x</mi></mrow></math>").unwrap();
        assert!(!ml.contains('\u{2061}') && !ml.contains('\u{2062}'), "{ml}");
        assert!(ml.contains("<m:d><m:dPr><m:begChr m:val=\"(\"/><m:endChr m:val=\")\"/></m:dPr>"), "{ml}");
        let int = omml("<math><msubsup><mo>∫</mo><mn>0</mn><mn>1</mn></msubsup></math>").unwrap();
        assert!(int.contains("<m:chr m:val=\"∫\"/><m:limLoc m:val=\"subSup\"/>"), "{int}");
        let body = omml("<math><mrow><msubsup><mo>∑</mo><mi>i</mi><mi>n</mi></msubsup><mi>x</mi><mo>=</mo><mn>1</mn></mrow></math>").unwrap();
        assert!(body.contains("<m:e><m:r><m:t xml:space=\"preserve\">x</m:t></m:r></m:e></m:nary>") || body.contains(">x</m:t></m:r></m:e></m:nary><m:r>"), "body up to the relation: {body}");
        let hat = omml("<math><mover accent=\"true\"><mi>x</mi><mo>&#x005e;</mo></mover></math>").unwrap();
        assert!(hat.contains("<m:chr m:val=\"\u{0302}\"/>"), "{hat}");
        // Arabic as right-to-left normal text; Arabic-Indic digits when asked.
        let ar = omml_with("<math><mi>س</mi><mo>+</mo><mn>3.5</mn></math>", true).unwrap();
        assert!(ar.contains("<m:nor/></m:rPr><w:rPr><w:rtl/></w:rPr><m:t xml:space=\"preserve\">س</m:t>"), "{ar}");
        assert!(ar.contains(">٣٫٥</m:t>"), "{ar}");
        assert!(omml("<math><mn>3.5</mn></math>").unwrap().contains(">3.5</m:t>"));
        assert!(omml("<math><mi>x</mi>").is_none() || omml("<math><mi>x</mi>").is_some(), "unbalanced input doesn't panic");
    }
}
