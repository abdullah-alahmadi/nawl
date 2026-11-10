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
            self.error(path, crate::tr!("must be greater than 0 (got {v})", "يجب أن تكون أكبر من 0 (القيمة {v})"));
        }
    }

    fn non_negative(&mut self, path: &str, v: f32) {
        if !(v.is_finite() && v >= 0.0) {
            self.error(path, crate::tr!("must not be negative (got {v})", "يجب ألا تكون سالبة (القيمة {v})"));
        }
    }

    /// Placeholders must name a field or a dynamic value valid in this context.
    fn placeholders(&mut self, path: &str, text: &str, allow_n: bool) {
        for name in placeholder::names(text) {
            let ok = if name == "n" {
                allow_n
            } else {
                placeholder::is_dynamic(name)
                    || self.t.field(name).is_some()
                    // A date field's Hijri form: {defense_date_hijri}.
                    || name
                        .strip_suffix(crate::date::HIJRI_SUFFIX)
                        .and_then(|id| self.t.field(id))
                        .is_some_and(|f| f.kind == crate::FieldKind::Date)
                    // An authors field's affiliations, English names, e-mail.
                    || crate::split_author_placeholder(name)
                        .and_then(|(id, _)| self.t.field(id))
                        .is_some_and(|f| f.kind == crate::FieldKind::Authors)
            };
            if !ok {
                self.error(path, crate::tr!("unknown placeholder {{{name}}}", "عنصر نائب غير معروف {{{name}}}"));
            }
        }
    }

    fn text_style(&mut self, path: &str, s: &TextStyle) {
        self.positive(&format!("{path}.sizePt"), s.size_pt);
        if let Some(c) = &s.color {
            let hex = c.strip_prefix('#').unwrap_or("");
            if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                self.error(format!("{path}.color"), crate::tr!("expected #RRGGBB, got {c:?}", "المتوقع لون بالصيغة #RRGGBB، والموجود {c:?}"));
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
            c.error("meta.id", crate::tr!("must start with a-z and contain only a-z, 0-9 and '-'", "يجب أن يبدأ بحرف من a-z وألا يحوي إلا a-z و0-9 و'-'"));
        }
        // The id names the generated class (<id>.cls), which loads LaTeX's own
        // classes: one of their names would load itself.
        if ["article", "report", "book", "letter", "slides", "beamer", "memoir", "minimal", "proc"].contains(&id.as_str()) {
            c.error("meta.id", crate::tr!("\"{id}\" is the name of a LaTeX class; choose another id", "\"{id}\" اسم صنف في LaTeX؛ اختر معرّفًا آخر"));
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
            c.error("page.margins", crate::tr!("margins leave too little room for text", "الهوامش لا تترك مساحة كافية للنص"));
        }
        if !(1..=2).contains(&t.page.columns) {
            c.error("page.columns", crate::tr!("must be 1 or 2", "يجب أن يكون 1 أو 2"));
        }
        if let Some(height) = t.title_page.block_height_mm
            && !(height > m.top_mm && height < h - m.bottom_mm - 30.0)
        {
            c.error(
                "titlePage.blockHeightMm",
                crate::tr!(
                    "the text must start below the top margin and leave room on the first page",
                    "يجب أن يبدأ النص أسفل الهامش العلوي وأن يترك مساحة في الصفحة الأولى"
                ),
            );
        }
        if let crate::LineSpacing::Custom { stretch } = t.page.line_spacing {
            c.positive("page.lineSpacing.stretch", stretch);
        }
        c.non_negative("page.paragraph.indentMm", t.page.paragraph.indent_mm);
        c.non_negative("page.paragraph.spacingPt", t.page.paragraph.spacing_pt);

        if ![10, 11, 12].contains(&t.fonts.base_size_pt) {
            c.error("fonts.baseSizePt", crate::tr!("must be 10, 11 or 12", "يجب أن يكون 10 أو 11 أو 12"));
        }
        for (name, f) in [("main", &t.fonts.main), ("sans", &t.fonts.sans), ("mono", &t.fonts.mono)] {
            if let crate::FontChoice::System { family } = f
                && family.trim().is_empty()
            {
                c.error(format!("fonts.{name}.family"), crate::tr!("font family is empty", "اسم عائلة الخط فارغ"));
            }
        }
        let mut styled: Vec<(String, Option<&crate::FontChoice>)> = vec![
            ("fonts.main".into(), Some(&t.fonts.main)),
            ("fonts.sans".into(), Some(&t.fonts.sans)),
            ("fonts.mono".into(), Some(&t.fonts.mono)),
            ("fonts.arabic".into(), t.fonts.arabic.as_ref()),
            ("fonts.arabicMath".into(), t.fonts.arabic_math.as_ref()),
        ];
        for (i, h) in t.headings.levels.iter().enumerate() {
            styled.push((format!("headings.levels[{i}].text.font"), h.text.font.as_ref()));
        }
        if let Some(part) = &t.headings.part {
            styled.push(("headings.part.text.font".into(), part.text.font.as_ref()));
        }
        for (path, item) in t.placed_items() {
            if let crate::TitleContent::Text { style, .. } = &item.content {
                styled.push((format!("{path}.content.style.font"), style.font.as_ref()));
            }
        }
        for (name, f) in styled {
            if let Some(crate::FontChoice::Library { id }) = f
                && crate::fonts::find(id).is_none()
            {
                c.error(if name.starts_with("fonts.") { format!("{name}.id") } else { name.clone() }, crate::tr!("{id:?} isn't in Texloom's font library", "{id:?} ليس من مكتبة خطوط Texloom"));
            }
        }

        if t.structure.numbered_depth > 4 {
            c.error("structure.numberedDepth", crate::tr!("must be 0–4", "يجب أن يكون من 0 إلى 4"));
        }
        if t.structure.toc_depth > 4 {
            c.error("structure.tocDepth", crate::tr!("must be 0–4", "يجب أن يكون من 0 إلى 4"));
        }

        // Fields first: other checks look placeholders up by field id.
        let mut seen = HashSet::new();
        for (i, f) in t.fields.iter().enumerate() {
            let p = format!("fields[{i}].id");
            if !placeholder::is_valid_name(&f.id) {
                c.error(&p, crate::tr!("{:?} must match [a-z][a-z0-9_]*", "{:?} يجب أن يطابق [a-z][a-z0-9_]*", f.id));
            } else if placeholder::is_dynamic(&f.id) {
                c.error(&p, crate::tr!("{:?} is reserved", "{:?} محجوز", f.id));
            }
            if !seen.insert(f.id.as_str()) {
                c.error(&p, crate::tr!("duplicate field id {:?}", "معرّف حقل مكرر {:?}", f.id));
            }
        }

        if t.headings.levels.len() != 4 {
            c.error("headings.levels", crate::tr!("must have exactly 4 levels (has {})", "يجب أن تكون 4 مستويات بالضبط (الموجود {})", t.headings.levels.len()));
        }
        for (i, h) in t.headings.levels.iter().enumerate() {
            let p = format!("headings.levels[{i}]");
            c.text_style(&format!("{p}.text"), &h.text);
            c.placeholders(&format!("{p}.label"), &h.label, true);
            c.non_negative(&format!("{p}.labelGapPt"), h.label_gap_pt);
        }
        if let Some(part) = &t.headings.part {
            c.text_style("headings.part.text", &part.text);
            c.placeholders("headings.part.label", &part.label, true);
            c.non_negative("headings.part.labelGapPt", part.label_gap_pt);
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
                        c.error(&path, crate::tr!(
                            "{{{n}}} has several paragraphs and can't go in a header or footer",
                            "{{{n}}} يتكون من عدة فقرات ولا يمكن وضعه في رأس الصفحة أو تذييلها"
                        ));
                    }
                }
            }
            c.non_negative(&format!("headerFooter.{name}.headerRulePt"), style.header_rule_pt);
            c.non_negative(&format!("headerFooter.{name}.footerRulePt"), style.footer_rule_pt);
        }

        for (p, item) in t.placed_items() {
            match &item.content {
                TitleContent::Text { text, style, width_mm } => {
                    c.placeholders(&format!("{p}.content.text"), text, false);
                    for n in placeholder::names(text) {
                        if matches!(n, "chapter" | "section") {
                            c.error(format!("{p}.content.text"), crate::tr!("{{{n}}} has no value outside chapters", "{{{n}}} ليس له قيمة خارج الفصول"));
                        }
                    }
                    c.text_style(&format!("{p}.content.style"), style);
                    if let Some(w) = width_mm {
                        c.positive(&format!("{p}.content.widthMm"), *w);
                    }
                }
                TitleContent::Image { asset, width_mm } => {
                    c.positive(&format!("{p}.content.widthMm"), *width_mm);
                    if let Some(id) = crate::template::image_field(asset) {
                        if !t.field(id).is_some_and(|f| f.kind == FieldKind::Image) {
                            c.error(format!("{p}.content.asset"), crate::tr!(
                                "{{{id}}} is not an image field of this template",
                                "{{{id}}} ليس حقل صورة في هذا القالب"
                            ));
                        }
                        continue;
                    }
                    let safe = !asset.is_empty()
                        && !asset.starts_with('/')
                        && !asset.contains('\\')
                        && asset.split('/').all(|s| !s.is_empty() && s != "..");
                    if !safe {
                        c.error(format!("{p}.content.asset"), crate::tr!(
                            "must be a relative path inside the template, like assets/logo.png",
                            "يجب أن يكون مسارًا نسبيًا داخل القالب، مثل assets/logo.png"
                        ),
                    );
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
                c.error(format!("captions.{name}"), crate::tr!("must not be empty", "يجب ألا يكون فارغًا"));
            }
        }

        if t.kind == crate::DocumentKind::Presentation {
            let p = t.presentation.clone().unwrap_or_default();
            let name_ok = |s: &str| s.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-');
            if p.theme.trim().is_empty() || !name_ok(&p.theme) {
                c.error(
                    "presentation.theme",
                    crate::tr!(
                        "needs a beamer theme name such as Madrid (letters, digits, '-')",
                        "يلزم اسم سمة beamer مثل Madrid (حروف وأرقام و'-')"
                    ),
                );
            }
            if !name_ok(&p.color_theme) {
                c.error("presentation.colorTheme", crate::tr!(
                        "colour theme names may only contain letters, digits and '-'",
                        "لا يحوي اسم سمة الألوان إلا الحروف والأرقام و'-'"
                    ),
                );
            }
            if !name_ok(&p.font_theme) {
                c.error("presentation.fontTheme", crate::tr!(
                        "font theme names may only contain letters, digits and '-'",
                        "لا يحوي اسم سمة الخطوط إلا الحروف والأرقام و'-'"
                    ),
                );
            }
            if let Some(color) = &p.accent_color {
                let hex = color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|ch| ch.is_ascii_hexdigit());
                if !hex {
                    c.error("presentation.accentColor", crate::tr!("needs a colour like #1f4e79", "يلزم لون مثل #1f4e79"));
                }
            }
            if !(8..=20).contains(&p.font_size_pt) {
                c.error("presentation.fontSizePt", crate::tr!("beamer font sizes are 8 to 20 pt", "أحجام خط beamer من 8 إلى 20 نقطة"));
            }
            if t.base.is_some() {
                c.error(
                    "base",
                    crate::tr!(
                        "a presentation can't also be built on a publisher class",
                        "لا يمكن بناء عرض تقديمي على صنف ناشر أيضًا"
                    ),
                );
            }
        }
        if let Some(base) = &t.base {
            let name_ok = !base.class.is_empty()
                && base.class.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_./".contains(ch))
                && !base.class.contains("..");
            if !name_ok {
                c.error(
                    "base.class",
                    crate::tr!(
                        "class names may only contain letters, digits, '-', '_', '.', '/'",
                        "لا يحوي اسم الصنف إلا الحروف والأرقام و'-' و'_' و'.' و'/'"
                    ),
                );
            }
            // Empty: the class sets the style itself (jmlr, elbioimp…).
            if base.bib_style.contains(['{', '}', '\\', ' ']) {
                c.error(
                    "base.bibStyle",
                    crate::tr!(
                        "needs a BibTeX style name such as IEEEtran (or empty when the class sets it)",
                        "يلزم اسم نمط BibTeX مثل IEEEtran (أو يُترك فارغًا إذا حدده الصنف)"
                    ),
                );
            }
            for (i, o) in base.options.iter().enumerate() {
                if o.contains(['{', '}', '\\', ',', ']']) || o.trim().is_empty() {
                    c.error(format!("base.options[{i}]"), crate::tr!("invalid class option {o:?}", "خيار صنف غير صالح {o:?}"));
                }
            }
            for (i, f) in base.files.iter().enumerate() {
                let safe = !f.starts_with('/') && !f.contains('\\') && f.split('/').all(|s| !s.is_empty() && s != "..");
                if !safe {
                    c.error(
                        format!("base.files[{i}]"),
                        crate::tr!("must be a relative path inside the template", "يجب أن يكون مسارًا نسبيًا داخل القالب"),
                    );
                }
            }
            for id in ["title", "authors"] {
                if t.field(id).is_none() {
                    c.error("fields", crate::tr!(
                        "templates built on a publisher class need a {id:?} field",
                        "القوالب المبنية على صنف ناشر تحتاج إلى حقل {id:?}"
                    ));
                }
            }
            if base.front_matter == crate::FrontMatterStyle::Custom
                && base.title_block.as_deref().is_none_or(|r| r.trim().is_empty())
            {
                c.error(
                    "base.titleBlock",
                    crate::tr!("the custom style needs a title block recipe", "النمط المخصص يحتاج إلى وصفة لكتلة العنوان"),
                );
            }
            if let Some(kit) = &base.kit {
                let have: Vec<&str> = base.files.iter().map(|f| f.rsplit('/').next().unwrap_or(f)).collect();
                let missing: Vec<&str> =
                    kit.required.iter().map(String::as_str).filter(|r| !have.contains(r)).collect();
                if !missing.is_empty() {
                    c.warn(
                        "base.kit",
                        crate::tr!(
                            "needs the {} ({}); import it under Publisher class",
                            "يحتاج إلى {} ({})؛ استورده من قسم صنف الناشر",
                            kit.name,
                            missing.join(", ")
                        ),
                    );
                }
            }
        }
        if t.bibliography.title.trim().is_empty() {
            c.error("bibliography.title", crate::tr!("must not be empty", "يجب ألا يكون فارغًا"));
        }
        c.placeholders("structure.appendixLabel", &t.structure.appendix_label, true);
        for (i, sec) in t.skeleton.iter().enumerate() {
            if sec.title.trim().is_empty() {
                c.error(format!("skeleton[{i}].title"), crate::tr!("must not be empty", "يجب ألا يكون فارغًا"));
            }
        }

        let mut has_title_page = false;
        for (i, item) in t.front_matter.iter().enumerate() {
            match item {
                FrontMatterItem::TitlePage => has_title_page = true,
                FrontMatterItem::FieldPage { field, .. } => match t.field(field) {
                    None => c.error(format!("frontMatter[{i}].field"), crate::tr!("no field with id {field:?}", "لا يوجد حقل بالمعرّف {field:?}")),
                    Some(f) if f.kind != FieldKind::Paragraphs => {
                        c.error(format!("frontMatter[{i}].field"), crate::tr!("field {field:?} must be of kind paragraphs", "الحقل {field:?} يجب أن يكون من نوع فقرات"))
                    }
                    Some(_) => {}
                },
                FrontMatterItem::FixedPage { title, items, in_toc } => {
                    if *in_toc && title.trim().is_empty() {
                        c.error(format!("frontMatter[{i}].title"), crate::tr!(
                                "needs a title to be listed in the table of contents",
                                "يحتاج إلى عنوان ليظهر في جدول المحتويات"
                            ),
                        );
                    }
                    if items.is_empty() {
                        c.warn(format!("frontMatter[{i}].items"), crate::tr!("the page is empty", "الصفحة فارغة"));
                    }
                }
                _ => {}
            }
        }
        if has_title_page && t.title_page.items.is_empty() {
            c.warn("titlePage.items", crate::tr!("the title page is empty", "صفحة العنوان فارغة"));
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
                    if level == "chapter" {
                        crate::tr!("Required chapter \"{}\" is missing", "الفصل المطلوب \"{}\" غير موجود", sec.title.trim())
                    } else {
                        crate::tr!("Required section \"{}\" is missing", "القسم المطلوب \"{}\" غير موجود", sec.title.trim())
                    },
                ));
            }
        }
        let broken = doc.body.broken_refs().len();
        if broken > 0 {
            issues.push(warn(
                "document.refs".into(),
                if broken > 1 {
                    crate::tr!(
                        "{broken} cross-references point to something that was deleted",
                        "عدد الإحالات التي تشير إلى محتوى محذوف: {broken}"
                    )
                } else {
                    crate::tr!(
                        "{broken} cross-reference points to something that was deleted",
                        "إحالة واحدة تشير إلى محتوى محذوف"
                    )
                },
            ));
        }
        if let Some(known) = known_keys {
            for key in doc.body.cited_keys() {
                if !known.contains(&key) {
                    issues.push(warn("document.citations".into(), crate::tr!(
                            "Citation \"{key}\" isn't in the references",
                            "الاستشهاد \"{key}\" غير موجود في المراجع"
                        )));
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
                    message: crate::tr!("\"{}\" is required but empty", "\"{}\" مطلوب لكنه فارغ", f.label),
                });
            }
        }
        for key in metadata.keys() {
            if self.field(key).is_none() {
                issues.push(Issue {
                    severity: Severity::Warning,
                    path: format!("metadata.{key}"),
                    message: crate::tr!("not a field of this template; ignored", "ليس حقلًا في هذا القالب؛ تم تجاهله"),
                });
            }
        }
        issues
    }
}
