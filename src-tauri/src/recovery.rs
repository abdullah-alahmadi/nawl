//! Crash recovery: while a document has unsaved changes, the editor writes it to the
//! app's data folder every few seconds. If Nawl closes without saving (a crash, a
//! power cut), the next start offers to bring those changes back.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::files;

const DOCUMENT: &str = "document.loom";
const INFO: &str = "info.json";
const FILES: &str = "files";

/// What the start-up prompt shows about the unsaved changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    /// The file the changes belong to; `None` for a document that was never saved.
    pub original_path: Option<String>,
    /// When the changes were written (seconds since 1970).
    pub saved_at: u64,
    /// For a chapter of a project: the chapter file (relative to the project folder);
    /// `original_path` is then the project file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter: Option<String>,
}

pub struct Recovery {
    dir: PathBuf,
}

impl Recovery {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Writes the document (a complete `.loom` file) and copies the figures it uses,
    /// so an unsaved document's images survive even when its temporary folder doesn't.
    pub fn write(&self, file: &Value, figures: &[String], figures_dir: &Path, original_path: Option<String>, chapter: Option<String>) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        files::copy_missing(figures, figures_dir, &self.dir.join(FILES))?;
        let info = RecoveryInfo { original_path, saved_at: now(), chapter };
        write_atomic(&self.dir.join(INFO), &serde_json::to_vec(&info)?)?;
        // Last, and atomically: the document only counts once everything else is there.
        write_atomic(&self.dir.join(DOCUMENT), &serde_json::to_vec(file)?)
    }

    /// Unsaved changes left by a previous run, if any.
    pub fn info(&self) -> Option<RecoveryInfo> {
        if !self.dir.join(DOCUMENT).is_file() {
            return None;
        }
        let info = std::fs::read(self.dir.join(INFO)).ok().and_then(|b| serde_json::from_slice(&b).ok());
        Some(info.unwrap_or(RecoveryInfo { original_path: None, saved_at: 0, chapter: None }))
    }

    /// The saved `.loom` text.
    pub fn read_document(&self) -> std::io::Result<String> {
        std::fs::read_to_string(self.dir.join(DOCUMENT))
    }

    /// Where the copied figures are (same relative paths as in the document).
    pub fn files_dir(&self) -> PathBuf {
        self.dir.join(FILES)
    }

    /// Forgets the changes (after saving, or when the user declines to restore them).
    pub fn discard(&self) -> std::io::Result<()> {
        match std::fs::remove_dir_all(&self.dir) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Writes to a temporary file first, so a crash mid-write never leaves half a file.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_restores_and_discards() {
        let root = std::env::temp_dir().join(format!("nawl-recovery-{}", std::process::id()));
        let doc_dir = root.join("doc");
        std::fs::create_dir_all(doc_dir.join("figures")).unwrap();
        std::fs::write(doc_dir.join("figures/a.png"), b"png").unwrap();
        let r = Recovery::new(root.join("recovery"));
        assert_eq!(r.info(), None);

        let file = serde_json::json!({"format": "texloom", "doc": {"type": "doc"}});
        r.write(&file, &["figures/a.png".into(), "../escape.png".into()], &doc_dir, Some("/x/report.loom".into()), None).unwrap();
        let info = r.info().unwrap();
        assert_eq!(info.original_path.as_deref(), Some("/x/report.loom"));
        assert_eq!(info.chapter, None);
        assert!(info.saved_at > 0);
        assert_eq!(serde_json::from_str::<Value>(&r.read_document().unwrap()).unwrap(), file);
        assert_eq!(std::fs::read(r.files_dir().join("figures/a.png")).unwrap(), b"png");
        assert!(!root.join("escape.png").exists());

        // A chapter of a project: the project file and the chapter.
        r.write(&file, &[], &doc_dir, Some("/x/book.loomproj".into()), Some("chapters/02.loom".into())).unwrap();
        assert_eq!(r.info().unwrap().chapter.as_deref(), Some("chapters/02.loom"));

        r.discard().unwrap();
        assert_eq!(r.info(), None);
        r.discard().unwrap(); // nothing left to discard is fine
        std::fs::remove_dir_all(root).unwrap();
    }
}
