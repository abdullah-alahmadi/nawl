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
            _ => s.push(c),
        }
    }
    s
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
}

/// Renders template text: literals are escaped, placeholders become macros.
/// Must be used inside the `.cls` (relies on `@` being a letter).
pub fn render_placeholders(text: &str, ctx: PlaceholderContext<'_>) -> String {
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
    fn formats_numbers() {
        assert_eq!(num(12.0), "12");
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(0.25), "0.25");
        assert_eq!(num(-0.0), "0");
    }

    #[test]
    fn renders_placeholders_safely() {
        let ctx = PlaceholderContext { number: Some("\\thechapter") };
        assert_eq!(render_placeholders("Chapter {n}", ctx), "Chapter \\thechapter");
        assert_eq!(
            render_placeholders("{title} — p. {page}/{pages} 100%", ctx),
            "\\tl@f{title} — p. \\thepage{}/\\pageref*{LastPage} 100\\%"
        );
    }
}
