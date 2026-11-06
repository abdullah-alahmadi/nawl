//! What an installed TeX distribution offers: the document classes listed in TeX
//! Live's package database, with the TeX Catalogue's topics for each ("journalpub",
//! "dissertation", "book-pub", "letter"…), and a lookup of any file by name.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::find_tex_binary;

/// A journal or conference class found in the installed TeX distribution.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledClass {
    /// Class name, as in `\documentclass{…}`.
    pub class: String,
    /// TeX Live package it comes from.
    pub package: String,
    /// The package's one-line description from the TeX Catalogue.
    pub description: String,
    /// BibTeX styles shipped in the same package.
    pub bib_styles: Vec<String>,
    /// The package's TeX Catalogue topics ("journalpub", "dissertation"…).
    pub topics: Vec<String>,
    /// The class file, when known.
    pub path: Option<PathBuf>,
}

impl InstalledClass {
    /// The class has chapters: it defines `\chapter` or loads a class that does.
    pub fn has_chapters(&self) -> bool {
        let Some(text) = self.path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) else { return false };
        let defines = ["\\newcommand\\chapter", "\\newcommand*\\chapter", "\\newcommand{\\chapter}", "\\newcommand*{\\chapter}", "\\def\\chapter", "\\DeclareRobustCommand\\chapter", "\\DeclareRobustCommand{\\chapter}"];
        if defines.iter().any(|d| text.contains(d)) {
            return true;
        }
        // \LoadClass[…]{book}, {report}, {memoir}, {scrbook}…
        text.match_indices("\\LoadClass").any(|(i, _)| {
            // By characters: class files may hold non-ASCII text (Cyrillic, Chinese…).
            let rest: String = text[i..].chars().take(200).collect();
            let name = rest.split('{').nth(1).and_then(|r| r.split('}').next()).unwrap_or("");
            ["book", "report", "memoir", "scrbook", "scrreprt", "extbook", "extreport", "bookest"].iter().any(|b| name.trim() == *b)
        })
    }
}

/// Runs `kpsewhich` with `args`; its output lines (empty when TeX isn't installed).
fn kpsewhich(args: &[&str]) -> Vec<String> {
    let Some(exe) = find_tex_binary("kpsewhich") else { return Vec::new() };
    let Ok(out) = Command::new(exe).args(args).stdin(std::process::Stdio::null()).output() else { return Vec::new() };
    String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
}

/// Which of `files` (e.g. "mnras.cls") the installed TeX can find, with their paths.
pub fn find_files(files: &[&str]) -> Vec<(String, PathBuf)> {
    if files.is_empty() {
        return Vec::new();
    }
    // `-all` isn't wanted; kpsewhich prints one path per file it finds, in order,
    // skipping missing ones, so match the results back by file name.
    kpsewhich(files)
        .into_iter()
        .filter_map(|p| {
            let path = PathBuf::from(&p);
            let name = path.file_name()?.to_str()?.to_string();
            files.contains(&name.as_str()).then_some((name, path))
        })
        .collect()
}

/// The document classes of the installed TeX Live (packages with the Catalogue topic
/// "class"), by class name. Empty when there is no TeX Live or its package database
/// can't be found (MiKTeX, for instance); use [`find_files`] for known classes then.
pub fn document_classes() -> Vec<InstalledClass> {
    // kpsewhich answers only the last -var-value of a call.
    let var = |name: &str| kpsewhich(&["-var-value", name]).into_iter().next().map(PathBuf::from);
    let (Some(root), Some(dist)) = (var("TEXMFROOT"), var("TEXMFDIST")) else { return Vec::new() };
    // A TeX Live install has the full database (with each package's files); Debian
    // and Ubuntu ship only the catalogue part, so files are looked up on disk.
    let tlpkg = root.join("tlpkg");
    let db = ["texlive.tlpdb", "texlive-catalogue-only.tlpdb"].iter().find_map(|f| std::fs::read_to_string(tlpkg.join(f)).ok());
    match db {
        Some(text) => from_database(&text, &root, &dist),
        None => Vec::new(),
    }
}

/// Beamer themes of the installed TeX distribution, by kind.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BeamerThemes {
    /// `\usetheme{…}` names.
    pub themes: Vec<String>,
    /// `\usecolortheme{…}` names.
    pub color_themes: Vec<String>,
    /// `\usefonttheme{…}` names.
    pub font_themes: Vec<String>,
}

/// Beamer themes found in the installed TeX's `tex/latex` trees (TEXMFDIST and
/// TEXMFLOCAL), from their file names: `beamerthemeX.sty`, `beamercolorthemeX.sty`,
/// `beamerfontthemeX.sty`. Empty without an installed TeX.
pub fn beamer_themes() -> BeamerThemes {
    let mut out = BeamerThemes::default();
    for var in ["TEXMFDIST", "TEXMFLOCAL"] {
        if let Some(root) = kpsewhich(&["-var-value", var]).into_iter().next() {
            collect_themes(&PathBuf::from(root).join("tex/latex"), &mut out, 6);
        }
    }
    for list in [&mut out.themes, &mut out.color_themes, &mut out.font_themes] {
        list.sort_by_key(|n| n.to_lowercase());
        list.dedup();
    }
    out
}

fn collect_themes(dir: &Path, out: &mut BeamerThemes, depth: u8) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            if depth > 0 {
                collect_themes(&path, out, depth - 1);
            }
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let Some(stem) = name.strip_suffix(".sty") else { continue };
        // Names go into \usetheme{…}: keep the plain ones.
        let plain = |n: &str| !n.is_empty() && n.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if let Some(n) = stem.strip_prefix("beamercolortheme") {
            if plain(n) {
                out.color_themes.push(n.to_string());
            }
        } else if let Some(n) = stem.strip_prefix("beamerfonttheme") {
            if plain(n) {
                out.font_themes.push(n.to_string());
            }
        } else if let Some(n) = stem.strip_prefix("beamertheme") {
            if plain(n) {
                out.themes.push(n.to_string());
            }
        }
    }
}

/// One package record of the database.
#[derive(Default)]
struct Record<'a> {
    name: &'a str,
    shortdesc: &'a str,
    topics: Vec<&'a str>,
    runfiles: Vec<&'a str>,
}

fn records(text: &str) -> Vec<Record<'_>> {
    let mut out = Vec::new();
    let mut cur = Record::default();
    let mut in_runfiles = false;
    for line in text.lines() {
        if line.is_empty() {
            if !cur.name.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            in_runfiles = false;
        } else if let Some(file) = line.strip_prefix(' ') {
            if in_runfiles {
                cur.runfiles.push(file.split_whitespace().next().unwrap_or(""));
            }
        } else {
            in_runfiles = line.starts_with("runfiles ");
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            match key {
                "name" => cur.name = value,
                "shortdesc" => cur.shortdesc = value,
                "catalogue-topics" => cur.topics = value.split_whitespace().collect(),
                _ => {}
            }
        }
    }
    if !cur.name.is_empty() {
        out.push(cur);
    }
    out
}

/// Where `class`.cls is: from the package's file list, else in its folder.
fn class_path(root: &Path, dir: &Path, runfiles: &[&str], class: &str) -> Option<PathBuf> {
    let file = format!("{class}.cls");
    if let Some(f) = runfiles.iter().find(|f| f.ends_with(&format!("/{file}"))) {
        return Some(root.join(f));
    }
    fn find(dir: &Path, file: &str, depth: u8) -> Option<PathBuf> {
        for e in std::fs::read_dir(dir).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() && depth > 0 {
                if let Some(found) = find(&p, file, depth - 1) {
                    return Some(found);
                }
            } else if p.file_name().is_some_and(|n| n == file) {
                return Some(p);
            }
        }
        None
    }
    find(dir, &file, 2)
}

fn stem(path: &str, ext: &str) -> Option<String> {
    let p = Path::new(path);
    (p.extension()? == ext).then(|| p.file_stem()?.to_str().map(str::to_string))?
}

/// Files with extension `ext` under `dir` (two levels deep), by stem.
fn files_in(dir: &Path, ext: &str, depth: u8) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() && depth > 0 {
            out.extend(files_in(&path, ext, depth - 1));
        } else if let Some(s) = path.to_str().and_then(|p| stem(p, ext)) {
            out.push(s);
        }
    }
    out.sort();
    out
}

fn from_database(text: &str, root: &Path, dist: &Path) -> Vec<InstalledClass> {
    let mut out = Vec::new();
    for r in records(text) {
        if !r.topics.contains(&"class") {
            continue;
        }
        let class_dir = dist.join("tex/latex").join(r.name);
        let (classes, bib_styles) = if r.runfiles.is_empty() {
            (files_in(&class_dir, "cls", 2), files_in(&dist.join("bibtex/bst").join(r.name), "bst", 1))
        } else {
            // Listed files only count when they're really there.
            let pick = |ext: &str| -> Vec<String> {
                let mut v: Vec<String> =
                    r.runfiles.iter().filter(|f| root.join(f).is_file()).filter_map(|f| stem(f, ext)).collect();
                v.sort();
                v
            };
            (pick("cls"), pick("bst"))
        };
        for class in classes {
            out.push(InstalledClass {
                package: r.name.to_string(),
                description: r.shortdesc.to_string(),
                bib_styles: bib_styles.clone(),
                topics: r.topics.iter().map(|t| t.to_string()).collect(),
                path: class_path(root, &class_dir, &r.runfiles, &class),
                class,
            });
        }
    }
    out.sort_by(|a, b| a.class.to_lowercase().cmp(&b.class.to_lowercase()));
    out.dedup_by(|a, b| a.class == b.class);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_journal_packages_from_a_full_database() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for f in ["texmf-dist/tex/latex/mnras/mnras.cls", "texmf-dist/bibtex/bst/mnras/mnras.bst"] {
            std::fs::create_dir_all(root.join(f).parent().unwrap()).unwrap();
            std::fs::write(root.join(f), "").unwrap();
        }
        let db = "name mnras\ncategory Package\nshortdesc Monthly Notices of the Royal Astronomical Society\n\
runfiles size=10\n texmf-dist/bibtex/bst/mnras/mnras.bst\n texmf-dist/tex/latex/mnras/mnras.cls\n \
texmf-dist/tex/latex/mnras/missing.cls\ncatalogue-topics astronomy class journalpub\n\n\
name beamer\nshortdesc Slides\nrunfiles size=1\n texmf-dist/tex/latex/beamer/beamer.cls\ncatalogue-topics presentation class\n";
        let found = from_database(db, root, &root.join("texmf-dist"));
        assert_eq!(
            found,
            vec![InstalledClass {
                class: "mnras".into(),
                package: "mnras".into(),
                description: "Monthly Notices of the Royal Astronomical Society".into(),
                bib_styles: vec!["mnras".into()],
                topics: vec!["astronomy".into(), "class".into(), "journalpub".into()],
                path: Some(root.join("texmf-dist/tex/latex/mnras/mnras.cls")),
            }]
        );
    }

    #[test]
    fn catalogue_only_database_looks_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let dist = dir.path().join("texmf-dist");
        std::fs::create_dir_all(dist.join("tex/latex/nature")).unwrap();
        std::fs::write(dist.join("tex/latex/nature/nature.cls"), "").unwrap();
        let db = "name nature\nshortdesc Prepare papers for the journal Nature\ncatalogue-topics class journalpub\n\n\
name absent\nshortdesc Not installed\ncatalogue-topics class journalpub\n";
        let found = from_database(db, dir.path(), &dist);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].class, "nature");
        assert!(found[0].bib_styles.is_empty());
    }

    #[test]
    fn finds_beamer_themes_by_file_name() {
        let dir = tempfile::tempdir().unwrap();
        let t = dir.path().join("beamer/themes");
        std::fs::create_dir_all(&t).unwrap();
        for f in ["beamerthemeMadrid.sty", "beamercolorthemebeaver.sty", "beamerfontthemeserif.sty", "beamerinnerthemecircles.sty", "beamertheme-odd.sty", "notes.sty"] {
            std::fs::write(t.join(f), "").unwrap();
        }
        let mut out = BeamerThemes::default();
        collect_themes(dir.path(), &mut out, 3);
        assert_eq!(out.themes, vec!["Madrid"]);
        assert_eq!(out.color_themes, vec!["beaver"]);
        assert_eq!(out.font_themes, vec!["serif"]);
    }

    #[test]
    fn chapters_come_from_the_class_file() {
        let dir = tempfile::tempdir().unwrap();
        let class = |name: &str, text: &str| {
            let path = dir.path().join(format!("{name}.cls"));
            std::fs::write(&path, text).unwrap();
            InstalledClass { class: name.into(), package: name.into(), description: String::new(), bib_styles: vec![], topics: vec![], path: Some(path) }
        };
        assert!(class("a", "% Диссертация, кириллица\n\\LoadClass[12pt]{report}").has_chapters());
        assert!(class("b", "\\newcommand\\chapter{\\section}").has_chapters());
        assert!(!class("c", "% Статья\n\\LoadClass{article}\n\\@ifundefined{chapter}{}{}").has_chapters());
        assert!(!InstalledClass { path: None, ..class("d", "") }.has_chapters());
    }

    /// Lists what this computer's TeX Live offers. Run with --ignored --nocapture.
    #[test]
    #[ignore]
    fn lists_installed_journal_classes() {
        let found = document_classes();
        for c in &found {
            println!("{:28} {:24} {:?} {}", c.class, c.package, c.bib_styles, c.description);
        }
        assert!(!found.is_empty());
    }
}
