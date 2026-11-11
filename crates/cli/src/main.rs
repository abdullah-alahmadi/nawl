//! `nawl-cli`: render Nawl documents without the app.
//!
//!   nawl-cli render  report.loom --template templates/senior-design/template.json -o report.pdf
//!   nawl-cli export  report.loom --template ... --out-dir build/
//!   nawl-cli check   templates/senior-design/template.json

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use nawl_codegen::Generated;
use nawl_core::{has_errors, DocumentFile, Issue, Template, DOC_FORMAT};

#[derive(Parser)]
#[command(version, about = "Render Nawl documents to PDF, export LaTeX, check templates")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile a document to PDF.
    Render {
        document: PathBuf,
        #[arg(short, long)]
        template: PathBuf,
        /// Output PDF (default: the document path with .pdf).
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// tectonic (bundled), or an installed pdflatex, xelatex or lualatex.
        #[arg(short, long, default_value = "tectonic")]
        engine: String,
    },
    /// Write the .tex, .cls and assets to a folder (compiles anywhere, e.g. Overleaf).
    Export {
        document: PathBuf,
        #[arg(short, long)]
        template: PathBuf,
        #[arg(long)]
        out_dir: PathBuf,
        /// Engine the files are written for: tectonic/xelatex, pdflatex or lualatex.
        #[arg(short, long, default_value = "tectonic")]
        engine: String,
    },
    /// Validate a template and print its generated class.
    Check {
        template: PathBuf,
        /// Also print the generated .cls.
        #[arg(long)]
        print_cls: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Render { document, template, output, engine } => {
            let (target, program) = parse_engine(&engine)?;
            let (generated, template_dir, doc_dir) = prepare(&document, &template, target)?;
            let assets = load_assets(&generated, &template_dir, &doc_dir)?;
            let support = generated.support_files();
            let files: Vec<nawl_compile::Asset> = support
                .iter()
                .chain(assets.iter())
                .map(|(p, b)| nawl_compile::Asset { path: p, bytes: b })
                .collect();

            let result = match program {
                None => nawl_compile::Compiler::locate(&tectonic_candidates())?.compile(&generated.tex, &files),
                Some(p) => nawl_compile::SystemLatex::find(p)
                    .with_context(|| format!("{} not found; install TeX Live", p.command()))?
                    .compile(&generated.tex, &files),
            };
            let out = match result {
                Ok(out) => out,
                Err(nawl_compile::CompileError::Failed { errors, .. }) => {
                    for e in &errors {
                        match e.line {
                            Some(l) => eprintln!("tex: line {l}: {}", e.message),
                            None => eprintln!("tex: {}", e.message),
                        }
                    }
                    bail!("LaTeX compilation failed (run `export` to inspect the generated files)");
                }
                Err(e) => return Err(e.into()),
            };
            let output = output.unwrap_or_else(|| document.with_extension("pdf"));
            std::fs::write(&output, out.pdf).with_context(|| format!("writing {}", output.display()))?;
            println!("wrote {}", output.display());
        }
        Command::Export { document, template, out_dir, engine } => {
            let (target, _) = parse_engine(&engine)?;
            let (generated, template_dir, doc_dir) = prepare(&document, &template, target)?;
            let assets = load_assets(&generated, &template_dir, &doc_dir)?;
            std::fs::create_dir_all(&out_dir)?;
            let stem = document.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
            let tex_path = out_dir.join(format!("{stem}.tex"));
            std::fs::write(&tex_path, &generated.tex)?;
            for (rel, bytes) in generated.support_files().iter().chain(assets.iter()) {
                let dest = out_dir.join(rel);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(dest, bytes)?;
            }
            println!("wrote {} and {} to {}", tex_path.display(), generated.cls_file_name(), out_dir.display());
        }
        Command::Check { template, print_cls } => {
            let t = load_template(&template)?;
            if print_cls {
                print!("{}", nawl_codegen::template_to_cls(&t));
            }
            eprintln!("{}: template is valid", template.display());
        }
    }
    Ok(())
}

fn report(issues: &[Issue]) {
    for i in issues {
        eprintln!("{i}");
    }
}

fn load_template(path: &Path) -> Result<Template> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let t: Template = serde_json::from_str(&text).with_context(|| format!("parsing template {}", path.display()))?;
    let issues = t.validate();
    report(&issues);
    if has_errors(&issues) {
        bail!("template {} has errors", path.display());
    }
    Ok(t)
}

/// The generator target and, for installed engines, the program to run.
fn parse_engine(name: &str) -> Result<(nawl_codegen::Engine, Option<nawl_compile::LatexProgram>)> {
    use nawl_codegen::Engine;
    use nawl_compile::LatexProgram;
    Ok(match name {
        "tectonic" => (Engine::Xe, None),
        "pdflatex" => (Engine::Pdf, Some(LatexProgram::PdfLatex)),
        "xelatex" => (Engine::Xe, Some(LatexProgram::XeLatex)),
        "lualatex" => (Engine::Lua, Some(LatexProgram::LuaLatex)),
        other => bail!("unknown engine {other:?} (tectonic, pdflatex, xelatex or lualatex)"),
    })
}

/// Loads, validates and generates. Returns the output and the template's folder.
fn prepare(document: &Path, template: &Path, engine: nawl_codegen::Engine) -> Result<(Generated, PathBuf, PathBuf)> {
    let t = load_template(template)?;
    let text = std::fs::read_to_string(document).with_context(|| format!("reading {}", document.display()))?;
    let file: DocumentFile =
        serde_json::from_str(&text).with_context(|| format!("parsing document {}", document.display()))?;
    if file.format != DOC_FORMAT {
        bail!("{} is not a Nawl document (format {:?})", document.display(), file.format);
    }
    report(&t.check_metadata(&file.document.metadata));
    let dir = template.parent().map(Path::to_path_buf).unwrap_or_default();
    let doc_dir = document.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok((nawl_codegen::generate_for(&t, &file.document, engine), dir, doc_dir))
}

/// Template assets (relative to the template) and figure images (relative to the document).
fn load_assets(g: &Generated, template_dir: &Path, doc_dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let from = |dir: &Path, rel: &String, what: &str| -> Result<(String, Vec<u8>)> {
        let path = dir.join(rel);
        let bytes = std::fs::read(&path).with_context(|| format!("reading {what} {}", path.display()))?;
        Ok((rel.clone(), bytes))
    };
    let mut out = Vec::new();
    for rel in &g.assets {
        out.push(from(template_dir, rel, "template asset")?);
    }
    for rel in &g.figures {
        out.push(from(doc_dir, rel, "figure image")?);
    }
    // Library fonts: from NAWL_FONTS, or the checkout's src-tauri/fonts
    // (node scripts/fetch-fonts.mjs).
    let fonts = std::env::var_os("NAWL_FONTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/fonts"));
    for rel in &g.fonts {
        let (_, bytes) = from(&fonts, &rel.trim_start_matches("fonts/").to_string(), "library font")?;
        out.push((rel.clone(), bytes));
    }
    for rel in &g.kit_files {
        let (_, bytes) = from(template_dir, rel, "class kit file")?;
        let name = nawl_codegen::kit_destination(rel);
        out.push((name, bytes));
    }
    Ok(out)
}

/// In a dev checkout, the sidecar fetched by scripts/fetch-tectonic.sh.
fn tectonic_candidates() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/binaries");
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("tectonic-")))
        .collect()
}
