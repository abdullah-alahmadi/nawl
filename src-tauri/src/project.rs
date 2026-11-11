//! Projects: a book or thesis written as one `.nawl` file per chapter, listed in a
//! `.nawlproj` file with what the whole book shares (template, metadata,
//! bibliography).
//!
//! The backend holds the open project: its file and every chapter's body as on disk
//! (or as last saved). The editor has one chapter open and sends its live content
//! with each compile, index or export, so unsaved edits count.
//!
//! Figures are relative to the project folder (`figures/…`), as a single document's
//! are to its own folder: the document folder is the project folder while a project
//! is open.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};
use nawl_compile::{Asset, CompileError, PdfSpot, SyncTex};
use nawl_core::{tr, Body, Issue, Metadata, Project, ProjectEntry, ProjectFile, ProjectIndex, Severity, DOC_FORMAT, DOC_VERSION};

use crate::templates::TemplateInfo;
use crate::{files, AppError, AppState, LocatedTexError};

/// The open project, if any.
#[derive(Default)]
pub struct ProjectStore(Mutex<Option<OpenProject>>);

struct OpenProject {
    /// The `.nawlproj` file.
    path: PathBuf,
    /// The project file's JSON as read, so fields this version doesn't know survive
    /// saving.
    raw: Value,
    file: ProjectFile,
    /// Each chapter's body JSON (its `"doc"`), as on disk or as last saved. The
    /// editor's JSON is kept as-is (unknown node types survive).
    chapters: HashMap<String, Value>,
    /// What the last full compile was made from (see [`signature`]): a one-chapter
    /// compile is only right while it's the same.
    last_full: Option<u64>,
    /// SyncTeX data of the last compile.
    sync: Option<ProjectSync>,
}

impl OpenProject {
    fn folder(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    /// The project with its chapters, the open chapter (`open`: file and live body
    /// JSON) replacing its saved version, and live metadata and bibliography.
    fn project(&self, open: Option<(&str, &Value)>, metadata: Metadata, bibliography: String) -> Result<Project, AppError> {
        let mut file = self.file.clone();
        file.metadata = metadata;
        file.bibliography = bibliography;
        let empty = serde_json::json!({ "type": "doc", "content": [] });
        Project::load(&file, |name| {
            let doc = match open {
                Some((f, doc)) if f == name => doc,
                _ => self.chapters.get(name).unwrap_or(&empty),
            };
            serde_json::from_value::<Body>(doc.clone())
        })
        .map_err(|(name, e)| AppError::other(tr!("Chapter \"{name}\" can't be read: {e}", "يتعذّر قراءة الفصل \"{name}\": {e}")))
    }
}

/// Where the last compile's blocks are in its chapter files and in its PDF.
struct ProjectSync {
    synctex: SyncTex,
    /// Chapter files in book order (SyncTeX file `i + 1` is chapter `i`).
    files: Vec<String>,
    /// Line of each chapter's `.tex` where each of its top-level blocks starts.
    lines: Vec<Vec<u32>>,
}

/// What the editor gets when a project opens.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedProject {
    path: String,
    metadata: Metadata,
    bibliography: String,
    chapters: Vec<ProjectEntry>,
    template: Option<TemplateInfo>,
    index: ProjectIndex,
    /// Chapters that couldn't be read, and other problems.
    issues: Vec<Issue>,
}

/// Reads a chapter file's body JSON.
fn read_chapter_file(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut file: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if file.get("format").and_then(Value::as_str) != Some(DOC_FORMAT) {
        return Err(tr!("not a Nawl document", "ليس مستند نول"));
    }
    let doc = file.get_mut("doc").map(Value::take).ok_or_else(|| tr!("no document body", "لا يوجد متن للمستند"))?;
    serde_json::from_value::<Body>(doc.clone()).map_err(|e| e.to_string())?;
    Ok(doc)
}

/// Opens a project: its template becomes the active one, its folder the document
/// folder. Chapters that can't be read are reported, and count as empty.
#[tauri::command]
pub fn open_project(app: AppHandle, state: State<'_, AppState>, store: State<'_, ProjectStore>, path: String) -> Result<OpenedProject, AppError> {
    let path = PathBuf::from(path);
    let text = std::fs::read_to_string(&path)?;
    let raw: Value = serde_json::from_str(&text).map_err(|e| AppError::other(tr!("Not a valid project: {e}", "ليس مشروعًا صالحًا: {e}")))?;
    let file: ProjectFile = serde_json::from_value(raw.clone()).map_err(|e| AppError::other(tr!("Not a valid project: {e}", "ليس مشروعًا صالحًا: {e}")))?;
    let mut issues = file.validate();
    if let Some(e) = issues.iter().find(|i| i.severity == Severity::Error && (i.path == "format" || i.path == "version")) {
        return Err(AppError::other(&e.message));
    }
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut chapters = HashMap::new();
    for name in file.chapter_files() {
        if !nawl_core::is_safe_chapter_path(name) {
            continue;
        }
        match read_chapter_file(&folder.join(name)) {
            Ok(doc) => {
                chapters.insert(name.to_string(), doc);
            }
            Err(e) => issues.push(Issue {
                severity: Severity::Error,
                path: format!("chapters.{name}"),
                message: tr!("Chapter \"{name}\" can't be read: {e}", "يتعذّر قراءة الفصل \"{name}\": {e}"),
            }),
        }
    }
    let template = crate::restore_template(&app, &state, &raw);
    state.doc_folder.set(folder);
    let open = OpenProject { path: path.clone(), raw, file, chapters, last_full: None, sync: None };
    let project = open.project(None, open.file.metadata.clone(), open.file.bibliography.clone())?;
    issues.extend(project.validate());
    let opened = OpenedProject {
        path: path.display().to_string(),
        metadata: open.file.metadata.clone(),
        bibliography: open.file.bibliography.clone(),
        chapters: open.file.chapters.clone(),
        template,
        index: project.index(),
        issues,
    };
    *store.0.lock().expect("project lock") = Some(open);
    Ok(opened)
}

/// Closes the open project (a single document is opened or created instead).
#[tauri::command]
pub fn close_project(store: State<'_, ProjectStore>) {
    *store.0.lock().expect("project lock") = None;
}

fn with_project<T>(store: &ProjectStore, f: impl FnOnce(&mut OpenProject) -> Result<T, AppError>) -> Result<T, AppError> {
    let mut guard = store.0.lock().expect("project lock");
    let open = guard.as_mut().ok_or_else(|| AppError::other(tr!("No project is open", "لا يوجد مشروع مفتوح")))?;
    f(open)
}

/// A chapter's body JSON (as last read or saved), to open it in the editor.
#[tauri::command]
pub fn read_chapter(store: State<'_, ProjectStore>, file: String) -> Result<Value, AppError> {
    with_project(&store, |p| {
        p.chapters
            .get(&file)
            .cloned()
            .ok_or_else(|| AppError::other(tr!("Chapter \"{file}\" isn't in the project", "الفصل \"{file}\" ليس في المشروع")))
    })
}

/// The `"project"` value of a chapter file: the project file, relative to the
/// chapter's folder (`chapters/01.nawl` → `../book.nawlproj`).
fn project_link(chapter: &str, project: &Path) -> String {
    let depth = chapter.replace('\\', "/").matches('/').count();
    let name = project.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    format!("{}{name}", "../".repeat(depth))
}

fn write_chapter(project: &Path, folder: &Path, file: &str, doc: &Value) -> Result<(), AppError> {
    if !nawl_core::is_safe_chapter_path(file) {
        return Err(AppError::other(tr!("Chapter file \"{file}\" must be inside the project folder", "يجب أن يكون ملف الفصل \"{file}\" داخل مجلد المشروع")));
    }
    let json = serde_json::json!({
        "format": DOC_FORMAT,
        "version": DOC_VERSION,
        "project": project_link(file, project),
        "doc": doc,
    });
    let dest = folder.join(file);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dest, serde_json::to_string_pretty(&json).map_err(AppError::other)?)?;
    Ok(())
}

/// Saves a chapter (the editor's JSON, as-is).
#[tauri::command]
pub fn save_chapter(store: State<'_, ProjectStore>, file: String, doc: Value) -> Result<(), AppError> {
    serde_json::from_value::<Body>(doc.clone()).map_err(|e| AppError::other(tr!("Unreadable document: {e}", "تتعذّر قراءة المستند: {e}")))?;
    with_project(&store, |p| {
        write_chapter(&p.path, &p.folder(), &file, &doc)?;
        p.chapters.insert(file, doc);
        Ok(())
    })
}

/// The project file's JSON: the editor's values over the file as read, with a copy
/// of the active template (as in a `.nawl`).
fn project_json(state: &AppState, raw: &Value, file: &ProjectFile) -> Result<Value, AppError> {
    let active = state.templates.current();
    let mut json = if raw.is_object() { raw.clone() } else { serde_json::json!({}) };
    let fields = serde_json::to_value(file).map_err(AppError::other)?;
    let obj = json.as_object_mut().expect("object");
    if let Value::Object(fields) = fields {
        obj.extend(fields);
    }
    obj.insert("template".into(), serde_json::to_value(&*active.template).map_err(AppError::other)?);
    obj.insert("templatePath".into(), serde_json::to_value(&active.path).map_err(AppError::other)?);
    Ok(json)
}

/// Saves the project file: metadata, bibliography and the chapter list (order,
/// parts, appendices), with the active template. A chapter listed for the first time
/// whose file doesn't exist yet starts empty.
#[tauri::command]
pub fn save_project(
    state: State<'_, AppState>,
    store: State<'_, ProjectStore>,
    metadata: Metadata,
    bibliography: String,
    chapters: Vec<ProjectEntry>,
) -> Result<ProjectIndex, AppError> {
    with_project(&store, |p| {
        let mut file = p.file.clone();
        file.metadata = metadata;
        file.bibliography = bibliography;
        file.chapters = chapters;
        if let Some(e) = file.validate().into_iter().find(|i| i.severity == Severity::Error) {
            return Err(AppError::other(e.message));
        }
        let json = project_json(&state, &p.raw, &file)?;
        std::fs::write(&p.path, serde_json::to_string_pretty(&json).map_err(AppError::other)?)?;
        p.raw = json;
        p.file = file;
        let empty = serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph" }] });
        for name in p.file.chapter_files().map(str::to_string).collect::<Vec<_>>() {
            if !p.chapters.contains_key(&name) {
                write_chapter(&p.path, &p.folder(), &name, &empty)?;
                p.chapters.insert(name, empty.clone());
            }
        }
        let index = p.project(None, p.file.metadata.clone(), p.file.bibliography.clone())?.index();
        Ok(index)
    })
}

/// Creates a chapter file starting with a chapter heading, under `chapters/`
/// (`chapters/03.nawl`, appendices `chapters/a1.nawl`), and returns its path. The
/// caller adds it to the chapter list and saves the project.
#[tauri::command]
pub fn new_chapter(store: State<'_, ProjectStore>, title: String, appendix: bool) -> Result<String, AppError> {
    with_project(&store, |p| {
        let folder = p.folder();
        let taken = |name: &str| p.chapters.contains_key(name) || p.file.chapter_files().any(|f| f == name) || folder.join(name).exists();
        let name = (1..)
            .map(|n| if appendix { format!("chapters/a{n}.nawl") } else { format!("chapters/{n:02}.nawl") })
            .find(|n| !taken(n))
            .expect("a free name");
        let doc = serde_json::json!({ "type": "doc", "content": [
            { "type": "heading", "attrs": { "level": 1 }, "content": if title.trim().is_empty() { Value::Array(vec![]) } else { serde_json::json!([{ "type": "text", "text": title.trim() }]) } },
            { "type": "paragraph" }
        ]});
        write_chapter(&p.path, &folder, &name, &doc)?;
        p.chapters.insert(name.clone(), doc);
        Ok(name)
    })
}

/// The index of every chapter, with the open chapter's live content.
#[tauri::command]
pub fn project_index(store: State<'_, ProjectStore>, file: Option<String>, doc: Option<Value>) -> Result<ProjectIndex, AppError> {
    with_project(&store, |p| {
        let open = file.as_deref().zip(doc.as_ref());
        Ok(p.project(open, p.file.metadata.clone(), p.file.bibliography.clone())?.index())
    })
}

/// What a compile's numbers, references and citations depend on across chapters:
/// the template, the engine, the bibliography, the chapter list, and each chapter's
/// numbered targets and citations. A one-chapter compile is right only while this
/// is what the last full compile was made from.
fn signature(template: &nawl_core::Template, engine: &str, project: &Project, entries: &[ProjectEntry]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    serde_json::to_string(template).unwrap_or_default().hash(&mut h);
    engine.hash(&mut h);
    project.bibliography.hash(&mut h);
    serde_json::to_string(entries).unwrap_or_default().hash(&mut h);
    for c in project.index().chapters {
        for t in &c.targets {
            (&t.id, t.kind as u8, t.level, t.numbered).hash(&mut h);
        }
        c.citations.hash(&mut h);
    }
    h.finish()
}

/// The project's kept build folder for an engine (in the app's cache folder).
fn build_folder(app: &AppHandle, project: &Path, engine: &str) -> PathBuf {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    project.hash(&mut h);
    let cache = app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir().join("nawl-cache"));
    cache.join("projects").join(format!("{:016x}-{engine}", h.finish()))
}

/// Compiles the project. With `only_chapter`, just the open chapter (`\includeonly`),
/// unless the numbers could be out of date (first compile, chapters added, removed
/// or reordered, a numbered target or citation added or removed anywhere, the
/// template or bibliography changed): then the whole book. Returns the PDF.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn render_project(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, ProjectStore>,
    file: Option<String>,
    doc: Option<Value>,
    metadata: Metadata,
    bibliography: String,
    only_chapter: bool,
) -> Result<Response, AppError> {
    Ok(Response::new(compile_project(&app, &state, &store, file, doc, metadata, bibliography, only_chapter).await?))
}

/// Compiles the whole project and writes the PDF to `path`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn export_project_pdf(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, ProjectStore>,
    path: String,
    file: Option<String>,
    doc: Option<Value>,
    metadata: Metadata,
    bibliography: String,
) -> Result<(), AppError> {
    let pdf = compile_project(&app, &state, &store, file, doc, metadata, bibliography, false).await?;
    std::fs::write(path, pdf)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn compile_project(
    app: &AppHandle,
    state: &AppState,
    store: &ProjectStore,
    file: Option<String>,
    doc: Option<Value>,
    metadata: Metadata,
    bibliography: String,
    only_chapter: bool,
) -> Result<Vec<u8>, AppError> {
    let active = state.templates.current();
    if let Some(e) = active.missing_kit() {
        return Err(e);
    }
    let (project, entries, path, last_full) = with_project(store, |p| {
        let project = p.project(file.as_deref().zip(doc.as_ref()), metadata, bibliography)?;
        Ok((project, p.file.chapters.clone(), p.path.clone(), p.last_full))
    })?;
    if let Some(e) = project.validate().into_iter().find(|i| i.severity == Severity::Error) {
        return Err(AppError::other(e.message));
    }
    let combined = project.combined();
    let (runner, target) = crate::choose_runner(state, &active.template, &combined)?;
    let dir = build_folder(app, &path, runner.id());
    let sig = signature(&active.template, runner.id(), &project, &entries);
    let open_index = file.as_deref().and_then(|f| project.chapters().position(|(name, _, _)| name == f));
    let only = open_index.filter(|_| only_chapter && last_full == Some(sig) && dir.join("main.aux").is_file());

    let generated = nawl_codegen::generate_project_for(&active.template, &project, target, only);
    let mut assets = active.load_assets(&generated.assets)?;
    assets.extend(active.load_kit_files(&generated.kit_files)?);
    assets.extend(state.doc_folder.load(&generated.figures)?);
    assets.extend(crate::load_library_fonts(&generated.fonts)?);
    let chapter_files: Vec<String> = project.chapters().map(|(name, _, _)| name.to_string()).collect();
    let tex_paths: Vec<String> = generated.chapters.iter().map(|c| format!("{}.tex", c.path)).collect();
    let lines: Vec<Vec<u32>> = generated.chapters.iter().map(|c| c.block_lines.clone()).collect();
    let build = dir.clone();
    let out = tauri::async_runtime::spawn_blocking(move || {
        let support = generated.support_files();
        let files: Vec<Asset> = support.iter().chain(assets.iter()).map(|(p, b)| Asset { path: p, bytes: b }).collect();
        runner.compile_in(&build, &generated.tex, &files)
    })
    .await
    .map_err(AppError::other)?
    .map_err(|e| match e {
        // Errors say which chapter and block they came from.
        CompileError::Failed { errors, log } => AppError::TexErrors {
            errors: errors.into_iter().map(|e| locate(e, &tex_paths, &chapter_files, &lines)).collect(),
            log,
        },
        other => other.into(),
    })?;
    let synctex = SyncTex::read_with(&dir, "main", &tex_paths);
    with_project(store, |p| {
        if only.is_none() {
            p.last_full = Some(sig);
        }
        p.sync = synctex.map(|synctex| ProjectSync { synctex, files: chapter_files, lines });
        Ok(())
    })?;
    Ok(out.pdf)
}

/// The chapter and block a TeX error came from (errors in the main file: none).
fn locate(error: nawl_compile::TexError, tex_paths: &[String], chapters: &[String], lines: &[Vec<u32>]) -> LocatedTexError {
    let file = error.file.as_deref().map(|f| f.replace('\\', "/"));
    let chapter = file.and_then(|f| tex_paths.iter().position(|p| f == *p || f.ends_with(&format!("/{p}"))));
    let block = chapter.zip(error.line).and_then(|(c, line)| {
        let l = &lines[c];
        l.first().filter(|&&first| line >= first).and_then(|_| l.partition_point(|&b| b <= line).checked_sub(1))
    });
    LocatedTexError { error, block, chapter: chapter.map(|c| chapters[c].clone()) }
}

/// A place in a project: a chapter file and a top-level block in it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSpot {
    file: String,
    block: usize,
}

/// The chapter and block behind a point of the PDF preview (in PDF points from the
/// page's top-left corner), from the last compile.
#[tauri::command]
pub fn project_pdf_to_block(store: State<'_, ProjectStore>, page: u32, x: f32, y: f32) -> Option<ProjectSpot> {
    let guard = store.0.lock().expect("project lock");
    let sync = guard.as_ref()?.sync.as_ref()?;
    let (file, line) = sync.synctex.place_at(page, x, y)?;
    let chapter = file.checked_sub(1)?;
    let block = sync.lines.get(chapter)?.partition_point(|&l| l <= line).checked_sub(1)?;
    Some(ProjectSpot { file: sync.files.get(chapter)?.clone(), block })
}

/// Where a block of a chapter is in the PDF of the last compile (blocks that print
/// nothing: the next one).
#[tauri::command]
pub fn project_block_to_pdf(store: State<'_, ProjectStore>, file: String, block: usize) -> Option<PdfSpot> {
    let guard = store.0.lock().expect("project lock");
    let sync = guard.as_ref()?.sync.as_ref()?;
    let chapter = sync.files.iter().position(|f| *f == file)?;
    let lines = sync.lines.get(chapter)?;
    (block..lines.len()).find_map(|i| sync.synctex.spot_in(chapter + 1, lines[i], lines.get(i + 1).copied().unwrap_or(u32::MAX)))
}

/// Writes `main.tex` at `path`, with the chapter files, class, bibliography, figures
/// and template assets next to it, ready to compile anywhere (Overleaf handles
/// `\include`).
#[tauri::command]
pub fn export_project_tex(
    state: State<'_, AppState>,
    store: State<'_, ProjectStore>,
    path: String,
    file: Option<String>,
    doc: Option<Value>,
    metadata: Metadata,
    bibliography: String,
) -> Result<(), AppError> {
    let project = with_project(&store, |p| p.project(file.as_deref().zip(doc.as_ref()), metadata, bibliography))?;
    let active = state.templates.current();
    let combined = project.combined();
    let mut target = state.engine().target();
    if target == nawl_codegen::Engine::Pdf && nawl_codegen::language::Languages::of(&active.template, &combined).need_unicode_engine() {
        // Arabic can't be typeset by pdfLaTeX: written for XeLaTeX instead.
        target = nawl_codegen::Engine::Xe;
    }
    let g = nawl_codegen::generate_project_for(&active.template, &project, target, None);
    let mut assets = active.load_assets(&g.assets)?;
    assets.extend(active.load_kit_files(&g.kit_files)?);
    assets.extend(state.doc_folder.load(&g.figures)?);
    assets.extend(crate::load_library_fonts(&g.fonts)?);
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

/// Writes the whole project as one Word file (parts aren't written: Word export has
/// no parts). `mathml` maps every chapter's formulas' LaTeX to MathML.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn export_project_docx(
    state: State<'_, AppState>,
    store: State<'_, ProjectStore>,
    path: String,
    file: Option<String>,
    doc: Option<Value>,
    metadata: Metadata,
    bibliography: String,
    mathml: HashMap<String, String>,
    pictures: Option<HashMap<String, crate::FormulaPictureIn>>,
) -> Result<(), AppError> {
    let project = with_project(&store, |p| p.project(file.as_deref().zip(doc.as_ref()), metadata, bibliography))?;
    crate::write_docx(&state, project.combined(), path, mathml, pictures.unwrap_or_default()).await
}

/// Splits a single document into a project in `folder` (created if needed; it must
/// not hold a project already): one chapter file per chapter heading, appendices
/// after the appendix marker, figures copied along. Text before the first chapter
/// heading becomes a chapter of its own. Returns the project file's path, for
/// `open_project`.
#[tauri::command]
pub fn split_into_project(
    state: State<'_, AppState>,
    folder: String,
    name: String,
    doc: Value,
    metadata: Metadata,
    bibliography: Option<String>,
) -> Result<String, AppError> {
    let folder = PathBuf::from(folder);
    let stem = name.trim();
    let stem = nawl_core::PROJECT_EXTENSIONS.iter().fold(stem, |s, ext| s.trim_end_matches(&format!(".{ext}") as &str)).trim();
    if stem.is_empty() || stem.contains(['/', '\\']) {
        return Err(AppError::other(tr!("Choose a name for the project", "اختر اسمًا للمشروع")));
    }
    let path = folder.join(format!("{stem}.{}", nawl_core::PROJECT_EXTENSION));
    if path.exists() || folder.join("chapters").exists() {
        return Err(AppError::other(tr!(
            "The folder already holds a project; choose an empty folder",
            "يحتوي المجلد على مشروع بالفعل؛ اختر مجلدًا فارغًا"
        )));
    }
    let figures = crate::figure_sources(&doc)?;
    let (entries, chapters) = split_blocks(&doc);
    std::fs::create_dir_all(&folder)?;
    files::copy_relative(&figures, &state.doc_folder.get(), &folder)?;
    for (file, chapter) in &chapters {
        write_chapter(&path, &folder, file, chapter)?;
    }
    let mut file = ProjectFile::new();
    file.metadata = metadata;
    file.bibliography = bibliography.unwrap_or_default();
    file.chapters = entries;
    let json = project_json(&state, &Value::Null, &file)?;
    std::fs::write(&path, serde_json::to_string_pretty(&json).map_err(AppError::other)?)?;
    Ok(path.display().to_string())
}

/// An example project ("arabic-thesis": the Arabic thesis in two parts, else the
/// senior design report), written to a fresh temporary folder with its images.
/// Returns the project file's path, for `open_project`.
#[tauri::command]
pub fn sample_project(state: State<'_, AppState>, name: Option<String>) -> Result<String, AppError> {
    let sample = SampleProject::named(name.as_deref());
    state.doc_folder.reset_with(sample.files)?;
    Ok(sample.write(&state.doc_folder.get())?.display().to_string())
}

/// An example document offered as a project.
struct SampleProject {
    /// File name of the project, without the extension.
    stem: &'static str,
    /// The example `.nawl` (with its template, or none for the built-in one).
    document: &'static str,
    files: &'static [(&'static str, &'static [u8])],
    /// Parts: the title, and how many chapters come before it.
    parts: &'static [(usize, &'static str)],
}

impl SampleProject {
    fn named(name: Option<&str>) -> Self {
        match name {
            Some("arabic-thesis") => SampleProject {
                stem: "arabic-thesis",
                document: crate::SAMPLE_ARABIC_THESIS,
                files: crate::SAMPLE_THESIS_FILES,
                parts: &[(0, "الإطار العام للدراسة"), (2, "الدراسة التطبيقية")],
            },
            _ => SampleProject { stem: "senior-design-report", document: crate::SAMPLE_DOCUMENT, files: crate::SAMPLE_FILES, parts: &[] },
        }
    }

    /// Splits the example into chapter files in `folder` and writes the project file.
    fn write(&self, folder: &Path) -> Result<PathBuf, AppError> {
        let source: Value = serde_json::from_str(self.document).map_err(AppError::other)?;
        let path = folder.join(format!("{}.{}", self.stem, nawl_core::PROJECT_EXTENSION));
        let (chapters, bodies) = split_blocks(&source["doc"]);
        for (file, body) in &bodies {
            write_chapter(&path, folder, file, body)?;
        }
        let mut file = ProjectFile::new();
        file.metadata = serde_json::from_value(source["metadata"].clone()).map_err(AppError::other)?;
        file.bibliography = source["bibliography"].as_str().unwrap_or_default().to_string();
        let mut before = 0;
        for entry in chapters {
            if matches!(entry, ProjectEntry::Chapter { appendix: false, .. }) {
                file.chapters.extend(self.parts.iter().filter(|(n, _)| *n == before).map(|(_, title)| ProjectEntry::Part { part: (*title).into() }));
                before += 1;
            }
            file.chapters.push(entry);
        }
        let mut json = serde_json::to_value(&file).map_err(AppError::other)?;
        // The example's own template; the built-in one for the report.
        let template = match source.get("template") {
            Some(t) if !t.is_null() => t.clone(),
            _ => serde_json::from_str(crate::templates::BUILTIN_TEMPLATE).map_err(AppError::other)?,
        };
        json["template"] = template;
        json["templatePath"] = Value::Null;
        std::fs::write(&path, serde_json::to_string_pretty(&json).map_err(AppError::other)?)?;
        Ok(path)
    }
}

/// A document's top-level blocks split at chapter headings and the appendix marker:
/// the chapter list and each chapter file's body JSON.
fn split_blocks(doc: &Value) -> (Vec<ProjectEntry>, Vec<(String, Value)>) {
    let blocks = doc.get("content").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut groups: Vec<(bool, Vec<Value>)> = Vec::new();
    let mut appendix = false;
    for b in blocks {
        let kind = b.get("type").and_then(Value::as_str).unwrap_or("");
        if kind == "appendixStart" {
            appendix = true;
            continue;
        }
        let chapter_heading = kind == "heading" && b.pointer("/attrs/level").and_then(Value::as_u64) == Some(1);
        match groups.last_mut() {
            Some((a, g)) if !chapter_heading && *a == appendix => g.push(b),
            _ => groups.push((appendix, vec![b])),
        }
    }
    if groups.is_empty() {
        groups.push((false, vec![serde_json::json!({ "type": "paragraph" })]));
    }
    let (mut n, mut a) = (0, 0);
    let mut entries = Vec::new();
    let mut chapters = Vec::new();
    for (appendix, content) in groups {
        let file = if appendix {
            a += 1;
            format!("chapters/a{a}.nawl")
        } else {
            n += 1;
            format!("chapters/{n:02}.nawl")
        };
        entries.push(ProjectEntry::Chapter { file: file.clone(), appendix });
        chapters.push((file, serde_json::json!({ "type": "doc", "content": content })));
    }
    (entries, chapters)
}

/// Writes the whole project as one `.nawl` at `path` (parts become nothing: a
/// single document has no parts), with figures copied along when it's elsewhere.
#[tauri::command]
pub fn join_project(
    state: State<'_, AppState>,
    store: State<'_, ProjectStore>,
    path: String,
    file: Option<String>,
    doc: Option<Value>,
    metadata: Metadata,
    bibliography: String,
) -> Result<(), AppError> {
    let joined = with_project(&store, |p| {
        let mut content = Vec::new();
        let mut in_appendix = false;
        for entry in &p.file.chapters {
            let ProjectEntry::Chapter { file: name, appendix } = entry else { continue };
            if *appendix && !in_appendix {
                content.push(serde_json::json!({ "type": "appendixStart" }));
                in_appendix = true;
            }
            let body = match (&file, &doc) {
                (Some(f), Some(d)) if f == name => Some(d),
                _ => p.chapters.get(name),
            };
            if let Some(blocks) = body.and_then(|b| b.get("content")).and_then(Value::as_array) {
                content.extend(blocks.iter().cloned());
            }
        }
        Ok(serde_json::json!({ "type": "doc", "content": content }))
    })?;
    let figures = crate::figure_sources(&joined)?;
    let dest_dir = Path::new(&path).parent().map(Path::to_path_buf).unwrap_or_default();
    files::copy_relative(&figures, &state.doc_folder.get(), &dest_dir)?;
    let json = crate::document_file(&state, joined, metadata, Some(bibliography));
    std::fs::write(&path, serde_json::to_string_pretty(&json).map_err(AppError::other)?)?;
    Ok(())
}

/// The project a chapter file belongs to (its `"project"` value, resolved against
/// the chapter's folder), when that project file exists.
pub fn project_of_chapter(chapter_path: &Path, file: &Value) -> Option<String> {
    let link = file.get("project")?.as_str()?;
    let path = normalize(&chapter_path.parent()?.join(link));
    path.is_file().then(|| path.display().to_string())
}

/// `a/chapters/../b` → `a/b`, without touching the disk (canonicalize gives
/// `\\?\C:\…` paths on Windows, which then show in titles and recent files).
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push(c);
                }
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn splits_a_document_into_chapters() {
        let h = |t: &str| json!({"type": "heading", "attrs": {"level": 1}, "content": [{"type": "text", "text": t}]});
        let p = |t: &str| json!({"type": "paragraph", "content": [{"type": "text", "text": t}]});
        let doc = json!({"type": "doc", "content": [
            p("before any chapter"),
            h("One"), p("a"), {"type": "heading", "attrs": {"level": 2}},
            h("Two"), p("b"),
            {"type": "appendixStart"},
            h("Survey"), p("c"), {"type": "futureNode"}
        ]});
        let (entries, chapters) = split_blocks(&doc);
        let files: Vec<&str> = chapters.iter().map(|(f, _)| f.as_str()).collect();
        assert_eq!(files, ["chapters/01.nawl", "chapters/02.nawl", "chapters/03.nawl", "chapters/a1.nawl"]);
        assert_eq!(entries[3], ProjectEntry::Chapter { file: "chapters/a1.nawl".into(), appendix: true });
        let len = |i: usize| chapters[i].1["content"].as_array().unwrap().len();
        assert_eq!((len(0), len(1), len(2), len(3)), (1, 3, 2, 3));
        // Unknown node types are kept.
        assert_eq!(chapters[3].1["content"][2]["type"], "futureNode");
        // An empty document still makes one chapter.
        assert_eq!(split_blocks(&json!({"type": "doc", "content": []})).1.len(), 1);
    }

    /// Split → chapter files on disk → read back as a project → LaTeX.
    #[test]
    fn split_project_reads_back_and_generates() {
        let h = |id: &str, t: &str| json!({"type": "heading", "attrs": {"level": 1, "id": id}, "content": [{"type": "text", "text": t}]});
        let doc = json!({"type": "doc", "content": [
            h("c1", "مقدمة"),
            {"type": "equation", "attrs": {"latex": "x", "id": "eq"}},
            h("c2", "النتائج"),
            {"type": "paragraph", "content": [{"type": "ref", "attrs": {"target": "eq"}}]}
        ]});
        let dir = tempfile::tempdir().unwrap();
        let project_path = dir.path().join("book.nawlproj");
        let (entries, chapters) = split_blocks(&doc);
        for (file, body) in &chapters {
            write_chapter(&project_path, dir.path(), file, body).unwrap();
        }
        let mut file = ProjectFile::new();
        file.chapters = entries;
        std::fs::write(&project_path, serde_json::to_string(&file).unwrap()).unwrap();

        // A chapter file names its project, which a lone open finds.
        let chapter_path = dir.path().join("chapters/02.nawl");
        let on_disk: Value = serde_json::from_str(&std::fs::read_to_string(&chapter_path).unwrap()).unwrap();
        assert_eq!(on_disk["project"], "../book.nawlproj");
        let found = project_of_chapter(&chapter_path, &on_disk).unwrap();
        assert_eq!(PathBuf::from(found), project_path, "a plain path, without ..");

        let project = Project::load(&file, |name| read_chapter_file(&dir.path().join(name)).and_then(|d| serde_json::from_value::<Body>(d).map_err(|e| e.to_string()))).unwrap();
        assert!(project.validate().is_empty(), "{:?}", project.validate());
        let template: nawl_core::Template = serde_json::from_str(include_str!("../../templates/arabic-book/template.json")).unwrap();
        let g = nawl_codegen::generate_project_for(&template, &project, nawl_codegen::Engine::Xe, None);
        assert_eq!(g.chapters.len(), 2);
        assert!(g.chapters[1].tex.contains("\\eqref{tl:eq}"));
    }

    /// The example projects open without problems: chapters, parts, an appendix,
    /// their images, and a template that takes their metadata.
    #[test]
    fn sample_projects_are_complete() {
        for (name, parts) in [(Some("arabic-thesis"), 2), (None, 0)] {
            let sample = SampleProject::named(name);
            let dir = tempfile::tempdir().unwrap();
            let path = sample.write(dir.path()).unwrap();
            let raw: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let file: ProjectFile = serde_json::from_value(raw.clone()).unwrap();
            assert!(file.validate().is_empty(), "{name:?}: {:?}", file.validate());
            assert_eq!(file.chapters.iter().filter(|e| matches!(e, ProjectEntry::Part { .. })).count(), parts);
            assert!(matches!(file.chapters.last(), Some(ProjectEntry::Chapter { appendix: true, .. })), "{name:?}");
            if parts > 0 {
                assert!(matches!(file.chapters[0], ProjectEntry::Part { .. }) && matches!(file.chapters[3], ProjectEntry::Part { .. }), "{:?}", file.chapters);
            }
            let project = Project::load(&file, |f| read_chapter_file(&dir.path().join(f)).and_then(|d| serde_json::from_value::<Body>(d).map_err(|e| e.to_string()))).unwrap();
            assert!(project.validate().is_empty(), "{name:?}: {:?}", project.validate());
            let template: nawl_core::Template = serde_json::from_value(raw["template"].clone()).unwrap();
            let document = project.combined();
            assert!(template.check_metadata(&document.metadata).is_empty(), "{name:?}");
            let figures = document.body.figure_sources();
            assert!(!figures.is_empty() && figures.iter().all(|f| sample.files.iter().any(|(p, _)| p == f)), "{name:?}: {figures:?}");
            let g = nawl_codegen::generate_project_for(&template, &project, nawl_codegen::Engine::Xe, None);
            assert_eq!(g.tex.matches("\\texloompart{").count(), parts, "{name:?}");
        }
    }

    /// The example projects compile, with their references resolved.
    /// Run with NAWL_TECTONIC=… cargo test -p nawl sample_projects_compile -- --ignored
    /// (NAWL_SAVE_PDF=<folder> keeps the PDFs).
    #[test]
    #[ignore]
    fn sample_projects_compile() {
        for name in [Some("arabic-thesis"), None] {
            let sample = SampleProject::named(name);
            let dir = tempfile::tempdir().unwrap();
            let path = sample.write(dir.path()).unwrap();
            let raw: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let file: ProjectFile = serde_json::from_value(raw.clone()).unwrap();
            let project = Project::load(&file, |f| read_chapter_file(&dir.path().join(f)).and_then(|d| serde_json::from_value::<Body>(d).map_err(|e| e.to_string()))).unwrap();
            let template: nawl_core::Template = serde_json::from_value(raw["template"].clone()).unwrap();
            let g = nawl_codegen::generate_project_for(&template, &project, nawl_codegen::Engine::Xe, None);
            let support = g.support_files();
            let figures: Vec<(String, Vec<u8>)> = g.figures.iter().map(|f| (f.clone(), sample.files.iter().find(|(p, _)| p == f).unwrap().1.to_vec())).collect();
            let files: Vec<Asset> = support.iter().chain(figures.iter()).map(|(p, b)| Asset { path: p, bytes: b }).collect();
            let out = nawl_compile::Compiler::locate(&[]).expect("tectonic").compile(&g.tex, &files).unwrap_or_else(|e| panic!("{name:?}: {e:?}"));
            assert!(out.pdf.starts_with(b"%PDF"));
            if let Some(save) = std::env::var_os("NAWL_SAVE_PDF") {
                std::fs::write(Path::new(&save).join(format!("{}.pdf", sample.stem)), &out.pdf).unwrap();
            }
            let undefined: Vec<&str> = out.log.lines().filter(|l| l.contains("undefined")).collect();
            assert!(undefined.is_empty(), "{name:?}: {undefined:#?}");
        }
    }

    #[test]
    fn chapter_files_link_back_to_the_project() {
        let project = Path::new("/books/thesis/thesis.loomproj");
        assert_eq!(project_link("chapters/01.loom", project), "../thesis.loomproj");
        assert_eq!(project_link("intro.loom", project), "thesis.loomproj");
        assert_eq!(project_link("a/b/c.loom", project), "../../thesis.loomproj");
    }

    #[test]
    fn errors_point_at_chapters_and_blocks() {
        let tex_paths = vec!["chapters/ch01.tex".to_string(), "chapters/ch02.tex".to_string()];
        let chapters = vec!["chapters/01.loom".to_string(), "chapters/02.loom".to_string()];
        let lines = vec![vec![2, 5], vec![2, 9, 14]];
        let err = |file: Option<&str>, line| nawl_compile::TexError { file: file.map(String::from), line, message: "x".into() };
        let e = locate(err(Some("./chapters/ch02.tex"), Some(10)), &tex_paths, &chapters, &lines);
        assert_eq!((e.chapter.as_deref(), e.block), (Some("chapters/02.loom"), Some(1)));
        let e = locate(err(Some("main.tex"), Some(40)), &tex_paths, &chapters, &lines);
        assert_eq!((e.chapter, e.block), (None, None));
        let e = locate(err(Some("chapters/ch01.tex"), Some(1)), &tex_paths, &chapters, &lines);
        assert_eq!((e.chapter.as_deref(), e.block), (Some("chapters/01.loom"), None), "the header line");
    }
}
