/// Like `eprintln!`, but never panics when stderr is closed (e.g. launched without a terminal).
macro_rules! log {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

mod bundle;
mod files;
mod installed;
mod library;
mod project;
mod recovery;
mod templates;
mod themes;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};
use texloom_compile::{Asset, CompileError, CompileOutput, Compiler, LatexProgram, PdfSpot, SyncTex, SystemLatex, TexError};
use texloom_core::{bib, tr};
use texloom_core::{Body, Document, Issue, Metadata, Template, DOC_FORMAT, DOC_VERSION};

use library::{Library, LibraryEntry};
use recovery::{Recovery, RecoveryInfo};
use templates::{ActiveTemplate, TemplateInfo, TemplateStore};

/// Opened on startup so there's something real to look at.
const SAMPLE_DOCUMENT: &str = include_str!("../../examples/senior-design-report.loom");
/// The example presentation (same image as the report).
const SAMPLE_PRESENTATION: &str = include_str!("../../examples/presentation.loom");
/// The Arabic examples, offered when the interface is in Arabic.
const SAMPLE_ARABIC_ARTICLE: &str = include_str!("../../examples/arabic-article.loom");
const SAMPLE_ARABIC_PRESENTATION: &str = include_str!("../../examples/arabic-presentation.loom");
/// The Arabic thesis, offered as the example project.
const SAMPLE_ARABIC_THESIS: &str = include_str!("../../examples/arabic-thesis.loom");
const SAMPLE_THESIS_FILES: &[(&str, &[u8])] = &[
    ("figures/thesis-pipeline.png", include_bytes!("../../examples/figures/thesis-pipeline.png")),
    ("figures/thesis-f1.png", include_bytes!("../../examples/figures/thesis-f1.png")),
];
/// Images the sample documents use (written to their temporary folder).
const SAMPLE_FILES: &[(&str, &[u8])] = &[
    ("figures/system-diagram.png", include_bytes!("../../examples/figures/system-diagram.png")),
    ("figures/irrigation-system.png", include_bytes!("../../examples/figures/irrigation-system.png")),
    ("figures/irrigation-water.png", include_bytes!("../../examples/figures/irrigation-water.png")),
];

pub struct AppState {
    /// The bundled Tectonic (or why it wasn't found).
    compiler: Result<Arc<Compiler>, String>,
    /// Which engine compiles: Tectonic or an installed LaTeX (Settings → Preview).
    engine: std::sync::Mutex<EngineChoice>,
    templates: TemplateStore,
    /// Where the open document's figures live.
    doc_folder: files::DocumentFolder,
    /// Journal classes of the installed TeX, looked up once.
    installed: std::sync::Mutex<Option<Vec<texloom_compile::installed::InstalledClass>>>,
    /// Beamer themes of the installed TeX, looked up once.
    beamer_themes: std::sync::Mutex<Option<texloom_compile::installed::BeamerThemes>>,
    /// SyncTeX data of the last compile, for jumps between the PDF and the editor.
    sync: std::sync::Mutex<Option<SyncState>>,
}

/// Where the last compiled document's blocks are in its `.tex` and in its PDF.
struct SyncState {
    synctex: SyncTex,
    /// Line of the `.tex` where each top-level block starts.
    block_lines: Vec<u32>,
}

impl SyncState {
    /// The top-level block whose LaTeX made what's at (`x`, `y`) on `page`.
    fn block_at(&self, page: u32, x: f32, y: f32) -> Option<usize> {
        let line = self.synctex.line_at(page, x, y)?;
        // The last block starting at or before the line (a block can be several lines).
        let i = self.block_lines.partition_point(|&l| l <= line);
        i.checked_sub(1)
    }

    /// Where block `index` starts in the PDF (blocks that print nothing: the next one).
    fn spot_of(&self, index: usize) -> Option<PdfSpot> {
        (index..self.block_lines.len()).find_map(|i| {
            let first = self.block_lines[i];
            let end = self.block_lines.get(i + 1).copied().unwrap_or(u32::MAX);
            self.synctex.spot_of(first, end)
        })
    }
}

impl AppState {
    /// The installed TeX's journal classes (looked up on first use). Blocking.
    fn installed_classes(&self) -> Vec<texloom_compile::installed::InstalledClass> {
        let mut cache = self.installed.lock().expect("installed lock");
        cache.get_or_insert_with(installed::find).clone()
    }
}

/// Errors sent to the frontend, tagged so the UI can render each kind differently.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppError {
    TectonicMissing { message: String },
    TexErrors { errors: Vec<LocatedTexError>, log: String },
    TemplateInvalid { path: String, issues: Vec<Issue> },
    Other { message: String },
}

impl AppError {
    fn other(e: impl std::fmt::Display) -> Self {
        AppError::Other { message: e.to_string() }
    }
}

/// A TeX error with the document block it came from, when known (the error list
/// jumps there).
#[derive(Debug, Serialize)]
pub struct LocatedTexError {
    #[serde(flatten)]
    error: TexError,
    /// Index of the top-level block of the document whose LaTeX failed.
    block: Option<usize>,
    /// For a project: the chapter file the block is in.
    #[serde(skip_serializing_if = "Option::is_none")]
    chapter: Option<String>,
}

impl LocatedTexError {
    /// Finds the block from the error's line in the main file.
    fn locate(error: TexError, block_lines: &[u32]) -> Self {
        let in_main = error.file.as_deref().is_none_or(|f| f.replace('\\', "/").rsplit('/').next() == Some("main.tex"));
        let block = match error.line {
            // Lines before the first block are the preamble and title matter.
            Some(line) if in_main && block_lines.first().is_some_and(|&first| line >= first) => {
                block_lines.partition_point(|&l| l <= line).checked_sub(1)
            }
            _ => None,
        };
        Self { error, block, chapter: None }
    }
}

impl From<CompileError> for AppError {
    fn from(e: CompileError) -> Self {
        match e {
            CompileError::Failed { errors, log } => {
                AppError::TexErrors { errors: errors.into_iter().map(|error| LocatedTexError { error, block: None, chapter: None }).collect(), log }
            }
            CompileError::NotFound(_) => AppError::TectonicMissing { message: e.to_string() },
            other => AppError::other(other),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::other(e)
    }
}

/// The engine that compiles documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineChoice {
    Tectonic,
    System(LatexProgram),
}

impl EngineChoice {
    fn parse(id: &str) -> Option<Self> {
        if id == "tectonic" {
            return Some(Self::Tectonic);
        }
        LatexProgram::ALL.into_iter().find(|p| p.command() == id).map(Self::System)
    }

    /// What the generated LaTeX is written for.
    fn target(self) -> texloom_codegen::Engine {
        match self {
            Self::Tectonic | Self::System(LatexProgram::XeLatex) => texloom_codegen::Engine::Xe,
            Self::System(LatexProgram::PdfLatex) => texloom_codegen::Engine::Pdf,
            Self::System(LatexProgram::LuaLatex) => texloom_codegen::Engine::Lua,
        }
    }
}

/// A ready-to-run engine.
enum Runner {
    Tectonic(Arc<Compiler>),
    System(SystemLatex),
}

impl Runner {
    fn compile(&self, tex: &str, files: &[Asset<'_>]) -> Result<CompileOutput, CompileError> {
        match self {
            Runner::Tectonic(c) => c.compile(tex, files),
            Runner::System(s) => s.compile(tex, files),
        }
    }

    /// Compiles in a build folder that is kept between compiles (a project's).
    fn compile_in(&self, dir: &Path, tex: &str, files: &[Asset<'_>]) -> Result<CompileOutput, CompileError> {
        match self {
            Runner::Tectonic(c) => c.compile_in(dir, tex, files),
            Runner::System(s) => s.compile_in(dir, tex, files),
        }
    }

    /// Identifies the engine, for keeping build folders apart.
    fn id(&self) -> &'static str {
        match self {
            Runner::Tectonic(_) => "tectonic",
            Runner::System(s) => s.program.command(),
        }
    }
}

/// An engine that can typeset Arabic: Tectonic, else an installed XeLaTeX or LuaLaTeX.
fn unicode_runner(state: &AppState) -> Option<(Runner, texloom_codegen::Engine)> {
    if let Ok(c) = &state.compiler {
        return Some((Runner::Tectonic(c.clone()), texloom_codegen::Engine::Xe));
    }
    [(LatexProgram::XeLatex, texloom_codegen::Engine::Xe), (LatexProgram::LuaLatex, texloom_codegen::Engine::Lua)]
        .into_iter()
        .find_map(|(p, e)| SystemLatex::find(p).map(|s| (Runner::System(s), e)))
}

impl AppState {
    fn engine(&self) -> EngineChoice {
        *self.engine.lock().expect("engine lock")
    }

    fn runner(&self) -> Result<Runner, AppError> {
        match self.engine() {
            EngineChoice::Tectonic => self.compiler.clone().map(Runner::Tectonic).map_err(|message| AppError::TectonicMissing { message }),
            EngineChoice::System(p) => SystemLatex::find(p).map(Runner::System).ok_or_else(|| AppError::TectonicMissing {
                message: tr!(
                    "{} isn't installed (or not on PATH). Install TeX Live, or choose Tectonic in Settings → Preview.",
                    "{} غير مثبّت (أو غير موجود في PATH). ثبّت TeX Live، أو اختر Tectonic من الإعدادات ← معاينة.",
                    p.command()
                ),
            }),
        }
    }
}

/// An engine for the settings window.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineInfo {
    id: &'static str,
    available: bool,
    /// Version or path when available.
    detail: Option<String>,
    /// Runs through latexmk (installed engines).
    latexmk: bool,
}

#[tauri::command]
async fn list_engines(state: State<'_, AppState>) -> Result<Vec<EngineInfo>, AppError> {
    let tectonic = state.compiler.as_ref().ok().map(|c| c.tectonic_path().display().to_string());
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = vec![EngineInfo { id: "tectonic", available: tectonic.is_some(), detail: tectonic, latexmk: false }];
        for p in LatexProgram::ALL {
            let found = SystemLatex::find(p);
            out.push(EngineInfo {
                id: p.command(),
                available: found.is_some(),
                latexmk: found.as_ref().is_some_and(|s| s.latexmk.is_some()),
                detail: found.map(|s| s.version().unwrap_or_else(|| s.path.display().to_string())),
            });
        }
        out
    })
    .await
    .map_err(AppError::other)
}

#[tauri::command]
fn set_engine(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let choice = EngineChoice::parse(&id).ok_or_else(|| AppError::other(tr!("Unknown engine: {id}", "محرك غير معروف: {id}")))?;
    *state.engine.lock().expect("engine lock") = choice;
    Ok(())
}

/// What the frontend holds: the editor's raw JSON plus metadata values, and the
/// template the document was saved with (when opening a file).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenedDocument {
    doc: Value,
    #[serde(default)]
    metadata: Metadata,
    /// BibTeX source.
    #[serde(default)]
    bibliography: String,
    #[serde(default, skip_deserializing)]
    template: Option<TemplateInfo>,
    /// Opened from a single-file `.loom` (document and images in one zip): saving
    /// it again writes one too.
    #[serde(default, skip_deserializing)]
    bundled: bool,
    /// Opened a chapter of a project: the project file (the editor opens the project
    /// at that chapter instead).
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    project: Option<String>,
}

/// The template fields saved inside a `.loom` file.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedTemplate {
    template: Option<Template>,
    template_path: Option<PathBuf>,
}

/// Linux: WebKitGTK checks spelling only when the app turns it on (Windows and macOS
/// webviews do it by themselves). The editor's spellcheck attribute (Settings →
/// Editor) then decides whether words are underlined.
fn enable_spell_checking(app: &tauri::App) {
    #[cfg(target_os = "linux")]
    if let Some(window) = app.get_webview_window("main") {
        let languages = spell_languages();
        let _ = window.with_webview(move |webview| {
            use webkit2gtk::{WebContextExt, WebViewExt};
            if let Some(context) = webview.inner().context() {
                context.set_spell_checking_enabled(true);
                let langs: Vec<&str> = languages.iter().map(String::as_str).collect();
                context.set_spell_checking_languages(&langs);
            }
        });
    }
    #[cfg(not(target_os = "linux"))]
    let _ = app;
}

/// Dictionaries to use: the system language ("en_GB" from LANG=en_GB.UTF-8), then
/// American English.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn spell_languages() -> Vec<String> {
    let system = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .map(|v| v.split(['.', '@']).next().unwrap_or("").to_string())
        .filter(|v| v.len() >= 2 && v != "C" && v != "POSIX");
    let mut out: Vec<String> = system.into_iter().collect();
    if !out.iter().any(|l| l == "en_US") {
        out.push("en_US".into());
    }
    out
}

/// Where Tectonic may live: next to the app binary (bundled sidecar, also where
/// `tauri dev` copies it) and, in debug builds, the source tree's `binaries/` folder.
fn tectonic_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        v.push(dir.join(format!("tectonic{}", std::env::consts::EXE_SUFFIX)));
    }
    if cfg!(debug_assertions) {
        let triple = env!("TARGET_TRIPLE");
        v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("binaries/tectonic-{triple}{}", std::env::consts::EXE_SUFFIX)));
    }
    v
}

fn to_document(doc: Value, metadata: Metadata, bibliography: Option<String>) -> Result<Document, AppError> {
    let body: Body = serde_json::from_value(doc).map_err(|e| AppError::other(tr!("Unreadable document: {e}", "تتعذّر قراءة المستند: {e}")))?;
    Ok(Document { metadata, body, bibliography: bibliography.unwrap_or_default() })
}

#[tauri::command]
fn get_template(state: State<'_, AppState>) -> TemplateInfo {
    state.templates.current().info()
}

/// Loads a `template.json`, makes it active and watches it for changes.
#[tauri::command]
fn open_template(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<TemplateInfo, AppError> {
    let path = std::fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(&path));
    let t = templates::load_file(&path)
        .map_err(|issues| AppError::TemplateInvalid { path: path.display().to_string(), issues })?;
    let info = t.info();
    state.templates.set(&app, t);
    Ok(info)
}

/// The templates that ship with Texloom (report, IEEE, Elsevier, ACM, Springer, APS).
#[tauri::command]
fn list_builtin_templates() -> Vec<templates::BuiltinInfo> {
    templates::builtin_list()
}

/// Switches to a built-in template (the default one when `id` is absent).
#[tauri::command]
fn use_builtin_template(app: AppHandle, state: State<'_, AppState>, id: Option<String>) -> Result<TemplateInfo, AppError> {
    let t = match id.as_deref() {
        Some(id) => ActiveTemplate::builtin_by_id(id).ok_or_else(|| AppError::other(tr!("No built-in template {id:?}", "لا يوجد قالب مدمج {id:?}")))?,
        None => ActiveTemplate::builtin(),
    };
    let info = t.info();
    state.templates.set(&app, t);
    Ok(info)
}

/// Journal templates for the classes of the installed TeX distribution (empty
/// without one).
#[tauri::command]
async fn list_installed_templates(app: AppHandle) -> Result<installed::InstalledList, AppError> {
    tauri::async_runtime::spawn_blocking(move || installed::list(&app.state::<AppState>().installed_classes()))
        .await
        .map_err(AppError::other)
}

/// Beamer themes found in the installed TeX (empty without one).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BeamerThemeList {
    themes: Vec<String>,
    color_themes: Vec<String>,
    font_themes: Vec<String>,
}

#[tauri::command]
async fn list_beamer_themes(app: AppHandle) -> Result<BeamerThemeList, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut cache = state.beamer_themes.lock().expect("themes lock");
        let t = cache.get_or_insert_with(texloom_compile::installed::beamer_themes).clone();
        let themes = t.themes.into_iter().filter(|n| !themes::BROKEN_THEMES.contains(&n.as_str())).collect();
        BeamerThemeList { themes, color_themes: t.color_themes, font_themes: t.font_themes }
    })
    .await
    .map_err(AppError::other)
}

/// A two-slide PDF showing a theme and colour theme, for the theme picker. Rendered
/// with the current engine and cached; a theme only the installed TeX has
/// (`installed`) is rendered with pdfLaTeX when the engine is Tectonic.
#[tauri::command]
async fn theme_preview(
    app: AppHandle,
    theme: String,
    color_theme: String,
    font_theme: String,
    installed: bool,
) -> Result<Response, AppError> {
    if ![&theme, &color_theme, &font_theme].iter().all(|n| themes::safe(n)) || theme.is_empty() {
        return Err(AppError::other(tr!("Not a theme name", "ليس اسم سمة")));
    }
    let state = app.state::<AppState>();
    let mut choice = state.engine();
    if installed && choice == EngineChoice::Tectonic && SystemLatex::find(LatexProgram::PdfLatex).is_some() {
        choice = EngineChoice::System(LatexProgram::PdfLatex);
    }
    let engine_id = match choice {
        EngineChoice::Tectonic => "tectonic",
        EngineChoice::System(p) => p.command(),
    };
    let cache = app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir().join("texloom-cache"));
    let file = themes::cache_path(&cache, engine_id, &theme, &color_theme, &font_theme);
    if let Ok(bytes) = std::fs::read(&file) {
        return Ok(Response::new(bytes));
    }
    let runner = match choice {
        EngineChoice::Tectonic => state.compiler.clone().map(Runner::Tectonic).map_err(|message| AppError::TectonicMissing { message })?,
        EngineChoice::System(p) => Runner::System(SystemLatex::find(p).ok_or_else(|| AppError::other(tr!("{} isn't installed", "{} غير مثبّت", p.command())))?),
    };
    let t = themes::preview_template(&theme, &color_theme, &font_theme);
    let (tex, files) = themes::preview_files(&t, choice.target());
    let pdf = tauri::async_runtime::spawn_blocking(move || {
        let assets: Vec<Asset> = files.iter().map(|(p, b)| Asset { path: p, bytes: b }).collect();
        runner.compile(&tex, &assets)
    })
    .await
    .map_err(AppError::other)??
    .pdf;
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
        let tmp = file.with_extension("tmp");
        if std::fs::write(&tmp, &pdf).is_ok() {
            let _ = std::fs::rename(&tmp, &file);
        }
    }
    Ok(Response::new(pdf))
}

/// Switches to the template for an installed journal class.
#[tauri::command]
async fn use_installed_template(app: AppHandle, class: String) -> Result<TemplateInfo, AppError> {
    let found = {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().installed_classes())
            .await
            .map_err(AppError::other)?
    };
    let c = found
        .iter()
        .find(|c| c.class == class)
        .ok_or_else(|| AppError::other(tr!("The class {class} isn't installed", "الصنف {class} غير مثبّت")))?;
    let t = ActiveTemplate { template: Arc::new(installed::template(c)), path: None, warnings: Vec::new() };
    let info = t.info();
    app.state::<AppState>().templates.set(&app, t);
    Ok(info)
}

/// Copies a file from a publisher's LaTeX kit (.cls, .sty, .bst, logos…) into the
/// active template's `assets/` folder; returns its path for `base.files`.
#[tauri::command]
fn import_template_file(state: State<'_, AppState>, source: String) -> Result<String, AppError> {
    let file = state
        .templates
        .current()
        .path
        .ok_or_else(|| AppError::other(tr!(
            "Save the template first (Save as…) so its files have a folder to live in.",
            "احفظ القالب أولًا (حفظ باسم…) ليكون لملفاته مجلد تُحفظ فيه."
        )))?;
    files::import_file(
        file.parent().unwrap_or(Path::new(".")),
        "assets",
        Path::new(&source),
        files::CLASS_KIT_EXTENSIONS,
        &tr!(
            "Choose a file from the publisher's LaTeX kit (.cls, .sty, .bst, images or fonts)",
            "اختر ملفًا من حزمة LaTeX الخاصة بالناشر (.cls أو .sty أو .bst أو صور أو خطوط)"
        ),
    )
}

/// Live edits from the template designer: validates the draft and, if it has no
/// errors, makes it active (assets still resolve from the template's folder).
#[tauri::command]
fn set_template(state: State<'_, AppState>, template: Template) -> Result<Vec<Issue>, AppError> {
    let current = state.templates.current();
    let t = templates::from_parsed(template, current.path.clone()).map_err(|issues| AppError::TemplateInvalid {
        path: current.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
        issues,
    })?;
    let warnings = t.warnings.clone();
    state.templates.replace(t);
    Ok(warnings)
}

/// Writes the template to `path` (copying its image assets when saving to a new
/// folder), then makes that file the active, watched template.
#[tauri::command]
fn save_template(app: AppHandle, state: State<'_, AppState>, path: String, template: Template) -> Result<TemplateInfo, AppError> {
    let current = state.templates.current();
    let t = templates::save_to(Path::new(&path), template, current.path.as_deref())?;
    let info = t.info();
    state.templates.set(&app, t);
    Ok(info)
}

/// Saves the template into My templates as a new template called `name` (Save as…),
/// copying its files along, then makes it the active, watched template.
#[tauri::command]
fn save_template_to_library(
    app: AppHandle,
    state: State<'_, AppState>,
    library: State<'_, Library>,
    name: String,
    template: Template,
) -> Result<TemplateInfo, AppError> {
    let current = state.templates.current();
    let t = library.save_new(&name, template, current.path.as_deref())?;
    let info = t.info();
    state.templates.set(&app, t);
    Ok(info)
}

/// Starts a template from scratch ("New blank template"): saved into My templates
/// as `name`, so images can be added right away, and made active.
#[tauri::command]
fn create_blank_template(
    app: AppHandle,
    state: State<'_, AppState>,
    library: State<'_, Library>,
    name: String,
    spec: templates::BlankSpec,
) -> Result<TemplateInfo, AppError> {
    let t = library.save_new(&name, templates::blank_template(spec), None)?;
    let info = t.info();
    state.templates.set(&app, t);
    Ok(info)
}

/// Copies an image into the active template's `assets/` folder for the title page.
#[tauri::command]
fn import_template_asset(state: State<'_, AppState>, source: String) -> Result<String, AppError> {
    let current = state.templates.current();
    let file = current
        .path
        .ok_or_else(|| AppError::other(tr!(
            "Save the template first (Save as…) so its images have a folder to live in.",
            "احفظ القالب أولًا (حفظ باسم…) ليكون لصوره مجلد تُحفظ فيه."
        )))?;
    templates::import_asset(&file, Path::new(&source))
}

/// Raw bytes of one of the active template's assets, for the title page canvas.
#[tauri::command]
fn read_template_asset(state: State<'_, AppState>, asset: String) -> Result<Response, AppError> {
    let current = state.templates.current();
    let mut loaded = current.load_assets(std::slice::from_ref(&asset))?;
    Ok(Response::new(loaded.remove(0).1))
}

/// Imports a publisher's kit `.zip` into the active template's `assets/` folder.
#[tauri::command]
fn import_kit_zip(state: State<'_, AppState>, source: String) -> Result<Vec<String>, AppError> {
    let file = state
        .templates
        .current()
        .path
        .ok_or_else(|| AppError::other(tr!(
            "Save the template first (Save as…) so the kit has a folder to live in.",
            "احفظ القالب أولًا (حفظ باسم…) ليكون للحزمة مجلد تُحفظ فيه."
        )))?;
    files::import_kit_zip(&file, Path::new(&source))
}

/// Font families installed on the system: from fontconfig, or on Windows (which
/// has no fontconfig) from the registry's font list.
#[tauri::command]
async fn list_system_fonts() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut families: Vec<String> = match texloom_compile::tool("fc-list").args([":", "family"]).output() {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                // "DejaVu Sans,DejaVu Sans Condensed" → the first name is the family.
                .filter_map(|l| l.split(',').next())
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty())
                .collect(),
            _ => windows_font_families(),
        };
        families.sort_by_key(|f| f.to_lowercase());
        families.dedup();
        families
    })
    .await
    .unwrap_or_default()
}

/// Installed font families with Arabic letters, for the Arabic font list (Diwani,
/// Thuluth and other fonts that come with Office, say): from fontconfig, or on
/// Windows by reading each font file's character map.
#[tauri::command]
async fn list_arabic_system_fonts() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut families: Vec<String> = match texloom_compile::tool("fc-list").args([":lang=ar", "family"]).output() {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| l.split(',').next())
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty())
                .collect(),
            _ => windows_arabic_font_families(),
        };
        families.sort_by_key(|f| f.to_lowercase());
        families.dedup();
        families
    })
    .await
    .unwrap_or_default()
}

/// Windows fonts whose files have Arabic letters, by family.
fn windows_arabic_font_families() -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let fonts_dir = std::env::var_os("WINDIR").map(|w| PathBuf::from(w).join("Fonts")).unwrap_or_default();
    let key = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";
    let mut out = Vec::new();
    for root in ["HKLM", "HKCU"] {
        let Ok(o) = texloom_compile::tool("reg").args(["query", &format!("{root}\\{key}")]).output() else { continue };
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let (Some(family), Some((_, file))) = (font_family_from_registry(line), line.split_once("REG_SZ")) else { continue };
            // The machine's fonts are named by file; a user's own by full path.
            let file = Path::new(file.trim());
            let path = if file.is_absolute() { file.to_path_buf() } else { fonts_dir.join(file) };
            if !out.contains(&family) && font_has_arabic(&path) {
                out.push(family);
            }
        }
    }
    out
}

/// The font (or a font in the collection) has the basic Arabic letters.
fn font_has_arabic(path: &Path) -> bool {
    let is_font = path.extension().and_then(|e| e.to_str()).is_some_and(|e| ["ttf", "otf", "ttc", "otc"].contains(&e.to_lowercase().as_str()));
    let Some(data) = is_font.then(|| std::fs::read(path).ok()).flatten() else { return false };
    let faces = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    (0..faces).any(|i| ttf_parser::Face::parse(&data, i).is_ok_and(|f| ['ا', 'ب', 'ع', 'ي'].iter().all(|c| f.glyph_index(*c).is_some())))
}

/// Windows fonts (the machine's and the user's own), by family: "Traditional Arabic
/// Bold (TrueType)" → "Traditional Arabic". Empty elsewhere.
fn windows_font_families() -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let key = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";
    let mut out = Vec::new();
    for root in ["HKLM", "HKCU"] {
        let Ok(o) = texloom_compile::tool("reg").args(["query", &format!("{root}\\{key}")]).output() else { continue };
        // Bitmap fonts (.fon: "8514fix", "Terminal") are for the screen; TeX can't use them.
        out.extend(String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.trim_end().to_lowercase().ends_with(".fon")).filter_map(font_family_from_registry));
    }
    out
}

/// The family in a line of `reg query` output for the font list.
fn font_family_from_registry(line: &str) -> Option<String> {
    let (name, _) = line.trim().split_once("    REG_SZ")?;
    // "Name (TrueType)", and collections: "Cambria & Cambria Math (TrueType)".
    let name = name.split(" (").next()?.split(" & ").next()?.trim();
    const STYLES: [&str; 12] = ["Bold", "Italic", "Oblique", "Light", "Semilight", "Semibold", "Black", "Regular", "Medium", "Thin", "ExtraLight", "Heavy"];
    let words: Vec<&str> = name.split_whitespace().collect();
    let keep = words.iter().rposition(|w| !STYLES.contains(w)).map_or(0, |i| i + 1);
    let family = words[..keep].join(" ");
    (!family.is_empty()).then_some(family)
}

/// An example document ("presentation", else the report), with its images in a
/// fresh unsaved-document folder.
#[tauri::command]
fn sample_document(state: State<'_, AppState>, name: Option<String>) -> Result<OpenedDocument, AppError> {
    state.doc_folder.reset_with(SAMPLE_FILES)?;
    sample(name.as_deref())
}

fn sample(name: Option<&str>) -> Result<OpenedDocument, AppError> {
    let text = match name {
        Some("presentation") => SAMPLE_PRESENTATION,
        Some("arabic-article") => SAMPLE_ARABIC_ARTICLE,
        Some("arabic-presentation") => SAMPLE_ARABIC_PRESENTATION,
        _ => SAMPLE_DOCUMENT,
    };
    serde_json::from_str(text).map_err(AppError::other)
}

/// Warnings about the document: required fields and chapters, broken
/// cross-references, citations missing from the references.
#[tauri::command]
fn check_document(
    state: State<'_, AppState>,
    doc: Value,
    metadata: Metadata,
    bibliography: Option<String>,
) -> Result<Vec<Issue>, AppError> {
    let document = to_document(doc, metadata, bibliography)?;
    let keys = bib::keys(&document.bibliography);
    Ok(state.templates.current().template.check_document(&document, keys.as_ref()))
}

/// Entries of a BibTeX source, for the citation picker and on-screen labels.
#[tauri::command]
fn parse_bibliography(source: String) -> bib::BibInfo {
    bib::parse(&source)
}

/// Reads a `.bib` file to import its entries.
#[tauri::command]
fn read_bib_file(path: String) -> Result<String, AppError> {
    if !path.to_lowercase().ends_with(".bib") {
        return Err(AppError::other(tr!("Choose a .bib file", "اختر ملف .bib")));
    }
    Ok(std::fs::read_to_string(path)?)
}

/// A new, unsaved, empty document.
#[tauri::command]
fn new_document(state: State<'_, AppState>) -> Result<OpenedDocument, AppError> {
    // A presentation starts with one empty slide.
    let content = if state.templates.current().template.kind == texloom_core::DocumentKind::Presentation {
        vec![serde_json::json!({ "type": "slide", "content": [{ "type": "slideTitle" }, { "type": "paragraph" }] })]
    } else {
        vec![serde_json::json!({ "type": "paragraph" })]
    };
    state.doc_folder.reset()?;
    Ok(OpenedDocument {
        doc: serde_json::json!({ "type": "doc", "content": content }),
        metadata: Metadata::new(),
        bibliography: String::new(),
        template: None,
        bundled: false,
        project: None,
    })
}

/// Generates LaTeX for the document and compiles it. Returns raw PDF bytes
/// (an ArrayBuffer on the JS side, avoiding a huge JSON number array).
#[tauri::command]
async fn render_pdf(state: State<'_, AppState>, doc: Value, metadata: Metadata, bibliography: Option<String>) -> Result<Response, AppError> {
    Ok(Response::new(compile_document(&state, doc, metadata, bibliography).await?))
}

/// Generates and compiles the document with the active template; returns PDF bytes.
async fn compile_document(
    state: &AppState,
    doc: Value,
    metadata: Metadata,
    bibliography: Option<String>,
) -> Result<Vec<u8>, AppError> {
    let active = state.templates.current();
    if let Some(e) = active.missing_kit() {
        return Err(e);
    }
    let document = to_document(doc, metadata, bibliography)?;
    let (runner, target) = choose_runner(state, &active.template, &document)?;
    let generated = texloom_codegen::generate_for(&active.template, &document, target);
    let mut assets = active.load_assets(&generated.assets)?;
    assets.extend(active.load_kit_files(&generated.kit_files, target)?);
    assets.extend(state.doc_folder.load(&generated.figures)?);
    assets.extend(load_library_fonts(&generated.fonts)?);
    let block_lines = generated.block_lines.clone();
    let out = tauri::async_runtime::spawn_blocking(move || {
        let support = generated.support_files();
        let files: Vec<Asset> = support.iter().chain(assets.iter()).map(|(p, b)| Asset { path: p, bytes: b }).collect();
        runner.compile(&generated.tex, &files)
    })
    .await
    .map_err(AppError::other)?
    .map_err(|e| match e {
        // Errors say which block of the document they came from.
        CompileError::Failed { errors, log } => AppError::TexErrors {
            errors: errors.into_iter().map(|e| LocatedTexError::locate(e, &block_lines)).collect(),
            log,
        },
        other => other.into(),
    })?;
    *state.sync.lock().expect("sync lock") = out.synctex.map(|synctex| SyncState { synctex, block_lines });
    Ok(out.pdf)
}

/// The engine for a document: the chosen one, except where it can't typeset it
/// (Arabic slides need LuaLaTeX; pdfLaTeX can't typeset Arabic).
fn choose_runner(state: &AppState, template: &Template, document: &Document) -> Result<(Runner, texloom_codegen::Engine), AppError> {
    let mut runner = state.runner()?;
    let mut target = state.engine().target();
    if needs_lualatex(template) && target != texloom_codegen::Engine::Lua {
        // Right-to-left slides only work with LuaLaTeX (beamer-rl): the installed one,
        // whatever engine is chosen for everything else.
        let lua = SystemLatex::find(LatexProgram::LuaLatex).ok_or_else(|| {
            AppError::other(tr!(
                "Arabic slides need LuaLaTeX, which comes with TeX Live and MiKTeX: install one of them (the built-in Tectonic can typeset Arabic documents, but not right-to-left slides).",
                "تحتاج الشرائح العربية إلى LuaLaTeX، وهو يأتي مع TeX Live وMiKTeX: ثبّت أحدهما (يستطيع Tectonic المدمج تنضيد المستندات العربية، لكن لا يستطيع تنضيد الشرائح من اليمين إلى اليسار)."
            ))
        })?;
        runner = Runner::System(lua);
        target = texloom_codegen::Engine::Lua;
    }
    if target == texloom_codegen::Engine::Pdf && texloom_codegen::language::Languages::of(template, document).need_unicode_engine() {
        // pdfLaTeX can't typeset Arabic: such documents use the built-in Tectonic
        // (or an installed XeLaTeX/LuaLaTeX) whatever engine is chosen for the rest.
        (runner, target) = unicode_runner(state).ok_or_else(|| {
            AppError::other(tr!(
                "Arabic text needs XeLaTeX or LuaLaTeX, and pdfLaTeX can't typeset it. Install TeX Live or MiKTeX, or use the built-in Tectonic.",
                "يحتاج النص العربي إلى XeLaTeX أو LuaLaTeX، ولا يستطيع pdfLaTeX تنضيده. ثبّت TeX Live أو MiKTeX، أو استخدم Tectonic المدمج."
            ))
        })?;
    }
    Ok((runner, target))
}

/// Presentations in a right-to-left language: beamer-rl, which needs LuaLaTeX.
fn needs_lualatex(t: &Template) -> bool {
    t.kind == texloom_core::DocumentKind::Presentation && t.lang().main.rtl()
}

/// The top-level block of the document behind a point of the PDF preview (in PDF
/// points from the page's top-left corner), from the last compile.
#[tauri::command]
fn pdf_to_block(state: State<'_, AppState>, page: u32, x: f32, y: f32) -> Option<usize> {
    state.sync.lock().expect("sync lock").as_ref()?.block_at(page, x, y)
}

/// Where a top-level block of the document is in the PDF of the last compile.
#[tauri::command]
fn block_to_pdf(state: State<'_, AppState>, block: usize) -> Option<PdfSpot> {
    state.sync.lock().expect("sync lock").as_ref()?.spot_of(block)
}

/// The generated `.tex` followed by the class, for the source view.
#[tauri::command]
fn to_latex(state: State<'_, AppState>, doc: Value, metadata: Metadata, bibliography: Option<String>) -> Result<String, AppError> {
    let g = texloom_codegen::generate_for(&state.templates.current().template, &to_document(doc, metadata, bibliography)?, state.engine().target());
    Ok(format!("{}\n%%%%%%%%%%%%%%%% {} %%%%%%%%%%%%%%%%\n\n{}", g.tex, g.cls_file_name(), g.cls))
}

/// A complete `.loom` file: the document with a copy of its template, so it opens
/// the same way even if the template file later moves or changes. The editor's JSON
/// is written as-is so node types this version doesn't know survive.
fn document_file(state: &AppState, doc: Value, metadata: Metadata, bibliography: Option<String>) -> Value {
    let active = state.templates.current();
    serde_json::json!({
        "format": DOC_FORMAT,
        "version": DOC_VERSION,
        "template": *active.template,
        "templatePath": active.path,
        "metadata": metadata,
        "bibliography": bibliography.unwrap_or_default(),
        "doc": doc,
    })
}

fn figure_sources(doc: &Value) -> Result<Vec<String>, AppError> {
    Ok(to_document(doc.clone(), Metadata::new(), None)?.body.figure_sources())
}

#[tauri::command]
fn save_document(
    state: State<'_, AppState>,
    path: String,
    doc: Value,
    metadata: Metadata,
    bibliography: Option<String>,
    bundled: Option<bool>,
) -> Result<(), AppError> {
    // Unless asked, a file keeps its format: a single-file .loom stays one.
    let bundled = bundled.unwrap_or_else(|| {
        let mut head = [0u8; 4];
        std::fs::File::open(&path).and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head)).is_ok() && bundle::is_bundle(&head)
    });
    if bundled {
        // One file with the images inside; the images stay where they are for now.
        let document = to_document(doc.clone(), metadata.clone(), bibliography.clone())?;
        let images: Vec<(String, Vec<u8>)> = texloom_codegen::generate(&state.templates.current().template, &document)
            .figures
            .into_iter()
            .filter_map(|rel| state.doc_folder.load(std::slice::from_ref(&rel)).ok())
            .flatten()
            .collect();
        let file = document_file(&state, doc, metadata, bibliography);
        return bundle::write(Path::new(&path), &serde_json::to_string_pretty(&file).map_err(AppError::other)?, &images);
    }
    // Saving to another folder takes the figures along.
    let new_dir = Path::new(&path).parent().map(Path::to_path_buf).unwrap_or_default();
    files::copy_relative(&figure_sources(&doc)?, &state.doc_folder.get(), &new_dir)?;
    let file = document_file(&state, doc, metadata, bibliography);
    std::fs::write(&path, serde_json::to_string_pretty(&file).map_err(AppError::other)?)?;
    state.doc_folder.set(new_dir);
    Ok(())
}

/// Opens a document and switches to its template: the template file on disk if it
/// still exists and is valid (it may have been edited since), else the saved copy.
#[tauri::command]
fn open_document(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<OpenedDocument, AppError> {
    let bytes = std::fs::read(&path)?;
    if bundle::is_bundle(&bytes) {
        // A single-file document: its images go to a folder of their own.
        let (json, images) = bundle::read(Path::new(&path))?;
        let mut opened = read_document(&app, &state, &json)?;
        let seed: Vec<(&str, &[u8])> = images.iter().map(|(p, b)| (p.as_str(), b.as_slice())).collect();
        state.doc_folder.reset_with(&seed)?;
        opened.bundled = true;
        return Ok(opened);
    }
    let text = String::from_utf8(bytes).map_err(|_| AppError::other(tr!("Not a Texloom document", "ليس مستند Texloom")))?;
    if let Some(project) = serde_json::from_str::<Value>(&text).ok().and_then(|f| project::project_of_chapter(Path::new(&path), &f)) {
        // A chapter of a project: the editor opens the project instead.
        return Ok(OpenedDocument {
            doc: Value::Null,
            metadata: Metadata::new(),
            bibliography: String::new(),
            template: None,
            bundled: false,
            project: Some(project),
        });
    }
    let opened = read_document(&app, &state, &text)?;
    state.doc_folder.set(Path::new(&path).parent().map(Path::to_path_buf).unwrap_or_default());
    Ok(opened)
}

/// Parses `.loom` text and makes its template the active one.
fn read_document(app: &AppHandle, state: &AppState, text: &str) -> Result<OpenedDocument, AppError> {
    let file: Value = serde_json::from_str(text).map_err(|e| AppError::other(tr!("Not a valid document: {e}", "ليس مستندًا صالحًا: {e}")))?;
    if file.get("format").and_then(Value::as_str) != Some(DOC_FORMAT) {
        return Err(AppError::other(tr!("Not a Texloom document", "ليس مستند Texloom")));
    }
    let mut opened: OpenedDocument =
        serde_json::from_value(file.clone()).map_err(|e| AppError::other(tr!("Not a valid document: {e}", "ليس مستندًا صالحًا: {e}")))?;
    opened.template = restore_template(app, state, &file);
    Ok(opened)
}

/// Makes the template saved in a document or project file the active one: the
/// template file on disk if it still exists and is valid, else the saved copy.
fn restore_template(app: &AppHandle, state: &AppState, file: &Value) -> Option<TemplateInfo> {
    let saved: SavedTemplate = serde_json::from_value(file.clone()).unwrap_or(SavedTemplate { template: None, template_path: None });
    let from_disk = saved.template_path.as_deref().filter(|p| p.is_file()).and_then(|p| templates::load_file(p).ok());
    let restored = from_disk.or_else(|| {
        let t = saved.template?;
        // Keep the path so image assets can still be found next to it.
        let path = saved.template_path.filter(|p| p.is_file());
        templates::from_parsed(t, path).ok()
    })?;
    let info = restored.info();
    state.templates.set(app, restored);
    Some(info)
}

/// Keeps a copy of the unsaved changes for crash recovery (see `recovery.rs`).
#[tauri::command]
fn write_recovery(
    state: State<'_, AppState>,
    recovery: State<'_, Recovery>,
    original_path: Option<String>,
    chapter: Option<String>,
    doc: Value,
    metadata: Metadata,
    bibliography: Option<String>,
) -> Result<(), AppError> {
    let figures = figure_sources(&doc)?;
    let file = document_file(&state, doc, metadata, bibliography);
    Ok(recovery.write(&file, &figures, &state.doc_folder.get(), original_path, chapter)?)
}

/// Unsaved changes left behind by a previous run (Texloom closed without saving).
#[tauri::command]
fn recovery_info(recovery: State<'_, Recovery>) -> Option<RecoveryInfo> {
    recovery.info()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoredDocument {
    #[serde(flatten)]
    document: OpenedDocument,
    /// The file the changes belong to, if it still exists.
    original_path: Option<String>,
    /// For a chapter of a project (`original_path` is the project file).
    chapter: Option<String>,
}

/// Opens the recovered changes. Figures are found next to the original file, or in
/// the recovery copy (for documents that were never saved).
#[tauri::command]
fn restore_recovery(app: AppHandle, state: State<'_, AppState>, recovery: State<'_, Recovery>) -> Result<RestoredDocument, AppError> {
    let info = recovery.info().ok_or_else(|| AppError::other(tr!("There are no unsaved changes to restore", "لا توجد تغييرات غير محفوظة لاستعادتها")))?;
    let document = read_document(&app, &state, &recovery.read_document()?)?;
    let original = info.original_path.filter(|p| Path::new(p).parent().is_some_and(Path::is_dir));
    match original.as_deref().and_then(|p| Path::new(p).parent()) {
        Some(dir) => state.doc_folder.set(dir.to_path_buf()),
        None => state.doc_folder.reset()?,
    }
    files::copy_missing(&figure_sources(&document.doc)?, &recovery.files_dir(), &state.doc_folder.get())?;
    Ok(RestoredDocument { document, original_path: original, chapter: info.chapter })
}

/// Templates in the user's library ("My templates").
#[tauri::command]
fn list_user_templates(library: State<'_, Library>) -> Vec<LibraryEntry> {
    library.list()
}

/// Imports a template file, a template package or a publisher kit into the library
/// and makes it the active template.
#[tauri::command]
fn import_template(app: AppHandle, state: State<'_, AppState>, library: State<'_, Library>, source: String) -> Result<TemplateInfo, AppError> {
    let path = library.import(Path::new(&source))?;
    match templates::load_file(&path) {
        Ok(t) => {
            let info = t.info();
            state.templates.set(&app, t);
            Ok(info)
        }
        Err(issues) => {
            if let Some(dir) = path.parent() {
                let _ = std::fs::remove_dir_all(dir);
            }
            Err(AppError::TemplateInvalid { path: source, issues })
        }
    }
}

/// Removes a template from the library (not the one in use).
#[tauri::command]
fn remove_user_template(state: State<'_, AppState>, library: State<'_, Library>, id: String) -> Result<(), AppError> {
    let in_use = state.templates.current().path.as_deref().and_then(Path::parent).and_then(Path::file_name).is_some_and(|n| n == id.as_str());
    if in_use {
        return Err(AppError::other(tr!("This template is in use; switch to another one first", "هذا القالب مستخدم حاليًا؛ انتقل إلى قالب آخر أولًا")));
    }
    library.remove(&id)
}

/// Saves the active template and its assets as a .zip others can import.
#[tauri::command]
fn export_template(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    let active = state.templates.current();
    library::export_package(&active.template, active.path.as_deref().and_then(Path::parent), Path::new(&path))
}

/// Where the user's settings live (settings.json in the app's config folder).
struct SettingsFile(PathBuf);

/// The user's settings and recent files. The frontend owns their shape; a missing
/// or unreadable file means defaults.
#[tauri::command]
fn get_settings(file: State<'_, SettingsFile>) -> Value {
    std::fs::read(&file.0).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

#[tauri::command]
fn save_settings(file: State<'_, SettingsFile>, settings: Value) -> Result<(), AppError> {
    if let Some(dir) = file.0.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = file.0.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&settings).map_err(AppError::other)?)?;
    Ok(std::fs::rename(tmp, &file.0)?)
}

#[tauri::command]
fn discard_recovery(recovery: State<'_, Recovery>) -> Result<(), AppError> {
    Ok(recovery.discard()?)
}

/// Copies an image into the document's `figures/` folder; returns its relative path.
#[tauri::command]
fn import_figure(state: State<'_, AppState>, source: String) -> Result<String, AppError> {
    files::import_image(&state.doc_folder.get(), "figures", Path::new(&source))
}

/// Raw bytes of a file in the document's folder (figure images for the editor).
#[tauri::command]
fn read_document_file(state: State<'_, AppState>, path: String) -> Result<Response, AppError> {
    let mut loaded = state.doc_folder.load(std::slice::from_ref(&path))?;
    Ok(Response::new(loaded.remove(0).1))
}

/// Compiles the document and writes the PDF to `path`.
#[tauri::command]
async fn export_pdf(state: State<'_, AppState>, path: String, doc: Value, metadata: Metadata, bibliography: Option<String>) -> Result<(), AppError> {
    let pdf = compile_document(&state, doc, metadata, bibliography).await?;
    std::fs::write(path, pdf)?;
    Ok(())
}

/// Writes the `.tex` file, the class and the template's assets next to each other,
/// ready to compile anywhere.
#[tauri::command]
fn export_tex(state: State<'_, AppState>, path: String, doc: Value, metadata: Metadata, bibliography: Option<String>) -> Result<(), AppError> {
    let active = state.templates.current();
    // Written for the chosen engine (Overleaf and most editors default to pdfLaTeX).
    let document = to_document(doc, metadata, bibliography)?;
    let mut target = state.engine().target();
    if target == texloom_codegen::Engine::Pdf && texloom_codegen::language::Languages::of(&active.template, &document).need_unicode_engine() {
        // Arabic can't be typeset by pdfLaTeX: write it for XeLaTeX instead.
        target = texloom_codegen::Engine::Xe;
    }
    let g = texloom_codegen::generate_for(&active.template, &document, target);
    let mut assets = active.load_assets(&g.assets)?;
    assets.extend(active.load_kit_files(&g.kit_files, target)?);
    assets.extend(state.doc_folder.load(&g.figures)?);
    assets.extend(load_library_fonts(&g.fonts)?);
    let path = Path::new(&path);
    std::fs::write(path, &g.tex)?;
    let dir = path.parent().unwrap_or(Path::new("."));
    for (rel, bytes) in g.support_files().into_iter().chain(assets) {
        let dest = dir.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, bytes)?;
    }
    Ok(())
}

/// Where the app's resources are (the font library, the bundled TeX packages).
static RESOURCES: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// The font library's folder: the app's resources, or the checkout's
/// `src-tauri/fonts` in development (`node scripts/fetch-fonts.mjs`).
fn font_library_dir() -> PathBuf {
    let shipped = RESOURCES.get().map(|r| r.join("fonts"));
    match shipped {
        Some(dir) if dir.join("README.md").is_file() && dir.read_dir().is_ok_and(|mut d| d.any(|e| e.is_ok_and(|e| e.path().extension().is_some_and(|x| x == "ttf")))) => dir,
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"),
    }
}

/// A document's library font files (`fonts/cairo-Regular.ttf`…), for its build folder.
fn load_library_fonts(files: &[String]) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let dir = font_library_dir();
    files
        .iter()
        .map(|rel| {
            let name = rel.strip_prefix("fonts/").unwrap_or(rel);
            std::fs::read(dir.join(name)).map(|b| (rel.clone(), b)).map_err(|_| {
                AppError::other(texloom_core::tr!(
                    "The font file {name} isn't installed with Texloom. Reinstall Texloom, or in a checkout run: node scripts/fetch-fonts.mjs",
                    "ملف الخط {name} غير مثبت مع Texloom. أعد تثبيت Texloom، أو شغّل في نسخة المصدر: node scripts/fetch-fonts.mjs"
                ))
            })
        })
        .collect()
}

/// The font library, for the designer's font picker.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryFontInfo {
    id: String,
    name: String,
    category: String,
    has_bold: bool,
    /// Whether its files are there (a development checkout may not have them).
    available: bool,
}

#[tauri::command]
fn list_library_fonts() -> Vec<LibraryFontInfo> {
    let dir = font_library_dir();
    texloom_core::fonts::library()
        .iter()
        .map(|f| LibraryFontInfo {
            id: f.id.clone(),
            name: f.name.clone(),
            category: f.category.clone(),
            has_bold: f.has_bold(),
            available: dir.join(f.regular_file()).is_file(),
        })
        .collect()
}

/// A library font's regular or bold file, for the editor (`bold` picks which).
#[tauri::command]
fn read_library_font(id: String, bold: bool) -> Result<Response, AppError> {
    let font = texloom_core::fonts::find(&id).ok_or_else(|| AppError::other(texloom_core::tr!("Unknown font {id}", "خط غير معروف {id}")))?;
    let file = if bold { font.bold_file().unwrap_or_else(|| font.regular_file()) } else { font.regular_file() };
    Ok(Response::new(std::fs::read(font_library_dir().join(file))?))
}

/// The language of the backend's messages (errors, validation problems): the
/// interface's.
#[tauri::command]
fn set_message_language(arabic: bool) {
    texloom_core::msg::set_arabic(arabic);
}

/// Writes the document as a Word file. `mathml` maps each formula's LaTeX to
/// the MathML the editor made from it, so equations stay editable in Word.
#[tauri::command]
async fn export_docx(
    state: State<'_, AppState>,
    path: String,
    doc: Value,
    metadata: Metadata,
    bibliography: Option<String>,
    mathml: HashMap<String, String>,
) -> Result<(), AppError> {
    write_docx(&state, to_document(doc, metadata, bibliography)?, path, mathml).await
}

/// Writes `document` as a Word file at `path` with the active template.
async fn write_docx(state: &AppState, document: Document, path: String, mathml: HashMap<String, String>) -> Result<(), AppError> {
    let active = state.templates.current();
    let folder = state.doc_folder.get();
    tauri::async_runtime::spawn_blocking(move || {
        // A missing image becomes a placeholder in the Word file rather than an error.
        let image = |rel: &str| files::is_safe_relative(rel).then(|| std::fs::read(folder.join(rel)).ok()).flatten();
        let asset = |rel: &str| active.load_assets(&[rel.to_string()]).ok().and_then(|mut v| v.pop()).map(|(_, b)| b);
        // Library fonts are embedded, so the file shows them where they aren't installed.
        let font = |rel: &str| load_library_fonts(&[rel.to_string()]).ok().and_then(|mut v| v.pop()).map(|(_, b)| b);
        let input = texloom_docx::Input { template: &active.template, doc: &document, image: &image, asset: &asset, mathml: &mathml, font: &font };
        let bytes = texloom_docx::write(&input).map_err(AppError::other)?;
        std::fs::write(&path, bytes)?;
        Ok(())
    })
    .await
    .map_err(AppError::other)?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    debug_assert!(ActiveTemplate::builtin().template.validate().is_empty(), "built-in template is valid");

    let compiler = Compiler::locate(&tectonic_candidates()).map(Arc::new).map_err(|e| e.to_string());
    match &compiler {
        Ok(c) => log!("Using Tectonic at {}", c.tectonic_path().display()),
        Err(e) => log!("Warning: {e}"),
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            compiler,
            engine: std::sync::Mutex::new(EngineChoice::Tectonic),
            templates: TemplateStore::new(),
            doc_folder: files::DocumentFolder::new(SAMPLE_FILES).expect("can create a temporary folder"),
            installed: std::sync::Mutex::new(None),
            beamer_themes: std::sync::Mutex::new(None),
            sync: std::sync::Mutex::new(None),
        })
        .manage(project::ProjectStore::default())
        .setup(|app| {
            enable_spell_checking(app);
            let data = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir().join("texloom"));
            if let Ok(resources) = app.path().resource_dir() {
                let _ = RESOURCES.set(resources);
            }
            // Compiling without internet: the packages the built-in templates need ship
            // with the app (scripts/build-tex-cache.mjs). Tectonic writes to its cache, so
            // they're copied into the app's data folder, which it then uses (and where
            // other packages still download to).
            if let Ok(resources) = app.path().resource_dir() {
                let shipped = resources.join("tex-cache");
                if shipped.join("texloom-cache-version").is_file() {
                    let cache = data.join("tex-cache");
                    texloom_compile::set_cache_dir(cache.clone());
                    std::thread::spawn(move || {
                        if let Err(e) = texloom_compile::fill_cache(&shipped, &cache) {
                            log!("Warning: couldn't copy the bundled TeX packages: {e}");
                        }
                    });
                }
            }
            app.manage(Recovery::new(data.join("recovery")));
            let config = app.path().app_config_dir().unwrap_or_else(|_| data.clone());
            app.manage(SettingsFile(config.join("settings.json")));
            app.manage(Library::new(data.join("templates")));
            // Start with a template file (and watch it): TEXLOOM_TEMPLATE=path/template.json
            if let Some(path) = std::env::var_os("TEXLOOM_TEMPLATE") {
                let path = std::fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(&path));
                match templates::load_file(&path) {
                    Ok(t) => app.state::<AppState>().templates.set(app.handle(), t),
                    Err(issues) => {
                        log!("TEXLOOM_TEMPLATE {} has errors:", path.display());
                        issues.iter().for_each(|i| log!("  {i}"));
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_template,
            open_template,
            use_builtin_template,
            list_installed_templates,
            use_installed_template,
            list_beamer_themes,
            theme_preview,
            list_builtin_templates,
            import_template_file,
            import_kit_zip,
            set_template,
            save_template,
            save_template_to_library,
            create_blank_template,
            pdf_to_block,
            block_to_pdf,
            list_system_fonts,
            list_arabic_system_fonts,
            import_template_asset,
            import_figure,
            read_document_file,
            export_pdf,
            read_template_asset,
            sample_document,
            check_document,
            parse_bibliography,
            read_bib_file,
            new_document,
            render_pdf,
            to_latex,
            save_document,
            open_document,
            write_recovery,
            recovery_info,
            restore_recovery,
            discard_recovery,
            get_settings,
            list_engines,
            set_engine,
            list_user_templates,
            import_template,
            remove_user_template,
            export_template,
            save_settings,
            export_tex,
            export_docx,
            set_message_language,
            list_library_fonts,
            read_library_font,
            project::open_project,
            project::close_project,
            project::read_chapter,
            project::save_chapter,
            project::save_project,
            project::new_chapter,
            project::project_index,
            project::render_project,
            project::export_project_pdf,
            project::export_project_docx,
            project::project_pdf_to_block,
            project::project_block_to_pdf,
            project::export_project_tex,
            project::split_into_project,
            project::sample_project,
            project::join_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_arabic_fonts_are_found() {
        // A library font (fetched by scripts/fetch-fonts.mjs, when it's there).
        let ruqaa = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/aref-ruqaa-Regular.ttf");
        if ruqaa.exists() {
            assert!(font_has_arabic(&ruqaa));
        }
        assert!(!font_has_arabic(Path::new("missing.ttf")) && !font_has_arabic(Path::new("readme.txt")));
        // Windows: Arial and Tahoma have Arabic; Consolas doesn't.
        if cfg!(windows) {
            let fonts = windows_arabic_font_families();
            assert!(fonts.iter().any(|f| f == "Arial") && fonts.iter().any(|f| f == "Tahoma"), "{fonts:?}");
            assert!(!fonts.iter().any(|f| f == "Consolas"), "{fonts:?}");
            // Screen bitmap fonts (.fon), which TeX can't use, aren't offered at all.
            let all = windows_font_families();
            assert!(all.iter().any(|f| f == "Consolas") && !all.iter().any(|f| f == "8514fix" || f == "Terminal"), "{all:?}");
            // Office's Diwani, where it's installed.
            if Path::new(r"C:\Windows\Fonts\DIWANLTR.TTF").exists() {
                assert!(fonts.iter().any(|f| f == "Diwani Letter"), "{fonts:?}");
            }
            eprintln!("Arabic fonts: {fonts:?}");
        }
    }

    #[test]
    fn windows_font_names() {
        let f = |l: &str| font_family_from_registry(l);
        assert_eq!(f("    Traditional Arabic Bold (TrueType)    REG_SZ    tradbdo.ttf").as_deref(), Some("Traditional Arabic"));
        assert_eq!(f("    Sakkal Majalla (TrueType)    REG_SZ    majalla.ttf").as_deref(), Some("Sakkal Majalla"));
        assert_eq!(f("    Cambria & Cambria Math (TrueType)    REG_SZ    cambria.ttc").as_deref(), Some("Cambria"));
        assert_eq!(f("    Segoe UI Semibold Italic (TrueType)    REG_SZ    seguisbi.ttf").as_deref(), Some("Segoe UI"));
        assert_eq!(f("HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft"), None);
    }

    #[test]
    fn errors_and_pdf_spots_find_their_block() {
        let lines = [10, 12, 20];
        let err = |file: Option<&str>, line: Option<u32>| TexError { file: file.map(str::to_string), line, message: "x".into() };
        let block = |e| LocatedTexError::locate(e, &lines).block;
        assert_eq!(block(err(Some("main.tex"), Some(13))), Some(1));
        assert_eq!(block(err(Some("./main.tex"), Some(20))), Some(2));
        assert_eq!(block(err(None, Some(99))), Some(2));
        assert_eq!(block(err(Some("main.tex"), Some(4))), None, "the preamble is no block");
        assert_eq!(block(err(Some("senior-design.cls"), Some(13))), None, "another file");
        assert_eq!(block(err(Some("main.tex"), None)), None);

        // SyncTeX: a box made by line 13 belongs to block 1; block 0 (lines 10-11)
        // printed nothing, so showing it shows the next block that did.
        let synctex = SyncTex::parse("Input:1:main.tex\n{1\n(1,13:100,200:300,10,2\n)\n}1\n", "main.tex");
        let state = SyncState { synctex, block_lines: lines.to_vec() };
        assert_eq!(state.block_at(1, 0.01, 0.0), Some(1));
        assert_eq!(state.spot_of(0).map(|s| s.page), Some(1));
        assert_eq!(state.spot_of(2), None);
    }

    #[test]
    fn blank_templates_are_valid_and_plain() {
        use texloom_core::{FrontMatterItem, Paper, TopLevel};
        for presentation in [false, true] {
            for chapters in [false, true] {
                for (paper, arabic) in [(Paper::A4, false), (Paper::Letter, false), (Paper::A4, true)] {
                    let t = templates::blank_template(templates::BlankSpec { presentation, paper, chapters, arabic });
                    assert_eq!(t.lang().main.rtl(), arabic);
                    let issues = t.validate();
                    assert!(!texloom_core::has_errors(&issues), "{presentation} {chapters} {paper:?}: {issues:?}");
                    assert!(t.skeleton.is_empty(), "no required chapters");
                    if presentation {
                        assert_eq!(t.kind, texloom_core::DocumentKind::Presentation);
                        continue;
                    }
                    assert_eq!(t.page.paper, paper);
                    assert_eq!(t.structure.top_level, if chapters { TopLevel::Chapter } else { TopLevel::Section });
                    assert_eq!(t.fields.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), ["title", "authors", "date"]);
                    assert_eq!(t.front_matter.iter().any(|f| matches!(f, FrontMatterItem::TableOfContents { .. })), chapters);
                    // The title page is centred on the paper.
                    assert_eq!(t.title_page.items[0].x_mm, paper.size_mm().0 / 2.0);
                    // The class generates and the sample document still works with it.
                    assert!(texloom_codegen::template_to_cls(&t).contains("\\ProvidesClass"));
                }
            }
        }
    }

    /// Blank templates compile with the example report's text (Tectonic, and pdfLaTeX
    /// when installed). Run with TEXLOOM_TECTONIC=… cargo test -p texloom blank -- --ignored
    #[test]
    #[ignore]
    fn blank_templates_compile() {
        use texloom_core::{DocumentFile, Paper};
        let file: DocumentFile = serde_json::from_str(include_str!("../../examples/senior-design-report.loom")).unwrap();
        let mut doc = file.document;
        doc.metadata.retain(|k, _| k == "title");
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
        for chapters in [false, true] {
            let t = templates::blank_template(templates::BlankSpec { presentation: false, paper: Paper::Letter, chapters, arabic: false });
            for engine in [texloom_codegen::Engine::Xe, texloom_codegen::Engine::Pdf] {
                let g = texloom_codegen::generate_for(&t, &doc, engine);
                let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), std::fs::read(examples.join(f)).unwrap())).collect();
                let support = g.support_files();
                let files: Vec<Asset> = support.iter().chain(figures.iter()).map(|(p, b)| Asset { path: p, bytes: b }).collect();
                let out = match engine {
                    texloom_codegen::Engine::Pdf => match SystemLatex::find(LatexProgram::PdfLatex) {
                        Some(latex) => latex.compile(&g.tex, &files),
                        None => continue,
                    },
                    _ => Compiler::locate(&[]).expect("tectonic").compile(&g.tex, &files),
                };
                assert!(out.unwrap_or_else(|e| panic!("chapters {chapters}, {engine:?}: {e:?}")).pdf.starts_with(b"%PDF"));
            }
        }
    }

    #[test]
    fn all_builtin_templates_are_valid() {
        for info in templates::builtin_list() {
            let active = ActiveTemplate::builtin_by_id(&info.id).unwrap();
            let errors: Vec<_> = active.template.validate().into_iter().filter(|i| i.severity == texloom_core::Severity::Error).collect();
            assert!(errors.is_empty(), "{}: {errors:?}", info.id);
            assert!(active.missing_kit().is_none(), "{}: built-in templates must work without downloads", info.id);
        }
        assert!(templates::builtin_list().iter().any(|t| t.class.as_deref() == Some("IEEEtran")));
    }

    #[test]
    fn builtin_template_and_sample_load() {
        let template = ActiveTemplate::builtin().template;
        assert!(template.validate().is_empty());
        let sample = sample(None).unwrap();
        let doc = to_document(sample.doc, sample.metadata, Some(sample.bibliography)).unwrap();
        assert!(template.check_metadata(&doc.metadata).is_empty());
        assert!(texloom_codegen::generate(&template, &doc).tex.contains("\\chapter{Introduction}"));
    }

    #[test]
    fn sample_document_figures_are_available() {
        let folder = files::DocumentFolder::new(SAMPLE_FILES).unwrap();
        // Every example's images are there, the Arabic ones' too.
        for name in [None, Some("presentation"), Some("arabic-article"), Some("arabic-presentation")] {
            let sample = sample(name).unwrap();
            let doc = to_document(sample.doc, sample.metadata, Some(sample.bibliography)).unwrap();
            let figures = doc.body.figure_sources();
            assert!(!figures.is_empty(), "{name:?}");
            let loaded = folder.load(&figures).unwrap_or_else(|e| panic!("{name:?}: {e:?}"));
            assert!(loaded.iter().all(|(_, bytes)| bytes.starts_with(b"\x89PNG")), "{name:?}");
        }
        assert!(folder.load(&["../etc/passwd".into()]).is_err());
    }

    #[test]
    fn template_file_errors_are_reported() {
        let dir = std::env::temp_dir().join(format!("texloom-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("template.json");

        std::fs::write(&file, "{ not json").unwrap();
        let issues = templates::load_file(&file).err().unwrap();
        assert!(issues[0].message.contains("line 1"), "{issues:?}");

        let mut t: Value = serde_json::from_str(templates::BUILTIN_TEMPLATE).unwrap();
        t["titlePage"]["items"][0]["content"] =
            serde_json::json!({"type": "image", "asset": "assets/logo.png", "widthMm": 30});
        std::fs::write(&file, t.to_string()).unwrap();
        let issues = templates::load_file(&file).err().unwrap();
        assert!(issues.iter().any(|i| i.message.contains("file not found")), "{issues:?}");

        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("assets/logo.png"), b"png").unwrap();
        let active = templates::load_file(&file).unwrap();
        let assets = active.load_assets(&["assets/logo.png".into()]).unwrap();
        assert_eq!(assets[0].1, b"png");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn importing_assets_never_overwrites_different_files() {
        let root = std::env::temp_dir().join(format!("texloom-import-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let template = root.join("template.json");
        let (a, b) = (root.join("My Logo.png"), root.join("other.png"));
        std::fs::write(&a, b"one").unwrap();
        std::fs::write(&b, b"two").unwrap();

        assert_eq!(templates::import_asset(&template, &a).unwrap(), "assets/My-Logo.png");
        // Same content again: reuses the file.
        assert_eq!(templates::import_asset(&template, &a).unwrap(), "assets/My-Logo.png");
        // Different content, same name: gets a new name.
        std::fs::write(&a, b"changed").unwrap();
        assert_eq!(templates::import_asset(&template, &a).unwrap(), "assets/My-Logo-2.png");
        assert_eq!(std::fs::read(root.join("assets/My-Logo.png")).unwrap(), b"one");
        assert!(templates::import_asset(&template, &root.join("x.svg")).is_err());
        let _ = b;
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn save_as_copies_assets_and_refuses_invalid_templates() {
        let root = std::env::temp_dir().join(format!("texloom-save-{}", std::process::id()));
        let (old_dir, new_dir) = (root.join("old"), root.join("new"));
        std::fs::create_dir_all(old_dir.join("assets")).unwrap();
        std::fs::create_dir_all(&new_dir).unwrap();
        std::fs::write(old_dir.join("assets/logo.png"), b"png").unwrap();

        let mut t = (*ActiveTemplate::builtin().template).clone();
        t.title_page.items[0].content =
            texloom_core::TitleContent::Image { asset: "assets/logo.png".into(), width_mm: 30.0 };
        let old_file = old_dir.join("template.json");
        let dest = new_dir.join("copy.json");

        let saved = templates::save_to(&dest, t.clone(), Some(&old_file)).unwrap();
        assert_eq!(std::fs::read(new_dir.join("assets/logo.png")).unwrap(), b"png");
        let reread: Template = serde_json::from_str(&std::fs::read_to_string(&dest).unwrap()).unwrap();
        assert_eq!(reread, t);
        assert!(saved.path.unwrap().ends_with("copy.json"));

        let mut bad = t;
        bad.fonts.base_size_pt = 13;
        let bad_dest = new_dir.join("bad.json");
        assert!(matches!(templates::save_to(&bad_dest, bad, None), Err(AppError::TemplateInvalid { .. })));
        assert!(!bad_dest.exists(), "invalid templates are not written");
        std::fs::remove_dir_all(root).unwrap();
    }
}
