//! The user's template library ("My templates"): templates imported into the app's
//! data folder, one folder each (`<id>/template.json` plus its `assets/`).
//!
//! Import accepts a Texloom template (`.json`), a shared template package (a `.zip`
//! holding `template.json`), or a publisher's LaTeX kit (any other `.zip`), from
//! which a journal template is created around the kit's class.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use texloom_core::{ClassBase, FieldKind, FrontMatterStyle, MetadataField, Template};

use crate::{files, templates, AppError};

/// Title block for templates made from a publisher kit: standard LaTeX commands that
/// almost every class understands. Adjustable under Design template → Publisher class.
const KIT_TITLE_BLOCK: &str = r"\title{{title}}
\author{{#authors|\texloomand }{name}\textsuperscript{{affnums}}{/authors}}
\date{}
\maketitle
\begin{center}\small
{#affiliations|\\}\textsuperscript{{n}}{name}{/affiliations}{?corresponding_email}\\Corresponding author: {corresponding_email}{/corresponding_email}
\end{center}
\begin{abstract}
{abstract}
\end{abstract}
{?keywords}\noindent\textbf{Keywords:} {keywords}{/keywords}";

/// Largest file taken from a template package.
const MAX_PACKAGE_FILE: u64 = 50 * 1024 * 1024;

/// How Texloom writes the title block for a publisher class it knows: the style,
/// whether author biographies are printed, and fields the class needs beyond title,
/// authors, abstract and keywords. Unknown classes get the generic recipe.
struct KnownClass {
    style: FrontMatterStyle,
    label: &'static str,
    biographies: bool,
    extra_fields: &'static [(&'static str, &'static str, FieldKind)],
}

fn known_class(class: &str) -> Option<KnownClass> {
    let k = |style, label, biographies, extra_fields| Some(KnownClass { style, label, biographies, extra_fields });
    match class {
        // ieeeaccess's \maketitle needs \doi, \history… which the generic recipe lacks.
        "ieeeaccess" => k(
            FrontMatterStyle::IeeeAccess,
            "IEEE Access",
            true,
            &[("funding", "Funding", FieldKind::Paragraphs), ("doi", "DOI", FieldKind::Text), ("history", "History (received, accepted…)", FieldKind::Text)],
        ),
        "IEEEtran" => k(FrontMatterStyle::IeeeJournal, "IEEE", true, &[]),
        "elsarticle" => k(FrontMatterStyle::Elsevier, "Elsevier", false, &[]),
        "llncs" => k(FrontMatterStyle::Lncs, "Springer LNCS", false, &[]),
        "acmart" => k(FrontMatterStyle::Acm, "ACM", false, &[]),
        c if c.starts_with("revtex") => k(FrontMatterStyle::Revtex, "REVTeX", false, &[]),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Publisher class it's built on, if any.
    pub class: Option<String>,
    /// The template file.
    pub path: String,
}

pub struct Library {
    dir: PathBuf,
}

impl Library {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Templates in the library, by name.
    pub fn list(&self) -> Vec<LibraryEntry> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut out: Vec<LibraryEntry> = entries
            .flatten()
            .filter_map(|e| {
                let path = e.path().join("template.json");
                let t: Template = serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
                Some(LibraryEntry {
                    id: e.file_name().to_string_lossy().into_owned(),
                    name: t.meta.name,
                    description: t.meta.description,
                    class: t.base.map(|b| b.class),
                    path: path.display().to_string(),
                })
            })
            .collect();
        out.sort_by_key(|e| e.name.to_lowercase());
        out
    }

    /// Copies a template, package or kit into the library; returns the new template
    /// file. Nothing is left behind when it fails.
    pub fn import(&self, source: &Path) -> Result<PathBuf, AppError> {
        let ext = source.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
        let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("Template").to_string();
        let (folder, result) = match ext.as_str() {
            "json" => {
                let template = read_template(&std::fs::read(source)?)?;
                let folder = self.new_folder(&template.meta.name)?;
                let r = import_json(source, &folder);
                (folder, r)
            }
            "zip" => match package_root(source)? {
                Some(root) => {
                    let folder = self.new_folder(&stem)?;
                    let r = import_package(source, &root, &folder);
                    (folder, r)
                }
                None => {
                    let name = pretty_name(&stem);
                    let folder = self.new_folder(&name)?;
                    let r = create_from_kit(source, &name, &folder);
                    (folder, r)
                }
            },
            _ => return Err(AppError::other("Choose a Texloom template (.json), a template package or a publisher's LaTeX kit (.zip)")),
        };
        match result {
            Ok(()) => Ok(folder.join("template.json")),
            Err(e) => {
                let _ = std::fs::remove_dir_all(&folder);
                Err(e)
            }
        }
    }

    /// Removes a template from the library (only folders inside the library).
    pub fn remove(&self, id: &str) -> Result<(), AppError> {
        if !files::is_safe_relative(id) || id.contains('/') {
            return Err(AppError::other("Not a library template"));
        }
        let folder = self.dir.join(id);
        if folder.join("template.json").is_file() {
            std::fs::remove_dir_all(folder)?;
        }
        Ok(())
    }

    /// A new, empty folder named after `name` ("my-journal", "my-journal-2"…).
    fn new_folder(&self, name: &str) -> Result<PathBuf, AppError> {
        std::fs::create_dir_all(&self.dir)?;
        let base = slug(name);
        let mut n = 1;
        loop {
            let id = if n == 1 { base.clone() } else { format!("{base}-{n}") };
            let folder = self.dir.join(&id);
            if !folder.exists() {
                std::fs::create_dir(&folder)?;
                return Ok(folder);
            }
            n += 1;
        }
    }
}

fn read_template(bytes: &[u8]) -> Result<Template, AppError> {
    serde_json::from_slice(bytes).map_err(|e| AppError::other(format!("Not a Texloom template: {e}")))
}

/// A Texloom template file, with the `assets/` folder next to it if there is one.
fn import_json(source: &Path, folder: &Path) -> Result<(), AppError> {
    std::fs::copy(source, folder.join("template.json"))?;
    if let Some(assets) = source.parent().map(|d| d.join("assets")).filter(|d| d.is_dir()) {
        copy_dir(&assets, &folder.join("assets"))?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let (src, dest) = (entry.path(), to.join(entry.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dest)?;
        } else {
            std::fs::copy(&src, &dest)?;
        }
    }
    Ok(())
}

/// The folder inside a zip that holds `template.json` ("" for the top level), if any.
fn package_root(zip_path: &Path) -> Result<Option<String>, AppError> {
    let mut archive = open_zip(zip_path)?;
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().and_then(|f| f.name().ok().map(|n| n.to_string())))
        .collect();
    let root = names
        .iter()
        .filter(|n| *n == "template.json" || n.ends_with("/template.json"))
        .min_by_key(|n| n.len())
        .map(|n| n.trim_end_matches("template.json").to_string());
    Ok(root)
}

fn open_zip(path: &Path) -> Result<zip::ZipArchive<std::fs::File>, AppError> {
    zip::ZipArchive::new(std::fs::File::open(path)?).map_err(|e| AppError::other(format!("Not a zip file: {e}")))
}

/// A shared template package: `template.json` and its `assets/`, nothing else.
fn import_package(zip_path: &Path, root: &str, folder: &Path) -> Result<(), AppError> {
    let mut archive = open_zip(zip_path)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(AppError::other)?;
        let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
        let Some(rel) = name.strip_prefix(root) else { continue };
        let wanted = rel == "template.json" || rel.starts_with("assets/");
        if entry.is_dir() || !wanted || !files::is_safe_relative(rel) || entry.size() > MAX_PACKAGE_FILE {
            continue;
        }
        let dest = folder.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut bytes = Vec::new();
        (&mut entry).take(MAX_PACKAGE_FILE).read_to_end(&mut bytes)?;
        if rel == "template.json" {
            read_template(&bytes)?;
        }
        std::fs::write(dest, bytes)?;
    }
    Ok(())
}

/// A journal template around a publisher kit's class: the kit's files, the class and
/// options its sample paper uses, its BibTeX style, and a standard title block.
fn create_from_kit(zip_path: &Path, name: &str, folder: &Path) -> Result<(), AppError> {
    let template_file = folder.join("template.json");
    let imported = files::import_kit_zip(&template_file, zip_path)?;
    let stems = |ext: &str| -> Vec<String> {
        imported
            .iter()
            .filter(|f| f.ends_with(ext))
            .filter_map(|f| Path::new(f).file_stem().and_then(|s| s.to_str()).map(str::to_string))
            .collect()
    };
    let (classes, styles) = (stems(".cls"), stems(".bst"));
    let hints = kit_hints(zip_path)?;
    let (class, options) = match &hints.class {
        Some((opts, c)) if classes.contains(c) => (c.clone(), opts.clone()),
        _ if classes.len() == 1 => (classes[0].clone(), Vec::new()),
        _ if classes.is_empty() => return Err(AppError::other("That kit has no LaTeX class (.cls) file")),
        _ => {
            return Err(AppError::other(format!(
                "That kit has several classes ({}); import the template from a .zip with just one",
                classes.join(", ")
            )))
        }
    };
    let bib_style = match &hints.bib_style {
        Some(s) if styles.contains(s) => s.clone(),
        _ if styles.len() == 1 => styles[0].clone(),
        _ => "plain".to_string(),
    };

    let mut id = slug(name);
    // The generated wrapper class must not have the kit class's own name.
    if classes.contains(&id) {
        id.push_str("-texloom");
    }
    let kit_name = zip_path.file_name().and_then(|n| n.to_str()).unwrap_or("a publisher kit");
    let label = known_class(&class).map(|k| k.label);
    let description = match label {
        Some(l) => format!("Made from {kit_name} (class {class}, {l} title block)"),
        None => format!("Made from {kit_name} (class {class})"),
    };
    let t = journal_template(JournalSpec { id, name: name.to_string(), description, class, options, bib_style, files: imported, title_block: None });
    let mut f = std::fs::File::create(&template_file)?;
    f.write_all(serde_json::to_string_pretty(&t).map_err(AppError::other)?.as_bytes())?;
    Ok(())
}

/// What a journal template is made of: a publisher class and how to use it.
pub struct JournalSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub class: String,
    pub options: Vec<String>,
    pub bib_style: String,
    /// Kit files copied next to the document (empty for classes TeX already has).
    pub files: Vec<String>,
    /// Title block for classes without a built-in style; `None` for the generic one.
    pub title_block: Option<String>,
}

/// A journal template around a publisher class. Fields (title, authors, abstract,
/// keywords) and body settings come from the Elsevier template; classes Texloom
/// knows get their own title block style, others the given or the generic recipe.
pub fn journal_template(spec: JournalSpec) -> Template {
    let (_, base_json) = templates::BUILTIN_TEMPLATES
        .iter()
        .find(|(id, _)| *id == "elsevier-article")
        .expect("elsevier template exists");
    let mut t: Template = serde_json::from_str(base_json).expect("built-in template parses");
    t.meta.id = spec.id;
    t.meta.name = spec.name;
    t.meta.description = spec.description;
    t.meta.version = "0.1.0".into();
    t.meta.author = None;
    let known = known_class(&spec.class);
    if let Some(k) = &known {
        for (id, label, kind) in k.extra_fields {
            if !t.fields.iter().any(|f| f.id == *id) {
                t.fields.push(MetadataField { id: (*id).into(), label: (*label).into(), kind: *kind, required: false });
            }
        }
    }
    t.base = Some(ClassBase {
        class: spec.class,
        options: spec.options,
        front_matter: known.as_ref().map_or(FrontMatterStyle::Custom, |k| k.style),
        bib_style: spec.bib_style,
        files: spec.files,
        biographies: known.as_ref().is_some_and(|k| k.biographies),
        title_block: known.is_none().then(|| spec.title_block.unwrap_or_else(|| KIT_TITLE_BLOCK.into())),
        end_block: None,
        kit: None,
    });
    t
}

#[derive(Default)]
struct KitHints {
    /// `\documentclass[options]{class}` of the kit's sample paper.
    class: Option<(Vec<String>, String)>,
    /// `\bibliographystyle{style}` of the sample paper.
    bib_style: Option<String>,
}

/// What the kit's sample `.tex` uses (the first sample that says).
fn kit_hints(zip_path: &Path) -> Result<KitHints, AppError> {
    let mut archive = open_zip(zip_path)?;
    let mut hints = KitHints::default();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(AppError::other)?;
        if !entry.name().is_ok_and(|n| n.to_lowercase().ends_with(".tex")) || entry.size() > 2 * 1024 * 1024 {
            continue;
        }
        let mut text = String::new();
        if entry.read_to_string(&mut text).is_err() {
            continue;
        }
        let code: String = text.lines().map(|l| l.split('%').next().unwrap_or("")).collect::<Vec<_>>().join("\n");
        if hints.class.is_none() {
            hints.class = command_arg(&code, "\\documentclass").map(|(opts, arg)| {
                let opts = opts.map(|o| o.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
                (opts, arg)
            });
        }
        if hints.bib_style.is_none() {
            hints.bib_style = command_arg(&code, "\\bibliographystyle").map(|(_, arg)| arg);
        }
    }
    Ok(hints)
}

/// `\cmd[opts]{arg}` → (opts, arg).
fn command_arg(code: &str, cmd: &str) -> Option<(Option<String>, String)> {
    let rest = &code[code.find(cmd)? + cmd.len()..];
    let rest = rest.trim_start();
    let (opts, rest) = if let Some(r) = rest.strip_prefix('[') {
        let end = r.find(']')?;
        (Some(r[..end].to_string()), r[end + 1..].trim_start())
    } else {
        (None, rest)
    };
    let r = rest.strip_prefix('{')?;
    let arg = r[..r.find('}')?].trim().to_string();
    (!arg.is_empty()).then_some((opts, arg))
}

/// "my_journal-kit" → "My journal kit".
fn pretty_name(stem: &str) -> String {
    let s: String = stem.replace(['_', '-'], " ").split_whitespace().collect::<Vec<_>>().join(" ");
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().chain(c).collect(),
        None => "Journal template".into(),
    }
}

/// Lowercase id from a name ("My Journal 2" → "my-journal-2").
pub(crate) fn slug(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let s = if s.is_empty() { "template".to_string() } else { s };
    // Ids start with a letter (they're also LaTeX class names).
    if s.starts_with(|c: char| c.is_ascii_digit()) { format!("t-{s}") } else { s }
}

/// Writes the template and its assets as a shareable package.
pub fn export_package(template: &Template, template_dir: Option<&Path>, dest: &Path) -> Result<(), AppError> {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(dest)?);
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file("template.json", opts).map_err(AppError::other)?;
    zip.write_all(serde_json::to_string_pretty(template).map_err(AppError::other)?.as_bytes())?;
    if let Some(assets) = template_dir.map(|d| d.join("assets")).filter(|d| d.is_dir()) {
        add_dir(&mut zip, &assets, "assets", opts)?;
    }
    zip.finish().map_err(AppError::other)?;
    Ok(())
}

fn add_dir(zip: &mut zip::ZipWriter<std::fs::File>, dir: &Path, prefix: &str, opts: zip::write::SimpleFileOptions) -> Result<(), AppError> {
    for entry in std::fs::read_dir(dir)?.flatten() {
        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        if entry.path().is_dir() {
            add_dir(zip, &entry.path(), &name, opts)?;
        } else {
            zip.start_file(name, opts).map_err(AppError::other)?;
            zip.write_all(&std::fs::read(entry.path())?)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_zip(path: &Path, files: &[(&str, &str)]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, body) in files {
            z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(body.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("texloom-lib-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn kit_becomes_a_journal_template() {
        let root = temp("kit");
        let zip = root.join("Cool_Journal-kit.zip");
        write_zip(
            &zip,
            &[
                ("kit/cooljournal.cls", "\\ProvidesClass{cooljournal}"),
                ("kit/cooljournal.bst", "bst"),
                ("kit/other.bst", "bst"),
                ("kit/sample.tex", "% \\documentclass{wrong}\n\\documentclass[twocolumn, final]{cooljournal}\n\\bibliographystyle{cooljournal}"),
                ("kit/sample.pdf", "pdf"),
            ],
        );
        let lib = Library::new(root.join("library"));
        let file = lib.import(&zip).unwrap();
        let t: Template = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!(t.meta.name, "Cool Journal kit");
        assert_eq!(t.meta.id, "cool-journal-kit");
        let base = t.base.unwrap();
        assert_eq!((base.class.as_str(), base.options.clone()), ("cooljournal", vec!["twocolumn".to_string(), "final".to_string()]));
        assert_eq!(base.bib_style, "cooljournal");
        assert_eq!(base.front_matter, FrontMatterStyle::Custom);
        assert!(base.files.contains(&"assets/cooljournal.cls".to_string()));
        assert!(templates::load_file(&file).is_ok(), "the new template is valid");
        assert_eq!(lib.list().len(), 1);

        // A second import of the same kit gets its own folder.
        let second = lib.import(&zip).unwrap();
        assert!(second.parent().unwrap().ends_with("cool-journal-kit-2"));
        lib.remove("cool-journal-kit-2").unwrap();
        assert_eq!(lib.list().len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn known_classes_get_their_own_title_block() {
        let root = temp("known");
        let zip = root.join("ACCESS_latex_template.zip");
        write_zip(
            &zip,
            &[
                ("k/ieeeaccess.cls", "cls"),
                ("k/IEEEtran.cls", "cls"),
                ("k/IEEEtran.bst", "bst"),
                ("k/access.tex", "\\documentclass{ieeeaccess}\n\\bibliographystyle{IEEEtran}"),
            ],
        );
        let file = Library::new(root.join("library")).import(&zip).unwrap();
        let t: Template = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        let base = t.base.unwrap();
        assert_eq!(base.class, "ieeeaccess", "the sample's class wins over IEEEtran.cls");
        assert_eq!(base.front_matter, FrontMatterStyle::IeeeAccess);
        assert!(base.biographies && base.title_block.is_none());
        assert!(t.fields.iter().any(|f| f.id == "doi") && t.fields.iter().any(|f| f.id == "history"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn packages_round_trip_and_bad_imports_leave_nothing() {
        let root = temp("pkg");
        let lib = Library::new(root.join("library"));
        let mut t: Template = serde_json::from_str(templates::BUILTIN_TEMPLATES[0].1).unwrap();
        t.meta.name = "Shared Report".into();
        std::fs::create_dir_all(root.join("src/assets")).unwrap();
        std::fs::write(root.join("src/assets/logo.png"), b"png").unwrap();
        export_package(&t, Some(&root.join("src")), &root.join("shared.zip")).unwrap();

        let file = lib.import(&root.join("shared.zip")).unwrap();
        assert_eq!(std::fs::read(file.parent().unwrap().join("assets/logo.png")).unwrap(), b"png");
        assert_eq!(lib.list()[0].name, "Shared Report");

        write_zip(&root.join("nothing.zip"), &[("readme.txt", "hi")]);
        assert!(lib.import(&root.join("nothing.zip")).is_err());
        assert_eq!(lib.list().len(), 1, "a failed import leaves no folder");
        assert!(lib.remove("../src").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Leaves a kit-made template in $TEXLOOM_KIT_OUT for a real compile with the CLI
    /// (from $TEXLOOM_KIT_ZIP when set).
    #[test]
    #[ignore]
    fn kit_template_for_cli() {
        let out = PathBuf::from(std::env::var("TEXLOOM_KIT_OUT").expect("TEXLOOM_KIT_OUT"));
        // A real kit with TEXLOOM_KIT_ZIP, else a tiny class built on article.
        if let Ok(real) = std::env::var("TEXLOOM_KIT_ZIP") {
            let file = Library::new(out.join("library")).import(Path::new(&real)).unwrap();
            println!("TEMPLATE {}", file.display());
            return;
        }
        let zip = out.join("Demo_Journal.zip");
        write_zip(
            &zip,
            &[
                ("demo/demojournal.cls", "\\NeedsTeXFormat{LaTeX2e}\\ProvidesClass{demojournal}\\LoadClass[twocolumn]{article}"),
                ("demo/sample.tex", "\\documentclass[10pt]{demojournal}\n\\bibliographystyle{plain}"),
            ],
        );
        let file = Library::new(out.join("library")).import(&zip).unwrap();
        println!("TEMPLATE {}", file.display());
    }

    #[test]
    fn reads_sample_commands() {
        assert_eq!(command_arg("x \\documentclass [a,b] { j }", "\\documentclass"), Some((Some("a,b".into()), "j".into())));
        assert_eq!(command_arg("\\bibliographystyle{ieee}", "\\bibliographystyle"), Some((None, "ieee".into())));
        assert_eq!(slug("2024 Journal!"), "t-2024-journal");
    }
}
