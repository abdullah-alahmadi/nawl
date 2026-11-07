//! Paragraph and run properties, always written in the order Word's schema
//! requires (Word refuses files whose properties are out of order).

use crate::xml::esc;

/// Paragraph properties (`<w:pPr>`).
#[derive(Default, Clone)]
pub struct Pp {
    pub style: Option<String>,
    pub keep_next: bool,
    pub page_break_before: bool,
    /// List level and list instance.
    pub num: Option<(u8, usize)>,
    /// A bottom border (a rule): its width in eighths of a point.
    pub border_bottom: Option<i64>,
    /// Tab stops: (kind, position in twips).
    pub tabs: Vec<(&'static str, i64)>,
    /// Right to left (`Some(true)`), left to right in a right-to-left document
    /// (`Some(false)`), or the style's direction.
    pub bidi: Option<bool>,
    /// Space before and after (twips), and line height (twips, exact when set
    /// with `line_exact`).
    pub before: Option<i64>,
    pub after: Option<i64>,
    pub line: Option<i64>,
    pub line_exact: bool,
    /// Indentation attributes, as written (`w:left="567" w:firstLine="0"`).
    pub ind: Option<String>,
    pub jc: Option<&'static str>,
    pub outline: Option<usize>,
    /// Section properties ending a section here.
    pub sect: Option<String>,
}

impl Pp {
    pub fn xml(&self) -> String {
        let mut o = String::from("<w:pPr>");
        if let Some(s) = &self.style {
            o.push_str(&format!("<w:pStyle w:val=\"{}\"/>", esc(s)));
        }
        if self.keep_next {
            o.push_str("<w:keepNext/>");
        }
        if self.page_break_before {
            o.push_str("<w:pageBreakBefore/>");
        }
        if let Some((level, id)) = self.num {
            o.push_str(&format!("<w:numPr><w:ilvl w:val=\"{level}\"/><w:numId w:val=\"{id}\"/></w:numPr>"));
        }
        if let Some(sz) = self.border_bottom {
            o.push_str(&format!("<w:pBdr><w:bottom w:val=\"single\" w:sz=\"{sz}\" w:space=\"1\" w:color=\"auto\"/></w:pBdr>"));
        }
        if !self.tabs.is_empty() {
            o.push_str("<w:tabs>");
            for (kind, pos) in &self.tabs {
                o.push_str(&format!("<w:tab w:val=\"{kind}\" w:pos=\"{pos}\"/>"));
            }
            o.push_str("</w:tabs>");
        }
        match self.bidi {
            Some(true) => o.push_str("<w:bidi/>"),
            Some(false) => o.push_str("<w:bidi w:val=\"0\"/>"),
            None => {}
        }
        if self.before.is_some() || self.after.is_some() || self.line.is_some() {
            o.push_str("<w:spacing");
            if let Some(b) = self.before {
                o.push_str(&format!(" w:before=\"{b}\""));
            }
            if let Some(a) = self.after {
                o.push_str(&format!(" w:after=\"{a}\""));
            }
            if let Some(l) = self.line {
                o.push_str(&format!(" w:line=\"{l}\" w:lineRule=\"{}\"", if self.line_exact { "exact" } else { "auto" }));
            }
            o.push_str("/>");
        }
        if let Some(ind) = &self.ind {
            o.push_str(&format!("<w:ind {ind}/>"));
        }
        if let Some(jc) = self.jc {
            o.push_str(&format!("<w:jc w:val=\"{jc}\"/>"));
        }
        if let Some(level) = self.outline {
            o.push_str(&format!("<w:outlineLvl w:val=\"{level}\"/>"));
        }
        if let Some(sect) = &self.sect {
            o.push_str(sect);
        }
        o.push_str("</w:pPr>");
        o
    }
}

/// Run properties (`<w:rPr>`).
#[derive(Default, Clone)]
pub struct Rp {
    pub style: Option<&'static str>,
    /// A font for Latin letters (code).
    pub font: Option<String>,
    /// A font for every script, Arabic included (a heading's or a title's own font).
    pub face: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub caps: bool,
    pub small_caps: bool,
    pub strike: bool,
    pub color: Option<String>,
    /// Size in half-points.
    pub size: Option<i64>,
    pub underline: bool,
    /// Raised and smaller (an author's affiliation number).
    pub superscript: bool,
    pub rtl: bool,
}

impl Rp {
    pub fn bold() -> Self {
        Self { bold: true, ..Default::default() }
    }

    pub fn italic() -> Self {
        Self { italic: true, ..Default::default() }
    }

    pub fn xml(&self) -> String {
        let mut o = String::from("<w:rPr>");
        if let Some(s) = self.style {
            o.push_str(&format!("<w:rStyle w:val=\"{s}\"/>"));
        }
        if let Some(f) = &self.face {
            o.push_str(&format!("<w:rFonts w:ascii=\"{0}\" w:hAnsi=\"{0}\" w:cs=\"{0}\"/>", esc(f)));
        } else if let Some(f) = &self.font {
            o.push_str(&format!("<w:rFonts w:ascii=\"{0}\" w:hAnsi=\"{0}\"/>", esc(f)));
        }
        if self.bold {
            o.push_str("<w:b/><w:bCs/>");
        }
        if self.italic {
            o.push_str("<w:i/><w:iCs/>");
        }
        if self.caps {
            o.push_str("<w:caps/>");
        }
        if self.small_caps {
            o.push_str("<w:smallCaps/>");
        }
        if self.strike {
            o.push_str("<w:strike/>");
        }
        if let Some(c) = &self.color {
            o.push_str(&format!("<w:color w:val=\"{}\"/>", esc(c)));
        }
        if let Some(sz) = self.size {
            o.push_str(&format!("<w:sz w:val=\"{sz}\"/><w:szCs w:val=\"{sz}\"/>"));
        }
        if self.underline {
            o.push_str("<w:u w:val=\"single\"/>");
        }
        if self.superscript {
            o.push_str("<w:vertAlign w:val=\"superscript\"/>");
        }
        if self.rtl {
            o.push_str("<w:rtl/>");
        }
        o.push_str("</w:rPr>");
        o
    }
}
