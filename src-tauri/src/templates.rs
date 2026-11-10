//! The active template: the built-in one or a `template.json` on disk, which is
//! watched and reloaded whenever it (or its `assets/` folder) changes, so hand edits
//! show up in the preview right away.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use texloom_core::{has_errors, tr, Issue, Paper, Severity, Template, TitleContent};

use crate::AppError;

/// The built-in template until the designer (Phase 3) can create new ones.
pub const BUILTIN_TEMPLATE: &str = include_str!("../../templates/senior-design/template.json");

/// Templates that ship with Texloom, by id (the first is the default).
pub const BUILTIN_TEMPLATES: &[(&str, &str)] = &[
    ("senior-design", BUILTIN_TEMPLATE),
    ("engineering-report", include_str!("../../templates/engineering-report/template.json")),
    ("general-article", include_str!("../../templates/general-article/template.json")),
    ("article-two-column", include_str!("../../templates/article-two-column/template.json")),
    ("short-paper", include_str!("../../templates/short-paper/template.json")),
    ("ieee-journal", include_str!("../../templates/ieee-journal/template.json")),
    ("ieee-conference", include_str!("../../templates/ieee-conference/template.json")),
    ("elsevier-article", include_str!("../../templates/elsevier-article/template.json")),
    ("acm-conference", include_str!("../../templates/acm-conference/template.json")),
    ("springer-lncs", include_str!("../../templates/springer-lncs/template.json")),
    ("aps-physical-review", include_str!("../../templates/aps-physical-review/template.json")),
    ("beamer-presentation", include_str!("../../templates/beamer-presentation/template.json")),
    ("arabic-article", include_str!("../../templates/arabic-article/template.json")),
    ("arabic-article-two-column", include_str!("../../templates/arabic-article-two-column/template.json")),
    ("arabic-article-two-column-arabic-only", include_str!("../../templates/arabic-article-two-column-arabic-only/template.json")),
    ("arabic-thesis", include_str!("../../templates/arabic-thesis/template.json")),
    ("arabic-book", include_str!("../../templates/arabic-book/template.json")),
    ("arabic-presentation", include_str!("../../templates/arabic-presentation/template.json")),
    ("arabic-admin-order", include_str!("../../templates/arabic-admin-order/template.json")),
];

/// What a blank template starts as ("New blank template").
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlankSpec {
    /// Slides instead of a report or article.
    pub presentation: bool,
    pub paper: Paper,
    /// Chapters at the top (reports, theses) instead of sections (articles).
    pub chapters: bool,
    /// An Arabic template: right to left, Arabic labels and fields, English as the
    /// second language.
    #[serde(default)]
    pub arabic: bool,
}

/// A template to design from scratch: plain styles, a title, authors and date, a
/// simple title page and nothing else (no required chapters, no extra fields).
/// Presentations start from the beamer template, which is plain already.
pub fn blank_template(spec: BlankSpec) -> Template {
    let id = if spec.presentation { "beamer-presentation" } else { "senior-design" };
    let (_, json) = BUILTIN_TEMPLATES.iter().find(|(i, _)| *i == id).expect("built-in template exists");
    let mut v: serde_json::Value = serde_json::from_str(json).expect("built-in template parses");
    v["meta"] = serde_json::json!({ "id": "blank", "name": "Blank template", "description": "", "version": "0.1.0" });
    if !spec.presentation {
        let (width, _) = spec.paper.size_mm();
        let x = width / 2.0;
        v["page"]["paper"] = serde_json::to_value(spec.paper).expect("paper serializes");
        v["page"]["margins"] = serde_json::json!({ "topMm": 25, "bottomMm": 25, "innerMm": 25, "outerMm": 25 });
        v["page"]["lineSpacing"] = serde_json::json!({ "kind": "single" });
        v["structure"]["topLevel"] = serde_json::json!(if spec.chapters { "chapter" } else { "section" });
        v["structure"]["perChapterNumbering"] = serde_json::json!(spec.chapters);
        if !spec.chapters {
            // Sections at the top: no "Chapter 1" display heading.
            let heading = |size: u32, bold: bool, italic: bool, before: u32, after: u32| {
                serde_json::json!({ "text": { "sizePt": size, "bold": bold, "italic": italic, "align": "left" }, "layout": "inline",
                    "label": "{n}", "labelGapPt": 10, "spaceBeforePt": before, "spaceAfterPt": after })
            };
            v["headings"]["levels"] = serde_json::json!([heading(14, true, false, 18, 8), heading(12, true, false, 14, 6), heading(12, true, true, 12, 4), heading(12, false, true, 10, 4)]);
        }
        let footer = serde_json::json!({ "footer": { "center": "{page}" } });
        v["headerFooter"] = serde_json::json!({ "frontMatter": footer, "main": footer, "chapterOpening": footer });
        v["skeleton"] = serde_json::json!([]);
        v["fields"] = serde_json::json!([
            { "id": "title", "label": "Title", "kind": "text", "required": true },
            { "id": "authors", "label": "Authors", "kind": "list", "required": false },
            { "id": "date", "label": "Date", "kind": "text", "required": false },
        ]);
        let text = |text: &str, y: u32, size: u32, bold: bool| {
            serde_json::json!({ "xMm": x, "yMm": y, "anchor": "top", "content": { "type": "text", "text": text, "widthMm": 160,
                "style": { "sizePt": size, "bold": bold, "align": "center" } } })
        };
        v["titlePage"]["items"] = serde_json::json!([text("{title}", 90, 24, true), text("{authors}", 130, 14, false), text("{date}", 160, 12, false)]);
        v["frontMatter"] = if spec.chapters {
            serde_json::json!([{ "type": "titlePage" }, { "type": "tableOfContents", "title": "Contents" }])
        } else {
            serde_json::json!([{ "type": "titlePage" }])
        };
    }
    if spec.arabic {
        arabize(&mut v, spec);
    }
    serde_json::from_value(v).expect("blank template is a template")
}

/// Arabic labels, fields and direction for a blank template.
fn arabize(v: &mut serde_json::Value, spec: BlankSpec) {
    v["language"] = serde_json::json!({ "main": "arabic", "other": "english", "digits": "arabicIndic" });
    v["captions"]["figureLabel"] = "شكل".into();
    v["captions"]["tableLabel"] = "جدول".into();
    v["bibliography"]["title"] = "المراجع".into();
    let labels = [
        ("title", "العنوان"),
        ("authors", "المؤلفون"),
        ("date", "التاريخ"),
        ("subtitle", "العنوان الفرعي"),
        ("institute", "الجهة"),
        ("short_title", "عنوان مختصر"),
        ("logo", "الشعار"),
    ];
    if let Some(fields) = v["fields"].as_array_mut() {
        for f in fields {
            if let Some((_, label)) = labels.iter().find(|(id, _)| f["id"] == *id) {
                f["label"] = (*label).into();
            }
        }
    }
    if spec.presentation {
        return;
    }
    v["structure"]["appendixLabel"] = "الملحق {n}".into();
    if let Some(levels) = v["headings"]["levels"].as_array_mut() {
        for (i, level) in levels.iter_mut().enumerate() {
            if i == 0 && spec.chapters {
                level["label"] = "الفصل {n}".into();
                level["text"]["uppercase"] = false.into();
            } else {
                // Section headings start at the right, as Arabic text does.
                level["text"]["align"] = "right".into();
            }
        }
    }
    if let Some(items) = v["frontMatter"].as_array_mut() {
        for item in items {
            if item["type"] == "tableOfContents" {
                item["title"] = "المحتويات".into();
            }
        }
    }
}

/// A built-in template, for the template gallery.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    /// The publisher class it's built on, if any (e.g. "IEEEtran").
    pub class: Option<String>,
    pub kind: texloom_core::DocumentKind,
    /// The document language (the gallery lists Arabic templates together).
    pub language: texloom_core::Language,
    /// Sections at the top (articles) or chapters (reports, theses).
    pub top_level: texloom_core::TopLevel,
}

pub fn builtin_list() -> Vec<BuiltinInfo> {
    BUILTIN_TEMPLATES
        .iter()
        .map(|(id, json)| {
            let t: Template = serde_json::from_str(json).expect("built-in template parses");
            let language = t.lang().main;
            BuiltinInfo {
                id: (*id).to_string(),
                name: t.meta.name,
                description: t.meta.description,
                language,
                class: t.base.map(|b| b.class),
                kind: t.kind,
                top_level: t.structure.top_level,
            }
        })
        .collect()
}

/// Event sent when the watched template file changes.
pub const RELOADED_EVENT: &str = "template-reloaded";

#[derive(Clone)]
pub struct ActiveTemplate {
    pub template: Arc<Template>,
    /// The `template.json` this came from; `None` for the built-in or an embedded copy.
    pub path: Option<PathBuf>,
    /// Non-fatal issues found when loading.
    pub warnings: Vec<Issue>,
}

impl ActiveTemplate {
    pub fn builtin() -> Self {
        Self::builtin_by_id("senior-design").expect("default template exists")
    }

    pub fn builtin_by_id(id: &str) -> Option<Self> {
        let (_, json) = BUILTIN_TEMPLATES.iter().find(|(i, _)| *i == id)?;
        let template: Template = serde_json::from_str(json).expect("built-in template parses");
        Some(Self { template: Arc::new(template), path: None, warnings: Vec::new() })
    }

    /// Loads the template's image assets (title-page logos…) for a compile.
    pub fn load_assets(&self, assets: &[String]) -> Result<Vec<(String, Vec<u8>)>, AppError> {
        let Some(dir) = self.path.as_deref().and_then(Path::parent) else {
            return match assets.first() {
                Some(a) => Err(AppError::other(tr!(
                    "The template uses \"{a}\", but its folder is unknown. Open the template file again.",
                    "يستخدم القالب \"{a}\"، لكن مجلده غير معروف. افتح ملف القالب مرة أخرى."
                ))),
                None => Ok(Vec::new()),
            };
        };
        assets
            .iter()
            .map(|rel| {
                let bytes = std::fs::read(dir.join(rel))
                    .map_err(|e| AppError::other(tr!("Template asset {rel}: {e}", "ملف القالب {rel}: {e}")))?;
                Ok((rel.clone(), bytes))
            })
            .collect()
    }

    /// A clear message when the template needs a publisher kit that isn't imported.
    pub fn missing_kit(&self) -> Option<AppError> {
        let base = self.template.base.as_ref()?;
        let kit = base.kit.as_ref()?;
        let have: Vec<&str> = base.files.iter().map(|f| f.rsplit('/').next().unwrap_or(f)).collect();
        let missing: Vec<&str> = kit.required.iter().map(String::as_str).filter(|r| !have.contains(r)).collect();
        (!missing.is_empty()).then(|| {
            AppError::other(tr!(
                "This template needs the {} ({}). Download it from {} and import the .zip under Template → Publisher class.",
                "يحتاج هذا القالب إلى {} ({}). نزّلها من {} واستورد ملف .zip من القالب ← صنف الناشر.",
                kit.name,
                missing.join(", "),
                kit.url
            ))
        })
    }

    /// A publisher's kit files, placed next to the main file (by file name).
    pub fn load_kit_files(&self, files: &[String], engine: texloom_codegen::Engine) -> Result<Vec<(String, Vec<u8>)>, AppError> {
        let loaded = self.load_assets(files)?;
        Ok(loaded
            .into_iter()
            .map(|(rel, bytes)| {
                let name = texloom_codegen::kit_destination(&rel);
                // pdfLaTeX-only kit files get a stand-in unless pdfLaTeX itself runs.
                let bytes = texloom_codegen::kit_override(&name, engine).map(|s| s.as_bytes().to_vec()).unwrap_or(bytes);
                (name, bytes)
            })
            .collect())
    }

    pub fn info(&self) -> TemplateInfo {
        TemplateInfo {
            template: (*self.template).clone(),
            path: self.path.as_ref().map(|p| p.display().to_string()),
            warnings: self.warnings.clone(),
        }
    }
}

/// What the frontend gets about the active template.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub template: Template,
    pub path: Option<String>,
    pub warnings: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ReloadEvent {
    Reloaded { info: Box<TemplateInfo> },
    /// The file changed but is invalid; the previous version stays active.
    Invalid { path: String, issues: Vec<Issue> },
}

fn issue(path: &str, message: String) -> Issue {
    Issue { severity: Severity::Error, path: path.to_string(), message }
}

/// Validates a template, including that its asset files exist next to it.
pub fn check(template: &Template, dir: Option<&Path>) -> Vec<Issue> {
    let mut issues = template.validate();
    if let Some(base) = &template.base {
        for (field, recipe) in [("base.titleBlock", &base.title_block), ("base.endBlock", &base.end_block)] {
            for problem in recipe.as_deref().map(texloom_codegen::check_recipe).unwrap_or_default() {
                issues.push(issue(field, problem));
            }
        }
        if let Some(dir) = dir {
            for (i, f) in base.files.iter().enumerate() {
                if !dir.join(f).is_file() {
                    issues.push(issue(&format!("base.files[{i}]"), tr!("file not found: {}", "الملف غير موجود: {}", dir.join(f).display())));
                }
            }
        }
    }
    if let Some(dir) = dir {
        for (path, item) in template.placed_items() {
            if let TitleContent::Image { asset, .. } = &item.content
                && !dir.join(asset).is_file()
            {
                issues.push(issue(
                    &format!("{path}.content.asset"),
                    tr!("file not found: {}", "الملف غير موجود: {}", dir.join(asset).display()),
                ));
            }
        }
    }
    issues
}

/// Reads, parses and validates a `template.json`.
pub fn load_file(path: &Path) -> Result<ActiveTemplate, Vec<Issue>> {
    let text = std::fs::read_to_string(path).map_err(|e| vec![issue("", tr!("can't read file: {e}", "تعذّرت قراءة الملف: {e}"))])?;
    let template: Template = serde_json::from_str(&text)
        .map_err(|e| vec![issue("", tr!("not a valid template (line {}, column {}): {e}", "ليس قالبًا صالحًا (السطر {}، العمود {}): {e}", e.line(), e.column()))])?;
    from_parsed(template, Some(path.to_path_buf()))
}

/// Validates an already-parsed template (e.g. a copy embedded in a document).
pub fn from_parsed(template: Template, path: Option<PathBuf>) -> Result<ActiveTemplate, Vec<Issue>> {
    let issues = check(&template, path.as_deref().and_then(Path::parent));
    if has_errors(&issues) {
        return Err(issues);
    }
    Ok(ActiveTemplate { template: Arc::new(template), path, warnings: issues })
}

/// Saves `template` as `dest`. When the template came from another folder
/// (`previous`), its files (title-page images, publisher-kit files) are copied
/// along. Nothing is written if the template has errors.
pub fn save_to(dest: &Path, template: Template, previous: Option<&Path>) -> Result<ActiveTemplate, AppError> {
    let new_dir = dest.parent().map(Path::to_path_buf).unwrap_or_default();
    if let Some(old_dir) = previous.and_then(Path::parent)
        && std::fs::canonicalize(old_dir).ok() != std::fs::canonicalize(&new_dir).ok()
    {
        let placed = template.placed_items();
        let images = placed.iter().filter_map(|(_, item)| match &item.content {
            TitleContent::Image { asset, .. } => Some(asset),
            _ => None,
        });
        let kit = template.base.iter().flat_map(|b| &b.files);
        for asset in images.chain(kit).filter(|a| crate::files::is_safe_relative(a)) {
            let (from, to) = (old_dir.join(asset), new_dir.join(asset));
            if from.is_file() && !to.exists() {
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(from, to)?;
            }
        }
    }
    let t = from_parsed(template, Some(dest.to_path_buf()))
        .map_err(|issues| AppError::TemplateInvalid { path: dest.display().to_string(), issues })?;
    let json = serde_json::to_string_pretty(&*t.template).map_err(AppError::other)?;
    std::fs::write(dest, json + "\n")?;
    let canonical = std::fs::canonicalize(dest).unwrap_or_else(|_| dest.to_path_buf());
    Ok(ActiveTemplate { path: Some(canonical), ..t })
}

/// Copies an image into `<template folder>/assets/` and returns its asset path
/// (`assets/logo.png`), never replacing a different file of the same name.
pub fn import_asset(template_file: &Path, source: &Path) -> Result<String, AppError> {
    crate::files::import_image(template_file.parent().unwrap_or(Path::new(".")), "assets", source)
}

/// Owns the active template and the file watcher.
pub struct TemplateStore {
    active: Mutex<ActiveTemplate>,
    watcher: Mutex<Option<Debouncer<RecommendedWatcher>>>,
}

impl TemplateStore {
    pub fn new() -> Self {
        Self { active: Mutex::new(ActiveTemplate::builtin()), watcher: Mutex::new(None) }
    }

    pub fn current(&self) -> ActiveTemplate {
        self.active.lock().expect("template lock").clone()
    }

    /// Swaps in a new version of the same template (designer edits) without touching
    /// the file watcher.
    pub fn replace(&self, t: ActiveTemplate) {
        *self.active.lock().expect("template lock") = t;
    }

    /// Makes `t` active and watches its file (if any) for changes.
    pub fn set(&self, app: &AppHandle, t: ActiveTemplate) {
        let watch = t.path.clone();
        *self.active.lock().expect("template lock") = t;
        let watcher = watch.and_then(|p| match start_watcher(app.clone(), &p) {
            Ok(w) => Some(w),
            Err(e) => {
                log!("Not watching {}: {e}", p.display());
                None
            }
        });
        *self.watcher.lock().expect("watcher lock") = watcher;
    }
}

fn start_watcher(app: AppHandle, file: &Path) -> notify_debouncer_mini::notify::Result<Debouncer<RecommendedWatcher>> {
    let file = file.to_path_buf();
    let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    let assets = dir.join("assets");
    let watched_file = file.clone();
    let mut last = fingerprint(&file, &assets);
    let mut debouncer = new_debouncer(Duration::from_millis(300), move |res: DebounceEventResult| {
        let Ok(events) = res else { return };
        // Events on the assets folder itself are just it being listed; files in it count.
        let relevant =
            events.iter().any(|e| e.path == watched_file || (e.path.starts_with(&assets) && e.path != assets));
        if !relevant {
            return;
        }
        // Reading the file produces "accessed" events too; only reload on real changes,
        // detected without reading (so this doesn't loop).
        let now = fingerprint(&watched_file, &assets);
        if now == last {
            return;
        }
        last = now;
        let store = app.state::<crate::AppState>();
        let event = match load_file(&watched_file) {
            Ok(t) => {
                log!("Template reloaded: {}", watched_file.display());
                let info = t.info();
                *store.templates.active.lock().expect("template lock") = t;
                ReloadEvent::Reloaded { info: Box::new(info) }
            }
            Err(issues) => {
                log!("Template has errors ({} issues): {}", issues.len(), watched_file.display());
                ReloadEvent::Invalid { path: watched_file.display().to_string(), issues }
            }
        };
        let _ = app.emit(RELOADED_EVENT, event);
    })?;
    // Watch the folder, not the file: editors often save by replacing the file.
    debouncer.watcher().watch(&dir, RecursiveMode::Recursive)?;
    Ok(debouncer)
}

/// Modification time and size of the template file and every asset (stat only).
type Fingerprint = Vec<(PathBuf, Option<std::time::SystemTime>, u64)>;

fn fingerprint(file: &Path, assets: &Path) -> Fingerprint {
    fn stat(p: &Path, out: &mut Fingerprint) {
        match std::fs::metadata(p) {
            Ok(m) if m.is_dir() => {
                let mut entries: Vec<_> = std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).collect();
                entries.sort();
                entries.iter().for_each(|e| stat(e, out));
            }
            Ok(m) => out.push((p.to_path_buf(), m.modified().ok(), m.len())),
            Err(_) => out.push((p.to_path_buf(), None, 0)),
        }
    }
    let mut out = Vec::new();
    stat(file, &mut out);
    stat(assets, &mut out);
    out
}
