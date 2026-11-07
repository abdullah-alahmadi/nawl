//! Small LaTeX-writing helpers shared by the class and document generators.

use texloom_core::placeholder::{self, Piece};
use texloom_core::{Align, TextStyle};

/// Escapes characters that are special in LaTeX text mode.
pub fn escape(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => s.push_str("\\textbackslash{}"),
            '{' => s.push_str("\\{"),
            '}' => s.push_str("\\}"),
            '$' => s.push_str("\\$"),
            '&' => s.push_str("\\&"),
            '%' => s.push_str("\\%"),
            '#' => s.push_str("\\#"),
            '_' => s.push_str("\\_"),
            '~' => s.push_str("\\textasciitilde{}"),
            '^' => s.push_str("\\textasciicircum{}"),
            // Line breaks inside one-line text would end the paragraph or error.
            '\n' | '\r' => s.push(' '),
            // Typographic punctuation the classic TeX way, which works with every font
            // (publisher kits often use old 8-bit fonts that lack the Unicode forms).
            '\u{2018}' => s.push('`'),
            '\u{2019}' => s.push('\''),
            '\u{201C}' => s.push_str("``"),
            '\u{201D}' => s.push_str("''"),
            '\u{2013}' => s.push_str("--"),
            '\u{2014}' => s.push_str("---"),
            '\u{2026}' => s.push_str("\\ldots{}"),
            '\u{00A0}' => s.push('~'),
            // Symbols that old 8-bit (T1) fonts keep in other slots: write them as
            // commands, which every font setup renders correctly.
            c if symbol_command(c).is_some() => s.push_str(symbol_command(c).expect("checked")),
            _ => s.push(c),
        }
    }
    s
}

/// LaTeX commands for symbols in the Latin-1 range (and a few more) whose slots
/// differ between Unicode and the T1 encoding of publisher kits' fonts.
fn symbol_command(c: char) -> Option<&'static str> {
    Some(match c {
        '¡' => "\\textexclamdown{}",
        '¢' => "\\textcent{}",
        '£' => "\\pounds{}",
        '¥' => "\\textyen{}",
        '§' => "\\S{}",
        '©' => "\\copyright{}",
        'ª' => "\\textordfeminine{}",
        '«' => "\\guillemotleft{}",
        '¬' => "\\textlnot{}",
        '®' => "\\textregistered{}",
        '°' => "\\textdegree{}",
        '±' => "\\ensuremath{\\pm}",
        '²' => "\\textsuperscript{2}",
        '³' => "\\textsuperscript{3}",
        'µ' => "\\ensuremath{\\mu}",
        '¶' => "\\P{}",
        '·' => "\\textperiodcentered{}",
        '¹' => "\\textsuperscript{1}",
        'º' => "\\textordmasculine{}",
        '»' => "\\guillemotright{}",
        '¼' => "\\textonequarter{}",
        '½' => "\\textonehalf{}",
        '¾' => "\\textthreequarters{}",
        '¿' => "\\textquestiondown{}",
        '×' => "\\ensuremath{\\times}",
        '÷' => "\\ensuremath{\\div}",
        '€' => "\\texteuro{}",
        '™' => "\\texttrademark{}",
        '•' => "\\textbullet{}",
        '†' => "\\dag{}",
        '‡' => "\\ddag{}",
        '‰' => "\\textperthousand{}",
        '−' => "\\ensuremath{-}",
        _ => return None,
    })
}

pub fn escape_url(url: &str) -> String {
    url.chars()
        .filter(|c| !matches!(c, '\\' | '{' | '}' | '\n' | '\r'))
        .flat_map(|c| match c {
            '%' => vec!['\\', '%'],
            '#' => vec!['\\', '#'],
            c => vec![c],
        })
        .collect()
}

/// Formats a number compactly for LaTeX: `12`, `1.5`, `0.25`.
pub fn num(v: f32) -> String {
    let s = format!("{:.2}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// What `{n}` means where a placeholder appears (heading labels only).
#[derive(Clone, Copy)]
pub struct PlaceholderContext<'a> {
    /// LaTeX for `{n}`, e.g. `\thechapter`.
    pub number: Option<&'a str>,
    /// The template's languages: words in another script are kept in their own
    /// direction ("Page {page} of {pages}" in an Arabic document). `None`: as is.
    pub langs: Option<&'a crate::language::Languages>,
}

/// Renders template text: literals are escaped, placeholders become macros.
/// Must be used inside the `.cls` (relies on `@` being a letter).
pub fn render_placeholders(text: &str, ctx: PlaceholderContext<'_>) -> String {
    if let Some(langs) = ctx.langs.filter(|l| !l.english_only()) {
        // One run of English words can go across placeholders ("Page \thepage of …").
        let mut pieces = Vec::new();
        let mut last = None;
        for (i, piece) in placeholder::parse(text).into_iter().enumerate() {
            match piece {
                Piece::Literal(l) => pieces.extend(crate::language::split(i, &l, &mut last)),
                name => pieces.push(crate::language::Piece::latex(i, render_one(name, ctx))),
            }
        }
        return crate::language::join(&pieces, langs.main, langs, &|_, t| escape(t));
    }
    let mut out = String::new();
    for piece in placeholder::parse(text) {
        match piece {
            Piece::Literal(l) => out.push_str(&escape(&l)),
            Piece::Name("page") => out.push_str("\\thepage{}"),
            Piece::Name("pages") => out.push_str("\\pageref*{LastPage}"),
            Piece::Name("chapter") => out.push_str("\\leftmark{}"),
            Piece::Name("section") => out.push_str("\\rightmark{}"),
            Piece::Name("n") => out.push_str(ctx.number.unwrap_or("")),
            Piece::Name(field) => out.push_str(&format!("\\tl@f{{{field}}}")),
        }
    }
    out
}

/// One placeholder as LaTeX.
fn render_one(piece: Piece, ctx: PlaceholderContext<'_>) -> String {
    match piece {
        Piece::Literal(l) => escape(&l),
        Piece::Name("page") => "\\thepage{}".into(),
        Piece::Name("pages") => "\\pageref*{LastPage}".into(),
        Piece::Name("chapter") => "\\leftmark{}".into(),
        Piece::Name("section") => "\\rightmark{}".into(),
        Piece::Name("n") => ctx.number.unwrap_or("").into(),
        Piece::Name(field) => format!("\\tl@f{{{field}}}"),
    }
}

/// Font switches for a text style: size, weight, shape and colour (not alignment).
pub fn font_commands(s: &TextStyle) -> String {
    let size = num(s.size_pt);
    let leading = num(s.size_pt * 1.2);
    let mut out = format!("\\fontsize{{{size}}}{{{leading}}}\\selectfont");
    if s.bold {
        out.push_str("\\bfseries");
    }
    if s.italic {
        out.push_str("\\itshape");
    }
    if s.small_caps {
        out.push_str("\\scshape");
    }
    if let Some(c) = &s.color {
        out.push_str(&format!("\\color[HTML]{{{}}}", c.trim_start_matches('#').to_uppercase()));
    }
    out
}

pub fn align_name(a: Align) -> &'static str {
    match a {
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_specials() {
        assert_eq!(
            escape(r"50% & $5 #1 a_b {x} ~ ^ \"),
            r"50\% \& \$5 \#1 a\_b \{x\} \textasciitilde{} \textasciicircum{} \textbackslash{}"
        );
    }

    #[test]
    fn typographic_punctuation_uses_tex_ligatures() {
        assert_eq!(escape("\u{201C}It\u{2019}s\u{201D} \u{2014} 1\u{2013}2\u{2026}"), "``It's'' --- 1--2\\ldots{}");
        assert_eq!(escape("5 m³ at 20 °C ± 2"), "5 m\\textsuperscript{3} at 20 \\textdegree{}C \\ensuremath{\\pm} 2");
        assert_eq!(escape("café"), "café", "accented letters are fine as they are");
    }

    #[test]
    fn formats_numbers() {
        assert_eq!(num(12.0), "12");
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(0.25), "0.25");
        assert_eq!(num(-0.0), "0");
    }

    #[test]
    fn renders_placeholders_safely() {
        let ctx = PlaceholderContext { number: Some("\\thechapter"), langs: None };
        assert_eq!(render_placeholders("Chapter {n}", ctx), "Chapter \\thechapter");
        assert_eq!(
            render_placeholders("{title} — p. {page}/{pages} 100%", ctx),
            "\\tl@f{title} --- p. \\thepage{}/\\pageref*{LastPage} 100\\%"
        );
    }
}
