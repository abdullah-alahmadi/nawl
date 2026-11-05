use std::path::PathBuf;
use std::sync::Arc;

use compile::{CompileError, Compiler, TexError};
use serde::Serialize;
use serde_json::Value;
use tauri::ipc::Response;
use tauri::State;

/// Version stamp written into saved document files.
const DOC_FORMAT_VERSION: u32 = 1;

struct AppState {
    compiler: Result<Arc<Compiler>, String>,
}

/// Errors sent to the frontend, tagged so the UI can render each kind differently.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum AppError {
    TectonicMissing { message: String },
    TexErrors { errors: Vec<TexError>, log: String },
    Other { message: String },
}

impl From<CompileError> for AppError {
    fn from(e: CompileError) -> Self {
        match e {
            CompileError::Failed { errors, log } => AppError::TexErrors { errors, log },
            CompileError::NotFound(_) => AppError::TectonicMissing { message: e.to_string() },
            other => AppError::Other { message: other.to_string() },
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Other { message: e.to_string() }
    }
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

/// Converts the editor document to LaTeX and compiles it. Returns raw PDF bytes
/// (an ArrayBuffer on the JS side, avoiding a huge JSON number array).
#[tauri::command]
async fn render_pdf(state: State<'_, AppState>, doc: Value) -> Result<Response, AppError> {
    let compiler = state
        .compiler
        .clone()
        .map_err(|message| AppError::TectonicMissing { message })?;
    let tex = codegen::document_to_latex(&doc);
    let out = tauri::async_runtime::spawn_blocking(move || compiler.compile(&tex, &[]))
        .await
        .map_err(|e| AppError::Other { message: e.to_string() })??;
    Ok(Response::new(out.pdf))
}

#[tauri::command]
fn to_latex(doc: Value) -> String {
    codegen::document_to_latex(&doc)
}

#[tauri::command]
fn save_document(path: String, doc: Value) -> Result<(), AppError> {
    let file = serde_json::json!({ "format": "texloom", "version": DOC_FORMAT_VERSION, "doc": doc });
    let text = serde_json::to_string_pretty(&file).map_err(|e| AppError::Other { message: e.to_string() })?;
    std::fs::write(path, text)?;
    Ok(())
}

#[tauri::command]
fn open_document(path: String) -> Result<Value, AppError> {
    let text = std::fs::read_to_string(path)?;
    let file: Value = serde_json::from_str(&text).map_err(|e| AppError::Other { message: format!("Not a valid document: {e}") })?;
    file.get("doc")
        .filter(|d| d.get("type").and_then(Value::as_str) == Some("doc"))
        .cloned()
        .ok_or_else(|| AppError::Other { message: "Not a Texloom document".into() })
}

#[tauri::command]
fn export_tex(path: String, doc: Value) -> Result<(), AppError> {
    std::fs::write(path, codegen::document_to_latex(&doc))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let compiler = Compiler::locate(&tectonic_candidates())
        .map(Arc::new)
        .map_err(|e| e.to_string());
    match &compiler {
        Ok(c) => eprintln!("Using Tectonic at {}", c.tectonic_path().display()),
        Err(e) => eprintln!("Warning: {e}"),
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { compiler })
        .invoke_handler(tauri::generate_handler![
            render_pdf,
            to_latex,
            save_document,
            open_document,
            export_tex
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
