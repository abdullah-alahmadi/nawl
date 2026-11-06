//! Template and metadata checks. Errors make a template unusable; warnings are shown
//! to the user but generation still works (e.g. a required field not filled in yet
//! renders as "[Label]").

use std::collections::HashSet;

use serde::Serialize;
use ts_rs::TS;

use crate::placeholder;
use crate::{FieldKind, FrontMatterItem, Metadata, Template, TextStyle, TitleContent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Issue {
    pub severity: Severity,
    /// Where the problem is, e.g. `headings.levels[0].label`.
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{s}: {}: {}", self.path, self.message)
    }
}

pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.severity == Severity::Error)
}

struct Checker<'t> {
    t: &'t Template,
    issues: Vec<Issue>,
}

impl Checker<'_> {
    fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue { severity: Severity::Error, path: path.into(), message: message.into() });
    }

    fn warn(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue { severity: Severity::Warning, path: path.into(), message: message.into() });
    }

    fn positive(&mut self, path: &str, v: f32) {
        if !(v.is_finite() && v > 0.0) {
            self.error(path, format!("must be greater than 0 (got {v})"));
        }
    }

    fn non_negative(&mut self, path: &str, v: f32) {
        if !(v.is_finite() && v >= 0.0) {
            self.error(path, format!("must not be negative (got {v})"));
        }
    }

    /// Placeholders must name a field or a dynamic value valid in this context.
    fn placeholders(&mut self, path: &str, text: &str, allow_n: bool) {
        for name in placeholder::names(text) {
            let ok = if name == "n" {
                allow_n
            } else {
                placeholder::is_dynamic(name) || self.t.field(name).is_some()
            };
            if !ok {
                self.error(path, format!("unknown placeholder {{{name}}}"));
            }
        }
    }

    fn text_style(&mut self, path: &str, s: &TextStyle) {
        self.positive(&format!("{path}.sizePt"), s.size_pt);
        if let Some(c) = &s.color {
            let hex = c.strip_prefix('#').unwrap_or("");
            if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                self.error(format!("{path}.color"), format!("expected #RRGGBB, got {c:?}"));
            }
        }
    }
}

impl Template {
    pub fn validate(&self) -> Vec<Issue> {
        let mut c = Checker { t: self, issues: Vec::new() };
        let t = self;

        let id = &t.meta.id;
        let id_ok = id.chars().next().is_some_and(|ch| ch.is_ascii_lowercase())
            && id.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
        if !id_ok {
            c.error("meta.id", "must start with a-z and contain only a-z, 0-9 and '-'");
        }

        let (w, h) = t.page.paper.size_mm();
        c.positive("page.paper.widthMm", w);
        c.positive("page.paper.heightMm", h);
        let m = &t.page.margins;
        c.positive("page.margins.topMm", m.top_mm);
        c.positive("page.margins.bottomMm", m.bottom_mm);
        c.positive("page.margins.innerMm", m.inner_mm);
        c.positive("page.margins.outerMm", m.outer_mm);
        c.non_negative("page.bindingOffsetMm", t.page.binding_offset_mm);
        if m.inner_mm + m.outer_mm + t.page.binding_offset_mm >= w * 0.8
            || m.top_mm + m.bottom_mm >= h * 0.8
        {
            c.error("page.margins", "margins leave too little room for text");
        }
        if let crate::LineSpacing::Custom { stretch } = t.page.line_spacing {
            c.positive("page.lineSpacing.stretch", stretch);
        }
        c.non_negative("page.paragraph.indentMm", t.page.paragraph.indent_mm);
        c.non_negative("page.paragraph.spacingPt", t.page.paragraph.spacing_pt);

        if ![10, 11, 12].contains(&t.fonts.base_size_pt) {
            c.error("fonts.baseSizePt", "must be 10, 11 or 12");
        }
        for (name, f) in [("main", &t.fonts.main), ("sans", &t.fonts.sans), ("mono", &t.fonts.mono)] {
            if let crate::FontChoice::System { family } = f
                && family.trim().is_empty()
            {
                c.error(format!("fonts.{name}.family"), "font family is empty");
            }
        }

        if t.structure.numbered_depth > 4 {
            c.error("structure.numberedDepth", "must be 0–4");
        }
        if t.structure.toc_depth > 4 {
            c.error("structure.tocDepth", "must be 0–4");
        }

        // Fields first: other checks look placeholders up by field id.
        let mut seen = HashSet::new();
        for (i, f) in t.fields.iter().enumerate() {
            let p = format!("fields[{i}].id");
            if !placeholder::is_valid_name(&f.id) {
                c.error(&p, format!("{:?} must match [a-z][a-z0-9_]*", f.id));
            } else if placeholder::is_dynamic(&f.id) {
                c.error(&p, format!("{:?} is reserved", f.id));
            }
            if !seen.insert(f.id.as_str()) {
                c.error(&p, format!("duplicate field id {:?}", f.id));
            }
        }

        if t.headings.levels.len() != 4 {
            c.error("headings.levels", format!("must have exactly 4 levels (has {})", t.headings.levels.len()));
        }
        for (i, h) in t.headings.levels.iter().enumerate() {
            let p = format!("headings.levels[{i}]");
            c.text_style(&format!("{p}.text"), &h.text);
            c.placeholders(&format!("{p}.label"), &h.label, true);
            c.non_negative(&format!("{p}.labelGapPt"), h.label_gap_pt);
        }

        let hf = &t.header_footer;
        for (name, style) in [("frontMatter", &hf.front_matter), ("main", &hf.main), ("chapterOpening", &hf.chapter_opening)] {
            for (slot, text) in [
                ("header.left", &style.header.left),
                ("header.center", &style.header.center),
                ("header.right", &style.header.right),
                ("footer.left", &style.footer.left),
                ("footer.center", &style.footer.center),
                ("footer.right", &style.footer.right),
            ] {
                let path = format!("headerFooter.{name}.{slot}");
                c.placeholders(&path, text, false);
                for n in placeholder::names(text) {
                    if t.field(n).is_some_and(|f| matches!(f.kind, FieldKind::Paragraphs | FieldKind::Authors)) {
                        c.error(&path, format!("{{{n}}} has several paragraphs and can't go in a header or footer"));
                    }
                }
            }
            c.non_negative(&format!("headerFooter.{name}.headerRulePt"), style.header_rule_pt);
            c.non_negative(&format!("headerFooter.{name}.footerRulePt"), style.footer_rule_pt);
        }

        for (i, item) in t.title_page.items.iter().enumerate() {
            let p = format!("titlePage.items[{i}]");
            match &item.content {
                TitleContent::Text { text, style, width_mm } => {
                    c.placeholders(&format!("{p}.content.text"), text, false);
                    for n in placeholder::names(text) {
                        if matches!(n, "chapter" | "section") {
                            c.error(format!("{p}.content.text"), format!("{{{n}}} has no value on the title page"));
                        }
                    }
                    c.text_style(&format!("{p}.content.style"), style);
                    if let Some(w) = width_mm {
                        c.positive(&format!("{p}.content.widthMm"), *w);
                    }
                }
                TitleContent::Image { asset, width_mm } => {
                    c.positive(&format!("{p}.content.widthMm"), *width_mm);
                    let safe = !asset.is_empty()
                        && !asset.starts_with('/')
                        && !asset.contains('\\')
                        && asset.split('/').all(|s| !s.is_empty() && s != "..");
                    if !safe {
                        c.error(format!("{p}.content.asset"), "must be a relative path inside the template, like assets/logo.png");
                    }
                }
                TitleContent::Rule { length_mm, thickness_pt } => {
                    c.positive(&format!("{p}.content.lengthMm"), *length_mm);
                    c.positive(&format!("{p}.content.thicknessPt"), *thickness_pt);
                }
            }
        }

        for (name, label) in [("figureLabel", &t.captions.figure_label), ("tableLabel", &t.captions.table_label)] {
            if label.trim().is_empty() {
                c.error(format!("captions.{name}"), "must not be empty");
            }
        }

        if t.kind == crate::DocumentKind::Presentation {
            let p = t.presentation.clone().unwrap_or_default();
            let name_ok = |s: &str| s.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-');
            if p.theme.trim().is_empty() || !name_ok(&p.theme) {
                c.error("presentation.theme", "needs a beamer theme name such as Madrid (letters, digits, '-')");
            }
            if !name_ok(&p.color_theme) {
                c.error("presentation.colorTheme", "colour theme names may only contain letters, digits and '-'");
            }
            if !name_ok(&p.font_theme) {
                c.error("presentation.fontTheme", "font theme names may only contain letters, digits and '-'");
            }
            if !(8..=20).contains(&p.font_size_pt) {
                c.error("presentation.fontSizePt", "beamer font sizes are 8 to 20 pt");
            }
            if t.base.is_some() {
                c.error("base", "a presentation can't also be built on a publisher class");
            }
        }
        if let Some(base) = &t.base {
            let name_ok = !base.class.is_empty()
                && base.class.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_./".contains(ch))
                && !base.class.contains("..");
            if !name_ok {
                c.error("base.class", "class names may only contain letters, digits, '-', '_', '.', '/'");
            }
            // Empty: the class sets the style itself (jmlr, elbioimp…).
            if base.bib_style.contains(['{', '}', '\\', ' ']) {
                c.error("base.bibStyle", "needs a BibTeX style name such as IEEEtran (or empty when the class sets it)");
            }
            for (i, o) in base.options.iter().enumerate() {
                if o.contains(['{', '}', '\\', ',', ']']) || o.trim().is_empty() {
                    c.error(format!("base.options[{i}]"), format!("invalid class option {o:?}"));
                }
            }
            for (i, f) in base.files.iter().enumerate() {
                let safe = !f.starts_with('/') && !f.contains('\\') && f.split('/').all(|s| !s.is_empty() && s != "..");
                if !safe {
                    c.error(format!("base.files[{i}]"), "must be a relative path inside the template");
                }
            }
            for id in ["title", "authors"] {
                if t.field(id).is_none() {
                    c.error("fields", format!("templates built on a publisher class need a {id:?} field"));
                }
            }
            if base.front_matter == crate::FrontMatterStyle::Custom
                && base.title_block.as_deref().is_none_or(|r| r.trim().is_empty())
            {
                c.error("base.titleBlock", "the custom style needs a title block recipe");
            }
            if let Some(kit) = &base.kit {
                let have: Vec<&str> = base.files.iter().map(|f| f.rsplit('/').next().unwrap_or(f)).collect();
                let missing: Vec<&str> =
                    kit.required.iter().map(String::as_str).filter(|r| !have.contains(r)).collect();
                if !missing.is_empty() {
                    c.warn(
                        "base.kit",
                        format!("needs the {} ({}); import it under Publisher class", kit.name, missing.join(", ")),
                    );
                }
            }
        }
        if t.bibliography.title.trim().is_empty() {
            c.error("bibliography.title", "must not be empty");
        }
        c.placeholders("structure.appendixLabel", &t.structure.appendix_label, true);
        for (i, sec) in t.skeleton.iter().enumerate() {
            if sec.title.trim().is_empty() {
                c.error(format!("skeleton[{i}].title"), "must not be empty");
            }
        }

        let mut has_title_page = false;
        for (i, item) in t.front_matter.iter().enumerate() {
            match item {
                FrontMatterItem::TitlePage => has_title_page = true,
                FrontMatterItem::FieldPage { field, .. } => match t.field(field) {
                    None => c.error(format!("frontMatter[{i}].field"), format!("no field with id {field:?}")),
                    Some(f) if f.kind != FieldKind::Paragraphs => {
                        c.error(format!("frontMatter[{i}].field"), format!("field {field:?} must be of kind paragraphs"))
                    }
                    Some(_) => {}
                },
                _ => {}
            }
        }
        if has_title_page && t.title_page.items.is_empty() {
            c.warn("titlePage.items", "the title page is empty");
        }

        c.issues
    }

    /// Checks a whole document: metadata, required chapters, broken cross-references
    /// and (when `known_keys` is given) citations of unknown bibliography entries.
    pub fn check_document(
        &self,
        doc: &crate::Document,
        known_keys: Option<&HashSet<String>>,
    ) -> Vec<Issue> {
        let warn = |path: String, message: String| Issue { severity: Severity::Warning, path, message };
        let mut issues = self.check_metadata(&doc.metadata);
        let titles: Vec<String> = doc.body.top_level_titles().iter().map(|t| t.trim().to_lowercase()).collect();
        let level = if self.structure.top_level == crate::TopLevel::Chapter { "chapter" } else { "section" };
        for sec in self.skeleton.iter().filter(|s| s.required) {
            if !titles.contains(&sec.title.trim().to_lowercase()) {
                issues.push(warn(
                    "document.skeleton".into(),
                    format!("Required {level} \"{}\" is missing", sec.title.trim()),
                ));
            }
        }
        let broken = doc.body.broken_refs().len();
        if broken > 0 {
            issues.push(warn(
                "document.refs".into(),
                format!("{broken} cross-reference{} point{} to something that was deleted", if broken > 1 { "s" } else { "" }, if broken > 1 { "" } else { "s" }),
            ));
        }
        if let Some(known) = known_keys {
            for key in doc.body.cited_keys() {
                if !known.contains(&key) {
                    issues.push(warn("document.citations".into(), format!("Citation \"{key}\" isn't in the references")));
                }
            }
        }
        issues
    }

    /// Checks a document's metadata against this template's fields.
    pub fn check_metadata(&self, metadata: &Metadata) -> Vec<Issue> {
        let mut issues = Vec::new();
        for f in &self.fields {
            let value = metadata.get(&f.id);
            if f.required && value.is_none_or(|v| v.is_empty()) {
                issues.push(Issue {
                    severity: Severity::Warning,
                    path: format!("metadata.{}", f.id),
                    message: format!("\"{}\" is required but empty", f.label),
                });
            }
        }
        for key in metadata.keys() {
            if self.field(key).is_none() {
                issues.push(Issue {
                    severity: Severity::Warning,
                    path: format!("metadata.{key}"),
                    message: "not a field of this template; ignored".into(),
                });
            }
        }
        issues
    }
}
