//! Compiles LaTeX source to PDF by running the Tectonic engine as a child process.
//!
//! Each compile runs in a fresh temporary directory: the main `.tex` file plus any
//! assets (images, `.cls` files, bibliography) are written there, Tectonic runs, and the
//! PDF bytes and log come back. TeX errors are pulled out of Tectonic's output so the
//! UI can show them as a readable list instead of a raw log.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// Name of the main file inside the build directory.
const MAIN: &str = "main";

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("Tectonic executable not found (looked at: {0})")]
    NotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid asset path: {0}")]
    BadAssetPath(String),
    #[error("LaTeX compilation failed")]
    Failed { errors: Vec<TexError>, log: String },
}

/// One error reported by TeX, with the source line when Tectonic knows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TexError {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

#[derive(Debug)]
pub struct CompileOutput {
    pub pdf: Vec<u8>,
    pub log: String,
}

/// An extra file written next to the main `.tex` file before compiling.
pub struct Asset<'a> {
    /// Path relative to the build directory, e.g. `report.cls` or `assets/logo.png`.
    pub path: &'a str,
    pub bytes: &'a [u8],
}

pub struct Compiler {
    tectonic: PathBuf,
}

impl Compiler {
    /// Uses the given Tectonic executable.
    pub fn new(tectonic: impl Into<PathBuf>) -> Result<Self, CompileError> {
        let tectonic = tectonic.into();
        if tectonic.is_file() {
            Ok(Self { tectonic })
        } else {
            Err(CompileError::NotFound(tectonic.display().to_string()))
        }
    }

    /// Finds Tectonic: the first existing path in `candidates`, then the
    /// `TEXLOOM_TECTONIC` environment variable, then `tectonic` on `PATH`.
    pub fn locate(candidates: &[PathBuf]) -> Result<Self, CompileError> {
        let mut tried: Vec<String> = Vec::new();
        let mut all: Vec<PathBuf> = candidates.to_vec();
        if let Some(env) = std::env::var_os("TEXLOOM_TECTONIC") {
            all.push(PathBuf::from(env));
        }
        if let Some(path) = std::env::var_os("PATH") {
            all.extend(std::env::split_paths(&path).map(|dir| dir.join("tectonic")));
        }
        for c in all {
            if c.is_file() {
                return Ok(Self { tectonic: c });
            }
            tried.push(c.display().to_string());
        }
        Err(CompileError::NotFound(tried.join(", ")))
    }

    pub fn tectonic_path(&self) -> &Path {
        &self.tectonic
    }

    /// Compiles `tex` (the contents of the main file) with the given assets.
    /// Blocking: call from a worker thread in async contexts.
    pub fn compile(&self, tex: &str, assets: &[Asset<'_>]) -> Result<CompileOutput, CompileError> {
        let dir = tempfile::Builder::new().prefix("texloom-build-").tempdir()?;
        let root = dir.path();

        std::fs::write(root.join(format!("{MAIN}.tex")), tex)?;
        for asset in assets {
            let rel = safe_relative(asset.path)?;
            let dest = root.join(rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(dest, asset.bytes)?;
        }

        let output = Command::new(&self.tectonic)
            .current_dir(root)
            // Never allow \write18 shell escape: documents and templates may be shared.
            .arg("--untrusted")
            .arg("--keep-logs")
            .args(["--outdir", "."])
            .arg(format!("{MAIN}.tex"))
            .output()?;

        let log = std::fs::read_to_string(root.join(format!("{MAIN}.log"))).unwrap_or_default();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let pdf_path = root.join(format!("{MAIN}.pdf"));

        if output.status.success() && pdf_path.is_file() {
            Ok(CompileOutput { pdf: std::fs::read(pdf_path)?, log })
        } else {
            let mut errors = parse_stderr(&stderr);
            if errors.is_empty() {
                errors = parse_log(&log);
            }
            if errors.is_empty() {
                errors.push(TexError {
                    file: None,
                    line: None,
                    message: stderr.trim().to_string(),
                });
            }
            let full_log = if log.is_empty() { stderr.into_owned() } else { log };
            Err(CompileError::Failed { errors, log: full_log })
        }
    }
}

/// Rejects absolute paths and `..` so assets can't be written outside the build dir.
fn safe_relative(p: &str) -> Result<&Path, CompileError> {
    let path = Path::new(p);
    let ok = !p.is_empty()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
    if ok { Ok(path) } else { Err(CompileError::BadAssetPath(p.to_string())) }
}

/// Parses Tectonic's stderr, whose errors look like
/// `error: main.tex:5: Undefined control sequence`.
fn parse_stderr(stderr: &str) -> Vec<TexError> {
    stderr
        .lines()
        .filter_map(|l| l.strip_prefix("error: "))
        .filter(|rest| !rest.starts_with("halted on potentially-recoverable error"))
        .map(|rest| {
            let mut parts = rest.splitn(3, ':');
            let (a, b, c) = (parts.next(), parts.next(), parts.next());
            match (a, b.and_then(|b| b.trim().parse::<u32>().ok()), c) {
                (Some(file), Some(line), Some(msg)) => TexError {
                    file: Some(file.to_string()),
                    line: Some(line),
                    message: msg.trim().to_string(),
                },
                _ => TexError { file: None, line: None, message: rest.trim().to_string() },
            }
        })
        .collect()
}

/// Parses a classic TeX log: `! Message` followed later by `l.<n> ...`.
fn parse_log(log: &str) -> Vec<TexError> {
    let mut errors = Vec::new();
    let mut lines = log.lines().peekable();
    while let Some(l) = lines.next() {
        if let Some(msg) = l.strip_prefix("! ") {
            let mut line = None;
            for next in lines.by_ref().take(20) {
                if let Some(rest) = next.strip_prefix("l.") {
                    line = rest.split_whitespace().next().and_then(|n| n.parse().ok());
                    break;
                }
            }
            errors.push(TexError { file: None, line, message: msg.trim().to_string() });
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stderr_errors() {
        let s = "note: Running TeX ...\nerror: main.tex:5: Undefined control sequence\nerror: halted on potentially-recoverable error as specified\n";
        assert_eq!(
            parse_stderr(s),
            vec![TexError {
                file: Some("main.tex".into()),
                line: Some(5),
                message: "Undefined control sequence".into()
            }]
        );
    }

    #[test]
    fn parses_log_errors() {
        let log = "blah\n! Undefined control sequence.\n<argument> \\foo\nl.12 \\foo\n";
        assert_eq!(parse_log(log)[0].line, Some(12));
    }

    #[test]
    fn rejects_escaping_asset_paths() {
        assert!(safe_relative("../x").is_err());
        assert!(safe_relative("/etc/passwd").is_err());
        assert!(safe_relative("assets/logo.png").is_ok());
    }

    /// Needs network on first run (Tectonic downloads its package bundle).
    /// Run with: TEXLOOM_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p compile -- --ignored
    #[test]
    #[ignore]
    fn compiles_and_reports_errors() {
        let c = Compiler::locate(&[]).expect("tectonic");
        let ok = c
            .compile("\\documentclass{article}\\begin{document}Hello\\end{document}", &[])
            .expect("compile");
        assert!(ok.pdf.starts_with(b"%PDF"));

        match c.compile("\\documentclass{article}\\begin{document}\n\\nosuchmacro\n\\end{document}", &[]) {
            Err(CompileError::Failed { errors, .. }) => {
                assert!(errors.iter().any(|e| e.message.contains("Undefined control sequence")), "{errors:?}");
            }
            other => panic!("expected failure, got {other:?}"),
        }
    }
}
