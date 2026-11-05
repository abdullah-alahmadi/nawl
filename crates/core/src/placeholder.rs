//! Placeholders in template text: `{title}`, `{page}`, `{n}`…
//!
//! Syntax: `{name}` where `name` is `[a-z][a-z0-9_]*`. `{{` and `}}` produce literal
//! braces. Any other brace is literal text, so typing "{" by accident never breaks.

/// Placeholders the code generator fills in itself; field ids may not use them.
pub const DYNAMIC: &[(&str, &str)] = &[
    ("page", "current page number"),
    ("pages", "total number of pages"),
    ("chapter", "current chapter title (or section, without chapters)"),
    ("section", "current section title (or subsection, without chapters)"),
    ("n", "heading number (heading labels only)"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece<'a> {
    Literal(String),
    Name(&'a str),
}

pub fn is_valid_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

pub fn is_dynamic(name: &str) -> bool {
    DYNAMIC.iter().any(|(n, _)| *n == name)
}

pub fn parse(s: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut lit = String::new();
    let mut i = 0;
    let b = s.as_bytes();
    while i < s.len() {
        if s[i..].starts_with("{{") {
            lit.push('{');
            i += 2;
        } else if s[i..].starts_with("}}") {
            lit.push('}');
            i += 2;
        } else if b[i] == b'{' {
            match s[i + 1..].find('}') {
                Some(end) if is_valid_name(&s[i + 1..i + 1 + end]) => {
                    if !lit.is_empty() {
                        out.push(Piece::Literal(std::mem::take(&mut lit)));
                    }
                    out.push(Piece::Name(&s[i + 1..i + 1 + end]));
                    i += end + 2;
                }
                _ => {
                    lit.push('{');
                    i += 1;
                }
            }
        } else {
            let c = s[i..].chars().next().expect("in bounds");
            lit.push(c);
            i += c.len_utf8();
        }
    }
    if !lit.is_empty() {
        out.push(Piece::Literal(lit));
    }
    out
}

/// All placeholder names used in `s`.
pub fn names(s: &str) -> impl Iterator<Item = &str> {
    parse(s).into_iter().filter_map(|p| match p {
        Piece::Name(n) => Some(n),
        Piece::Literal(_) => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Piece::*;

    #[test]
    fn parses_names_literals_and_escapes() {
        assert_eq!(
            parse("Page {page} of {pages} — {{x}} {Not} {"),
            vec![
                Literal("Page ".into()),
                Name("page"),
                Literal(" of ".into()),
                Name("pages"),
                Literal(" — {x} {Not} {".into()),
            ]
        );
    }
}
