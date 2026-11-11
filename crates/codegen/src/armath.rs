//! Arabic mathematics: formulas written right to left, as in the schoolbooks and
//! universities of the Gulf, the Levant and Egypt. Arabic letters are the variables,
//! the integral, sum and root are mirrored, powers and indices sit on the left, and
//! limits and functions have Arabic names (`نهـا`, `جا`, `جتا`…).
//!
//! XeTeX has no right-to-left math. The formula is laid out left to right and
//! mirrored as a whole (`\reflectbox`); every letter, word and number is mirrored
//! back first, so it reads normally. What Arabic notation mirrors (∫ ∑ √, brackets,
//! `<`, `→`) is simply left alone. The same works with LuaLaTeX.
//!
//! Variables are set in a font of their own (Aref Ruqaa unless the template says
//! otherwise), so they stand out from the text; numbers, words and function names
//! are in the text font.

use nawl_core::Digits;

use crate::language::Languages;

/// Latin function commands and their Arabic names; `true` when limits go below in
/// display formulas (`\lim`).
const FUNCTIONS: [(&str, &str, bool); 12] = [
    ("sin", "جا", false),
    ("cos", "جتا", false),
    ("tan", "ظا", false),
    ("cot", "ظتا", false),
    ("sec", "قا", false),
    ("csc", "قتا", false),
    ("sinh", "جاز", false),
    ("cosh", "جتاز", false),
    ("tanh", "ظاز", false),
    ("log", "لو", false),
    ("ln", "لط", false),
    ("lim", "نهـــا", true),
];

/// Function names typed in Arabic (without their kashida), longest first, with what
/// is printed and whether limits go below.
const ARABIC_NAMES: [(&str, &str, bool); 13] = [
    ("جتاز", "جتاز", false),
    ("جتا", "جتا", false),
    ("ظتا", "ظتا", false),
    ("قتا", "قتا", false),
    ("جاز", "جاز", false),
    ("ظاز", "ظاز", false),
    ("نها", "نهـــا", true),
    ("جا", "جا", false),
    ("ظا", "ظا", false),
    ("قا", "قا", false),
    ("لو", "لو", false),
    ("لط", "لط", false),
    ("مجـ", "مجـ", true),
];

/// Latin operator names without an Arabic form here: kept, turned back.
const OPERATORS: [(&str, bool); 20] = [
    ("max", true),
    ("min", true),
    ("sup", true),
    ("inf", true),
    ("det", true),
    ("gcd", true),
    ("Pr", true),
    ("limsup", true),
    ("liminf", true),
    ("exp", false),
    ("arg", false),
    ("deg", false),
    ("dim", false),
    ("ker", false),
    ("hom", false),
    ("arcsin", false),
    ("arccos", false),
    ("arctan", false),
    ("coth", false),
    ("lg", false),
];

/// Symbols that aren't mirrored in Arabic notation: Greek letters and the like.
const TURNED_BACK: [&str; 52] = [
    "alpha", "beta", "gamma", "delta", "epsilon", "varepsilon", "zeta", "eta", "theta", "vartheta", "iota", "kappa", "lambda", "mu", "nu", "xi", "pi", "varpi", "rho",
    "varrho", "sigma", "varsigma", "tau", "upsilon", "phi", "varphi", "chi", "psi", "omega", "Gamma", "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma", "Upsilon", "Phi",
    "Psi", "Omega", "ell", "hbar", "imath", "jmath", "partial", "Re", "Im", "aleph", "wp", "emptyset", "varnothing", "checkmark",
];

/// Commands whose argument is letters in another alphabet (`\mathbb{R}`): turned
/// back as a whole.
const ALPHABETS: [&str; 11] = ["mathrm", "mathbf", "mathit", "mathsf", "mathtt", "mathbb", "mathcal", "mathfrak", "mathscr", "boldsymbol", "mathnormal"];

/// Text inside a formula: turned back as a whole, in the text font.
const TEXT: [&str; 8] = ["text", "textrm", "textbf", "textit", "textsf", "texttt", "textnormal", "mbox"];

/// Commands whose argument is a length, a name or a colour: copied as it is.
const RAW: [&str; 16] = ["hspace", "vspace", "kern", "mkern", "hskip", "mskip", "rule", "raisebox", "label", "ref", "eqref", "begin", "end", "color", "mathcolor", "tag"];

/// Environments whose second argument is a column specification.
const WITH_COLUMNS: [&str; 4] = ["array", "subarray", "alignedat", "tabular"];

/// The commands the mirrored formulas use; `font` is the family command of the
/// variables' font (`\tlfontarefruqaa`). Goes in the preamble, after the class.
pub fn preamble(font: &str) -> String {
    let mut o = String::from("% Arabic mathematics: formulas are mirrored as a whole; letters, words and numbers are mirrored back\n\\makeatletter\n");
    o.push_str(&format!("\\providecommand\\{font}{{}}\\let\\tlmathfont\\{font}\n"));
    // A variable, in the variables' font; a word or number, in the text font.
    o.push_str("\\DeclareRobustCommand\\tla[1]{\\text{\\reflectbox{\\textarabic{\\tlmathfont #1}}}}\n");
    o.push_str("\\DeclareRobustCommand\\tlw[1]{\\text{\\reflectbox{\\textarabic{#1}}}}\n");
    // A symbol that isn't mirrored (a Greek letter), at the size of its place.
    o.push_str("\\DeclareRobustCommand\\tlm[1]{\\mathpalette\\tl@m{#1}}\n\\def\\tl@m#1#2{\\reflectbox{$\\m@th#1#2$}}\n");
    o.push_str("\\DeclareRobustCommand\\tlarmath[1]{\\reflectbox{$#1$}}\n");
    o.push_str("\\DeclareRobustCommand\\tlardisplay[1]{\\reflectbox{$\\displaystyle #1$}}\n");
    o.push_str("\\makeatother\n");
    o
}

/// A formula in a line of text.
pub fn inline(latex: &str, langs: &Languages) -> String {
    format!("\\tlarmath{{{}}}", convert(latex, langs))
}

/// A displayed formula: the content of an `equation` environment.
pub fn display(latex: &str, langs: &Languages) -> String {
    format!("\\tlardisplay{{{}}}", convert(latex, langs))
}

/// White space around a formula's picture, so nothing is cut at its edges (pt).
pub const PICTURE_PADDING_PT: f32 = 2.0;

/// The body of a document that sets each formula on a page of its own, exactly its
/// size (plus the padding), for pictures of formulas (Word can't lay math out right
/// to left). Each page writes its width, height and depth to the log, see
/// [`picture_sizes`]. `formulas`: the LaTeX and whether it's a displayed equation.
pub fn picture_pages(formulas: &[(String, bool)], langs: &Languages) -> String {
    let pad = PICTURE_PADDING_PT;
    let mut o = String::from("\\begin{document}\n\\makeatletter\n");
    o.push_str(&format!(
        "\\newcommand\\tlformula[1]{{\\setbox0\\hbox{{\\kern{pad}pt #1\\kern{pad}pt}}\\typeout{{TLF:\\the\\wd0:\\the\\ht0:\\the\\dp0}}\\pdfpagewidth=\\wd0 \\pdfpageheight=\\dimexpr\\ht0+\\dp0+{}pt\\relax\\hoffset=-1in \\voffset=-1in \\shipout\\vbox{{\\kern{pad}pt\\box0\\kern{pad}pt}}}}\n",
        2.0 * pad
    ));
    o.push_str("\\makeatother\n");
    for (latex, is_display) in formulas {
        let clean = crate::tex::clean_math(latex);
        let formula = if *is_display { display(&clean, langs) } else { inline(&clean, langs) };
        o.push_str(&format!("\\tlformula{{{formula}}}\n"));
    }
    o.push_str("\\end{document}\n");
    o
}

/// Each formula picture's width, height above the baseline and depth below it (pt,
/// padding included), from the log of a [`picture_pages`] compile.
pub fn picture_sizes(log: &str) -> Vec<[f32; 3]> {
    let pad = PICTURE_PADDING_PT;
    log.lines()
        .filter_map(|l| l.trim().strip_prefix("TLF:"))
        .filter_map(|l| {
            let mut dims = l.split(':').map(|d| d.trim().trim_end_matches("pt").parse::<f32>().ok());
            match (dims.next()??, dims.next()??, dims.next()??) {
                (w, h, d) => Some([w, h + pad, d + pad]),
            }
        })
        .collect()
}

/// The formula with every letter, word and number wrapped so it's mirrored back.
pub fn convert(latex: &str, langs: &Languages) -> String {
    let chars: Vec<char> = latex.chars().collect();
    let mut w = Writer { chars: &chars, i: 0, out: String::with_capacity(latex.len() * 3), indic: langs.digits == Digits::ArabicIndic, after_script: false };
    w.run(false);
    w.out
}

fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0621}'..='\u{064A}' | '\u{066E}'..='\u{06D3}')
}

fn is_arabic_mark(c: char) -> bool {
    matches!(c, '\u{064B}'..='\u{065F}' | '\u{0670}')
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '\u{0660}'..='\u{0669}' | '\u{06F0}'..='\u{06F9}')
}

struct Writer<'a> {
    chars: &'a [char],
    i: usize,
    out: String,
    /// Numbers in Arabic-Indic digits.
    indic: bool,
    /// Right after `^` or `_`: only one character is the script (`x^12` is x¹ 2).
    after_script: bool,
}

impl Writer<'_> {
    /// Writes until the end, or until the `}` that closes the group being written.
    fn run(&mut self, in_group: bool) {
        while self.i < self.chars.len() {
            let c = self.chars[self.i];
            let script = std::mem::take(&mut self.after_script);
            match c {
                '}' if in_group => return,
                '{' => {
                    self.i += 1;
                    self.out.push('{');
                    self.run(true);
                    self.out.push('}');
                    self.i += 1;
                }
                '\\' => self.command(),
                '^' | '_' => {
                    self.out.push(c);
                    self.i += 1;
                    self.after_script = true;
                }
                c if is_digit(c) => self.number(script),
                c if is_arabic_letter(c) => self.arabic(script),
                c if c.is_ascii_alphabetic() => {
                    self.out.push_str(&format!("{{\\tlm{{{c}}}}}"));
                    self.i += 1;
                }
                ',' | '،' => self.punctuation("،"),
                ';' | '؛' => self.punctuation("؛"),
                '?' | '؟' => self.punctuation("؟"),
                '/' => {
                    self.out.push_str("{\\tlm{/}}");
                    self.i += 1;
                }
                c if c.is_whitespace() => {
                    // Spaces don't end a script's turn: `x^ 2`.
                    self.after_script = script;
                    self.out.push(c);
                    self.i += 1;
                }
                _ => {
                    self.out.push(c);
                    self.i += 1;
                }
            }
        }
    }

    fn punctuation(&mut self, mark: &str) {
        self.out.push_str(&format!("\\mathpunct{{\\tlw{{{mark}}}}}"));
        self.i += 1;
    }

    /// A number: its digits in the document's digits, the decimal point with them.
    fn number(&mut self, one_digit: bool) {
        let mut text = String::new();
        while self.i < self.chars.len() {
            let c = self.chars[self.i];
            let next_is_digit = self.chars.get(self.i + 1).is_some_and(|n| is_digit(*n));
            if is_digit(c) {
                text.push(match c.to_digit(10).filter(|_| self.indic && c.is_ascii_digit()) {
                    Some(n) => char::from_u32(0x0660 + n).unwrap_or(c),
                    None => c,
                });
            } else if (c == '.' || c == '٫') && next_is_digit && !one_digit {
                text.push(if self.indic { '٫' } else { c });
            } else {
                break;
            }
            self.i += 1;
            if one_digit {
                break;
            }
        }
        self.out.push_str(&format!("{{\\tlw{{{text}}}}}"));
    }

    /// Arabic letters: a function name, a word (with a kashida), or variables, each
    /// letter on its own so it keeps its isolated shape.
    fn arabic(&mut self, one_letter: bool) {
        let start = self.i;
        let mut end = start;
        while end < self.chars.len() && (is_arabic_letter(self.chars[end]) || is_arabic_mark(self.chars[end])) {
            end += 1;
        }
        let run = &self.chars[start..end];
        let mut k = 0;
        while k < run.len() {
            let rest = &run[k..];
            if let Some((len, shown, limits)) = arabic_name(rest) {
                self.operator(&format!("\\tlw{{{shown}}}"), limits);
                k += len;
            } else {
                let mut letter = String::from(rest[0]);
                let mut n = 1;
                while n < rest.len() && is_arabic_mark(rest[n]) {
                    letter.push(rest[n]);
                    n += 1;
                }
                if rest[0] == 'ـ' || rest.get(n) == Some(&'ـ') {
                    // A kashida after the letter: the rest of the run is one word.
                    let word: String = rest.iter().collect();
                    self.out.push_str(&format!("{{\\tlw{{{word}}}}}"));
                    k = run.len();
                } else {
                    self.out.push_str(&format!("{{\\tla{{{letter}}}}}"));
                    k += n;
                }
            }
            if one_letter {
                break;
            }
        }
        self.i = start + k;
    }

    /// A named operator: the function it's applied to follows after a thin space.
    fn operator(&mut self, name: &str, limits: bool) {
        self.out.push_str(&format!("\\mathop{{{name}}}"));
        if !limits {
            self.out.push_str("\\nolimits");
        }
    }

    fn command(&mut self) {
        let start = self.i;
        self.i += 1;
        while self.i < self.chars.len() && self.chars[self.i].is_ascii_alphabetic() {
            self.i += 1;
        }
        if self.i == start + 1 {
            // A control symbol: \{ \, \\ \%
            let symbol = self.chars.get(self.i).copied();
            self.i += usize::from(symbol.is_some());
            match symbol {
                Some(s @ ('%' | '$' | '#' | '&')) => self.out.push_str(&format!("{{\\tlm{{\\{s}}}}}")),
                Some(s) => {
                    self.out.push('\\');
                    self.out.push(s);
                }
                None => self.out.push('\\'),
            }
            return;
        }
        let name: String = self.chars[start + 1..self.i].iter().collect();
        let name = name.as_str();
        if let Some((_, arabic, limits)) = FUNCTIONS.iter().find(|(latin, ..)| *latin == name) {
            self.operator(&format!("\\tlw{{{arabic}}}"), *limits);
        } else if let Some((_, limits)) = OPERATORS.iter().find(|(latin, _)| *latin == name) {
            self.operator(&format!("\\tlm{{\\{name}}}"), *limits);
        } else if TURNED_BACK.contains(&name) {
            self.out.push_str(&format!("{{\\tlm{{\\{name}}}}}"));
        } else if TEXT.contains(&name) {
            match self.group() {
                Some(text) if name == "text" || name == "mbox" => self.out.push_str(&format!("{{\\tlw{{{text}}}}}")),
                Some(text) => self.out.push_str(&format!("{{\\tlw{{\\{name}{{{text}}}}}}}")),
                None => {}
            }
        } else if ALPHABETS.contains(&name) || name == "operatorname" {
            match self.group() {
                // Arabic inside: a function name (`\operatorname{جا}`), or plain letters.
                Some(inner) if inner.chars().any(is_arabic_letter) => {
                    if name == "operatorname" {
                        let limits = self.chars.get(self.i) == Some(&'*');
                        self.i += usize::from(limits);
                        self.operator(&format!("\\tlw{{{inner}}}"), limits);
                    } else {
                        let chars: Vec<char> = inner.chars().collect();
                        let mut w = Writer { chars: &chars, i: 0, out: String::new(), indic: self.indic, after_script: false };
                        w.run(false);
                        self.out.push_str(&format!("{{{}}}", w.out));
                    }
                }
                Some(inner) if name == "operatorname" => {
                    let limits = self.chars.get(self.i) == Some(&'*');
                    self.i += usize::from(limits);
                    self.operator(&format!("\\tlm{{\\mathrm{{{inner}}}}}"), limits);
                }
                Some(inner) => self.out.push_str(&format!("{{\\tlm{{\\{name}{{{inner}}}}}}}")),
                None => {}
            }
        } else if RAW.contains(&name) {
            self.out.push('\\');
            self.out.push_str(name);
            let Some(arg) = self.group() else { return };
            self.out.push_str(&format!("{{{arg}}}"));
            if name == "begin" && WITH_COLUMNS.contains(&arg.as_str()) {
                if let Some(columns) = self.group() {
                    self.out.push_str(&format!("{{{columns}}}"));
                }
            }
        } else {
            self.out.push('\\');
            self.out.push_str(name);
            // A letter after it would join the name: `\cdot x`.
            if self.chars.get(self.i).is_some_and(|c| c.is_ascii_alphabetic()) {
                self.out.push(' ');
            }
        }
    }

    /// The `{…}` that follows, as it is (without its braces); `None` when there
    /// isn't one.
    fn group(&mut self) -> Option<String> {
        let mut j = self.i;
        while self.chars.get(j).is_some_and(|c| c.is_whitespace()) {
            j += 1;
        }
        if self.chars.get(j) != Some(&'{') {
            return None;
        }
        let open = j + 1;
        let mut depth = 1;
        j = open;
        while j < self.chars.len() {
            match self.chars[j] {
                '\\' => j += 1,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let end = j.min(self.chars.len());
        self.i = (end + 1).min(self.chars.len());
        Some(self.chars[open..end].iter().collect())
    }
}

/// A function name at the start of `run` (kashidas inside it don't count): how many
/// characters it takes, what is printed, and whether limits go below.
fn arabic_name(run: &[char]) -> Option<(usize, &'static str, bool)> {
    for (name, shown, limits) in ARABIC_NAMES {
        let wanted: Vec<char> = name.chars().collect();
        let mut taken = 0;
        let mut matched = 0;
        while matched < wanted.len() && taken < run.len() {
            let c = run[taken];
            if c == wanted[matched] {
                matched += 1;
            } else if c == 'ـ' && matched > 0 {
                // A kashida stretching the name: نهـــا.
            } else {
                break;
            }
            taken += 1;
        }
        if matched == wanted.len() {
            // A kashida after the last letter belongs to it (مجـ).
            while run.get(taken) == Some(&'ـ') {
                taken += 1;
            }
            return Some((taken, shown, limits));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use nawl_core::Language;

    fn langs(digits: Digits) -> Languages {
        Languages { main: Language::Arabic, others: vec![Language::English], digits }
    }

    fn indic(latex: &str) -> String {
        convert(latex, &langs(Digits::ArabicIndic))
    }

    #[test]
    fn variables_numbers_and_scripts() {
        // Each letter on its own (isolated shapes), in the variables' font.
        assert_eq!(indic("س^2+ص^2=ع^2"), "{\\tla{س}}^{\\tlw{٢}}+{\\tla{ص}}^{\\tlw{٢}}={\\tla{ع}}^{\\tlw{٢}}");
        assert_eq!(indic("دس"), "{\\tla{د}}{\\tla{س}}");
        // Numbers are one piece, in the document's digits; a script takes one digit.
        assert_eq!(indic("3.5 + ١٢"), "{\\tlw{٣٫٥}} + {\\tlw{١٢}}");
        assert_eq!(indic("س^12"), "{\\tla{س}}^{\\tlw{١}}{\\tlw{٢}}");
        assert_eq!(indic("س_{12}"), "{\\tla{س}}_{{\\tlw{١٢}}}");
        assert_eq!(convert("3.5", &langs(Digits::Western)), "{\\tlw{3.5}}");
        // Latin and Greek letters stay the right way round.
        assert_eq!(indic("x + \\pi r"), "{\\tlm{x}} + {\\tlm{\\pi}} {\\tlm{r}}");
        assert_eq!(indic("\\mathbb{R}"), "{\\tlm{\\mathbb{R}}}");
        // A letter with its vowel mark stays together.
        assert_eq!(indic("سَ"), "{\\tla{سَ}}");
    }

    #[test]
    fn functions_and_limits() {
        assert_eq!(indic("\\sin س"), "\\mathop{\\tlw{جا}}\\nolimits {\\tla{س}}");
        assert_eq!(indic("\\lim_{س\\to\\infty}"), "\\mathop{\\tlw{نهـــا}}_{{\\tla{س}}\\to\\infty}");
        // Typed in Arabic, with or without a kashida, also right before the variable.
        assert_eq!(indic("جتاس"), "\\mathop{\\tlw{جتا}}\\nolimits{\\tla{س}}");
        assert_eq!(indic("نهـا_{س\\to 0}"), "\\mathop{\\tlw{نهـــا}}_{{\\tla{س}}\\to {\\tlw{٠}}}");
        assert_eq!(indic("مجـ_{ك=1}^{ن}"), "\\mathop{\\tlw{مجـ}}_{{\\tla{ك}}={\\tlw{١}}}^{{\\tla{ن}}}");
        assert_eq!(indic("\\operatorname{جا}س"), "\\mathop{\\tlw{جا}}\\nolimits{\\tla{س}}");
        assert_eq!(indic("\\max"), "\\mathop{\\tlm{\\max}}");
        // Any other word with a kashida is one word.
        assert_eq!(indic("حـد"), "{\\tlw{حـد}}");
    }

    #[test]
    fn structure_is_kept() {
        assert_eq!(indic("\\frac{1}{س}"), "\\frac{{\\tlw{١}}}{{\\tla{س}}}");
        assert_eq!(indic("\\sqrt[3]{س}"), "\\sqrt[{\\tlw{٣}}]{{\\tla{س}}}");
        assert_eq!(indic("\\int_0^1"), "\\int_{\\tlw{٠}}^{\\tlw{١}}");
        assert_eq!(indic("\\cdot x"), "\\cdot {\\tlm{x}}");
        // Text is one piece, in the text font; lengths and column specifications are copied.
        assert_eq!(indic("\\text{إذا كان } س>0"), "{\\tlw{إذا كان }} {\\tla{س}}>{\\tlw{٠}}");
        assert_eq!(indic("\\hspace{2pt}"), "\\hspace{2pt}");
        assert_eq!(indic("\\begin{array}{cc}1&2\\\\3&4\\end{array}"), "\\begin{array}{cc}{\\tlw{١}}&{\\tlw{٢}}\\\\{\\tlw{٣}}&{\\tlw{٤}}\\end{array}");
        assert_eq!(indic("(س,ص)"), "({\\tla{س}}\\mathpunct{\\tlw{،}}{\\tla{ص}})");
        assert_eq!(indic("50\\%"), "{\\tlw{٥٠}}{\\tlm{\\%}}");
        assert_eq!(inline("س", &langs(Digits::ArabicIndic)), "\\tlarmath{{\\tla{س}}}");
        assert_eq!(display("س", &langs(Digits::ArabicIndic)), "\\tlardisplay{{\\tla{س}}}");
    }
}
