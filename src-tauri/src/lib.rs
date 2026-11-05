/// Like `eprintln!`, but never panics when stderr is closed (e.g. launched without a terminal).
macro_rules! log {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

mod bib;
mod files;
mod templates;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};
use texloom_compile::{Asset, CompileError, Compiler, TexError};
use texloom_core::{Body, Document, Issue, Metadata, Template, DOC_FORMAT, DOC_VERSION};

use templates::{ActiveTemplate, TemplateInfo, TemplateStore};

/// Opened on startup so there's something real to look at.
const SAMPLE_DOCUMENT: &str = include_str!("../../examples/senior-design-report.loom");
/// Images the sample document uses (written to its temporary folder at startup).
const SAMPLE_FILES: &[(&str, &[u8])] =
    &[("figures/system-diagram.png", include_bytes!("../../examples/figures/system-diagram.png"))];

pub struct AppState {
    compiler: Result<Arc<Compiler>, String>,
    templates: TemplateStore,
    /// Where the open document's figures live.
    doc_folder: files::DocumentFolder,
}

/// Errors sent to the frontend, tagged so the UI can render each kind differently.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppError {
    TectonicMissing { message: String },
    TexErrors { errors: Vec<TexError>, log: String },
    TemplateInvalid { path: String, issues: Vec<Issue> },
    Other { message: String },
}

impl AppError {
    fn other(e: impl std::fmt::Display) -> Self {
        AppError::Other { message: e.to_string() }
    }
}

impl From<CompileError> for AppError {
    fn from(e: CompileError) -> Self {
        match e {
            CompileError::Failed { errors, log } => AppError::TexErrors { errors, log },
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
}

/// The template fields saved inside a `.loom` file.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedTemplate {
    template: Option<Template>,
    template_path: Option<PathBuf>,
}

/// Where Tectonic may live: next to the app binary (bundled sidecar, also where
/// `tauri dev` copies it) and, in debug builds, the source tree's `binaries/` folder.
fn tectonic_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        v.push(dir.join("tectonic"));
    }
    if cfg!(debug_assertions) {
        let triple = env!("TARGET_TRIPLE");
        v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("binaries/tectonic-{triple}")));
    }
    v
}

fn to_document(doc: Value, metadata: Metadata, bibliography: Option<String>) -> Result<Document, AppError> {
    let body: Body = serde_json::from_value(doc).map_err(|e| AppError::other(format!("Unreadable document: {e}")))?;
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
        Some(id) => ActiveTemplate::builtin_by_id(id).ok_or_else(|| AppError::other(format!("No built-in template {id:?}")))?,
        None => ActiveTemplate::builtin(),
    };
    let info = t.info();
    state.templates.set(&app, t);
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
        .ok_or_else(|| AppError::other("Save the template first (Save as…) so its files have a folder to live in."))?;
    files::import_file(
        file.parent().unwrap_or(Path::new(".")),
        "assets",
        Path::new(&source),
        files::CLASS_KIT_EXTENSIONS,
        "Choose a file from the publisher's LaTeX kit (.cls, .sty, .bst, images or fonts)",
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

/// Copies an image into the active template's `assets/` folder for the title page.
#[tauri::command]
fn import_template_asset(state: State<'_, AppState>, source: String) -> Result<String, AppError> {
    let current = state.templates.current();
    let file = current
        .path
        .ok_or_else(|| AppError::other("Save the template first (Save as…) so its images have a folder to live in."))?;
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
        .ok_or_else(|| AppError::other("Save the template first (Save as…) so the kit has a folder to live in."))?;
    files::import_kit_zip(&file, Path::new(&source))
}

/// Font families installed on the system (empty if fontconfig isn't available).
#[tauri::command]
async fn list_system_fonts() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(|| {
        let Ok(out) = std::process::Command::new("fc-list").args([":", "family"]).output() else {
            return Vec::new();
        };
        let mut families: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            // "DejaVu Sans,DejaVu Sans Condensed" → the first name is the family.
            .filter_map(|l| l.split(',').next())
            .map(|f| f.trim().to_string())
            .filter(|f| !f.is_empty())
            .collect();
        families.sort_by_key(|f| f.to_lowercase());
        families.dedup();
        families
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
fn sample_document() -> Result<OpenedDocument, AppError> {
    serde_json::from_str(SAMPLE_DOCUMENT).map_err(AppError::other)
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
        return Err(AppError::other("Choose a .bib file"));
    }
    Ok(std::fs::read_to_string(path)?)
}

/// A new, unsaved document started from the template's chapter skeleton.
#[tauri::command]
fn new_document(state: State<'_, AppState>) -> Result<OpenedDocument, AppError> {
    let template = state.templates.current().template;
    let mut content: Vec<Value> = Vec::new();
    for (i, sec) in template.skeleton.iter().enumerate() {
        content.push(serde_json::json!({
            "type": "heading",
            "attrs": { "level": 1, "id": format!("h-new-{i}") },
            "content": [{ "type": "text", "text": sec.title }],
        }));
        content.push(serde_json::json!({ "type": "paragraph" }));
    }
    if content.is_empty() {
        content.push(serde_json::json!({ "type": "paragraph" }));
    }
    state.doc_folder.reset()?;
    Ok(OpenedDocument {
        doc: serde_json::json!({ "type": "doc", "content": content }),
        metadata: Metadata::new(),
        bibliography: String::new(),
        template: None,
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
    let compiler = state.compiler.clone().map_err(|message| AppError::TectonicMissing { message })?;
    let active = state.templates.current();
    if let Some(e) = active.missing_kit() {
        return Err(e);
    }
    let generated = texloom_codegen::generate(&active.template, &to_document(doc, metadata, bibliography)?);
    let mut assets = active.load_assets(&generated.assets)?;
    assets.extend(active.load_kit_files(&generated.kit_files)?);
    assets.extend(state.doc_folder.load(&generated.figures)?);
    let out = tauri::async_runtime::spawn_blocking(move || {
        let support = generated.support_files();
        let files: Vec<Asset> = support.iter().chain(assets.iter()).map(|(p, b)| Asset { path: p, bytes: b }).collect();
        compiler.compile(&generated.tex, &files)
    })
    .await
    .map_err(AppError::other)??;
    Ok(out.pdf)
}

/// The generated `.tex` followed by the class, for the source view.
#[tauri::command]
fn to_latex(state: State<'_, AppState>, doc: Value, metadata: Metadata, bibliography: Option<String>) -> Result<String, AppError> {
    let g = texloom_codegen::generate(&state.templates.current().template, &to_document(doc, metadata, bibliography)?);
    Ok(format!("{}\n%%%%%%%%%%%%%%%% {} %%%%%%%%%%%%%%%%\n\n{}", g.tex, g.cls_file_name(), g.cls))
}

/// Saves the document with a copy of its template, so it opens the same way
/// even if the template file later moves or changes.
#[tauri::command]
fn save_document(state: State<'_, AppState>, path: String, doc: Value, metadata: Metadata, bibliography: Option<String>) -> Result<(), AppError> {
    let active = state.templates.current();
    // Saving to another folder takes the figures along.
    let new_dir = Path::new(&path).parent().map(Path::to_path_buf).unwrap_or_default();
    let figures = to_document(doc.clone(), Metadata::new(), None)?.body.figure_sources();
    files::copy_relative(&figures, &state.doc_folder.get(), &new_dir)?;
    // The editor's JSON is written as-is so node types this version doesn't know survive.
    let file = serde_json::json!({
        "format": DOC_FORMAT,
        "version": DOC_VERSION,
        "template": *active.template,
        "templatePath": active.path,
        "metadata": metadata,
        "bibliography": bibliography.unwrap_or_default(),
        "doc": doc,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&file).map_err(AppError::other)?)?;
    state.doc_folder.set(new_dir);
    Ok(())
}

/// Opens a document and switches to its template: the template file on disk if it
/// still exists and is valid (it may have been edited since), else the saved copy.
#[tauri::command]
fn open_document(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<OpenedDocument, AppError> {
    let text = std::fs::read_to_string(&path)?;
    let file: Value = serde_json::from_str(&text).map_err(|e| AppError::other(format!("Not a valid document: {e}")))?;
    if file.get("format").and_then(Value::as_str) != Some(DOC_FORMAT) {
        return Err(AppError::other("Not a Texloom document"));
    }
    let saved: SavedTemplate = serde_json::from_value(file.clone()).unwrap_or(SavedTemplate { template: None, template_path: None });
    let mut opened: OpenedDocument =
        serde_json::from_value(file).map_err(|e| AppError::other(format!("Not a valid document: {e}")))?;

    let from_disk = saved.template_path.as_deref().filter(|p| p.is_file()).and_then(|p| templates::load_file(p).ok());
    let restored = from_disk.or_else(|| {
        let t = saved.template?;
        // Keep the path so image assets can still be found next to it.
        let path = saved.template_path.filter(|p| p.is_file());
        templates::from_parsed(t, path).ok()
    });
    if let Some(t) = restored {
        opened.template = Some(t.info());
        state.templates.set(&app, t);
    }
    state.doc_folder.set(Path::new(&path).parent().map(Path::to_path_buf).unwrap_or_default());
    Ok(opened)
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
    let g = texloom_codegen::generate(&active.template, &to_document(doc, metadata, bibliography)?);
    let mut assets = active.load_assets(&g.assets)?;
    assets.extend(active.load_kit_files(&g.kit_files)?);
    assets.extend(state.doc_folder.load(&g.figures)?);
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
            templates: TemplateStore::new(),
            doc_folder: files::DocumentFolder::new(SAMPLE_FILES).expect("can create a temporary folder"),
        })
        .setup(|app| {
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
            list_builtin_templates,
            import_template_file,
            import_kit_zip,
            set_template,
            save_template,
            list_system_fonts,
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
            export_tex
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_templates_are_valid() {
        for info in templates::builtin_list() {
            let active = ActiveTemplate::builtin_by_id(&info.id).unwrap();
            let errors: Vec<_> = active.template.validate().into_iter().filter(|i| i.severity == texloom_core::Severity::Error).collect();
            assert!(errors.is_empty(), "{}: {errors:?}", info.id);
            // Only templates that need a downloaded kit may report it missing.
            assert_eq!(active.missing_kit().is_some(), info.id == "ieee-access", "{}", info.id);
        }
        assert!(templates::builtin_list().iter().any(|t| t.class.as_deref() == Some("IEEEtran")));
    }

    #[test]
    fn builtin_template_and_sample_load() {
        let template = ActiveTemplate::builtin().template;
        assert!(template.validate().is_empty());
        let sample = sample_document().unwrap();
        let doc = to_document(sample.doc, sample.metadata, Some(sample.bibliography)).unwrap();
        assert!(template.check_metadata(&doc.metadata).is_empty());
        assert!(texloom_codegen::generate(&template, &doc).tex.contains("\\chapter{Introduction}"));
    }

    #[test]
    fn sample_document_figures_are_available() {
        let folder = files::DocumentFolder::new(SAMPLE_FILES).unwrap();
        let sample = sample_document().unwrap();
        let doc = to_document(sample.doc, sample.metadata, Some(sample.bibliography)).unwrap();
        let figures = doc.body.figure_sources();
        assert!(!figures.is_empty());
        let loaded = folder.load(&figures).unwrap();
        assert!(loaded[0].1.starts_with(b"\x89PNG"));
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
