//! Compiles LaTeX source to PDF by running a TeX engine as a child process: the
//! bundled Tectonic, or an installed TeX distribution (TeX Live, MiKTeX, MacTeX)
//! with pdfLaTeX, XeLaTeX or LuaLaTeX.
//!
//! Each compile runs in a fresh temporary directory: the main `.tex` file plus any
//! assets (images, `.cls` files, bibliography) are written there, the engine runs, and
//! the PDF bytes and log come back. TeX errors are pulled out of the output so the UI
//! can show them as a readable list instead of a raw log.

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

/// LaTeX programs of an installed TeX distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatexProgram {
    PdfLatex,
    XeLatex,
    LuaLatex,
}

impl LatexProgram {
    pub const ALL: [LatexProgram; 3] = [LatexProgram::PdfLatex, LatexProgram::XeLatex, LatexProgram::LuaLatex];

    pub fn command(self) -> &'static str {
        match self {
            LatexProgram::PdfLatex => "pdflatex",
            LatexProgram::XeLatex => "xelatex",
            LatexProgram::LuaLatex => "lualatex",
        }
    }

    /// latexmk's switch for this program.
    fn latexmk_flag(self) -> &'static str {
        match self {
            LatexProgram::PdfLatex => "-pdf",
            LatexProgram::XeLatex => "-xelatex",
            LatexProgram::LuaLatex => "-lualatex",
        }
    }
}

/// An installed TeX distribution's LaTeX program, run through latexmk when present
/// (it reruns as needed and calls BibTeX), else directly with BibTeX in between.
#[derive(Debug, Clone)]
pub struct SystemLatex {
    pub program: LatexProgram,
    /// The program's executable.
    pub path: PathBuf,
    pub latexmk: Option<PathBuf>,
    pub bibtex: Option<PathBuf>,
}

impl SystemLatex {
    /// Finds `program` on PATH or in the usual TeX installation folders.
    pub fn find(program: LatexProgram) -> Option<Self> {
        let path = find_tex_binary(program.command())?;
        // Prefer the helpers from the same installation.
        let sibling = |name: &str| {
            let exe = path.with_file_name(format!("{name}{}", std::env::consts::EXE_SUFFIX));
            if exe.is_file() { Some(exe) } else { find_tex_binary(name) }
        };
        Some(Self { program, latexmk: sibling("latexmk"), bibtex: sibling("bibtex"), path })
    }

    /// The program's first `--version` line, e.g. "pdfTeX 3.14… (TeX Live 2026)".
    pub fn version(&self) -> Option<String> {
        let out = Command::new(&self.path).arg("--version").output().ok()?;
        String::from_utf8_lossy(&out.stdout).lines().next().map(|l| l.trim().to_string())
    }

    /// Compiles like [`Compiler::compile`]. Blocking.
    pub fn compile(&self, tex: &str, assets: &[Asset<'_>]) -> Result<CompileOutput, CompileError> {
        let dir = prepare_build_dir(tex, assets)?;
        let root = dir.path();
        // Never shell escape: documents and templates may be shared. Stop at the first
        // error instead of waiting for input.
        let tex_args = ["-interaction=nonstopmode", "-halt-on-error", "-no-shell-escape"];
        let mut transcript = String::new();
        let mut run = |cmd: &mut Command| run_limited(cmd, root, &mut transcript);
        let ok = match &self.latexmk {
            Some(latexmk) => run(Command::new(latexmk)
                .arg(self.program.latexmk_flag())
                .args(tex_args.iter().map(|a| format!("-latexoption={a}")))
                // -norc: ignore the user's latexmkrc, so every machine builds the same way.
                .args(["-norc", "-bibtex", "-f-", "-quiet"])
                .arg(format!("{MAIN}.tex")))?,
            None => {
                let mut ok = run(Command::new(&self.path).args(tex_args).arg(format!("{MAIN}.tex")))?;
                let aux = std::fs::read_to_string(root.join(format!("{MAIN}.aux"))).unwrap_or_default();
                if ok && aux.contains("\\bibdata")
                    && let Some(bibtex) = &self.bibtex
                {
                    run(Command::new(bibtex).arg(MAIN))?;
                }
                // Twice more: references and the table of contents settle.
                for _ in 0..2 {
                    if ok {
                        ok = run(Command::new(&self.path).args(tex_args).arg(format!("{MAIN}.tex")))?;
                    }
                }
                ok
            }
        };
        finish(root, ok, &transcript)
    }
}

/// Longest an installed engine may run before it is stopped (a stuck document must
/// never freeze the app).
const SYSTEM_TIME_LIMIT: std::time::Duration = std::time::Duration::from_secs(180);

/// Runs a TeX program with no input and a time limit; its output is appended to
/// `transcript`. Returns whether it succeeded.
fn run_limited(cmd: &mut Command, root: &Path, transcript: &mut String) -> Result<bool, CompileError> {
    let out_path = root.join("texloom-run.out");
    let out = std::fs::File::create(&out_path)?;
    let mut child = cmd
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .stdout(out.try_clone()?)
        .stderr(out)
        .spawn()?;
    let started = std::time::Instant::now();
    let ok = loop {
        if let Some(status) = child.try_wait()? {
            break status.success();
        }
        if started.elapsed() > SYSTEM_TIME_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            transcript.push_str(&format!("\nStopped: TeX ran longer than {} s.\n", SYSTEM_TIME_LIMIT.as_secs()));
            break false;
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    };
    transcript.push_str(&std::fs::read_to_string(&out_path).unwrap_or_default());
    Ok(ok)
}

/// Where TeX distributions usually put their programs, besides PATH.
fn tex_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    // macOS (MacTeX) and TeX Live's default install prefix, newest year first.
    dirs.push(PathBuf::from("/Library/TeX/texbin"));
    for root in ["/usr/local/texlive", "C:\\texlive"] {
        if let Ok(years) = std::fs::read_dir(root) {
            let mut years: Vec<PathBuf> = years.flatten().map(|e| e.path().join("bin")).collect();
            years.sort();
            for bin in years.into_iter().rev() {
                if let Ok(platforms) = std::fs::read_dir(&bin) {
                    dirs.extend(platforms.flatten().map(|e| e.path()));
                }
            }
        }
    }
    dirs
}

fn find_tex_binary(name: &str) -> Option<PathBuf> {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    tex_dirs().into_iter().map(|d| d.join(&file)).find(|p| p.is_file())
}

/// A fresh build folder with the main file and the assets written into it.
fn prepare_build_dir(tex: &str, assets: &[Asset<'_>]) -> Result<tempfile::TempDir, CompileError> {
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
    Ok(dir)
}

/// The PDF, or the errors from the log (else from the program's output).
fn finish(root: &Path, ok: bool, output: &str) -> Result<CompileOutput, CompileError> {
    let log = std::fs::read_to_string(root.join(format!("{MAIN}.log"))).unwrap_or_default();
    let pdf_path = root.join(format!("{MAIN}.pdf"));
    if ok && pdf_path.is_file() {
        return Ok(CompileOutput { pdf: std::fs::read(pdf_path)?, log });
    }
    let mut errors = parse_log(&log);
    if errors.is_empty() {
        errors = parse_stderr(output);
    }
    if errors.is_empty() {
        let tail: Vec<&str> = output.lines().rev().take(8).collect();
        let message = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
        errors.push(TexError { file: None, line: None, message: if message.trim().is_empty() { "The TeX run failed".into() } else { message } });
    }
    let full_log = if log.is_empty() { output.to_string() } else { log };
    Err(CompileError::Failed { errors, log: full_log })
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
        let dir = prepare_build_dir(tex, assets)?;
        let root = dir.path();

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

    /// Needs an installed TeX distribution: cargo test -p texloom-compile -- --ignored
    #[test]
    #[ignore]
    fn compiles_with_installed_latex() {
        let tex = "\\documentclass{article}\\begin{document}Hi \\cite{k}\\bibliographystyle{plain}\\bibliography{refs}\\end{document}";
        let bib = b"@book{k, author={A. Author}, title={T}, year={2020}, publisher={P}}";
        for program in LatexProgram::ALL {
            let Some(mut sys) = SystemLatex::find(program) else { continue };
            for latexmk in [true, false] {
                if !latexmk {
                    sys.latexmk = None;
                }
                let out = sys.compile(tex, &[Asset { path: "refs.bib", bytes: bib }]).unwrap();
                assert!(out.pdf.starts_with(b"%PDF"), "{program:?}");
                assert!(!out.log.contains("Citation `k' undefined"), "BibTeX ran ({program:?}, latexmk: {latexmk})");
            }
            let err = sys.compile("\\documentclass{article}\\begin{document}\\nope\\end{document}", &[]).unwrap_err();
            let CompileError::Failed { errors, .. } = err else { panic!() };
            assert_eq!(errors[0].message, "Undefined control sequence.", "{program:?}");
        }
    }

    /// Needs network on first run (Tectonic downloads its package bundle).
    /// Run with: TEXLOOM_TECTONIC=src-tauri/binaries/tectonic-<triple> cargo test -p texloom-compile -- --ignored
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
