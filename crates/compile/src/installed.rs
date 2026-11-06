//! What an installed TeX distribution offers: the journal and conference classes
//! listed in TeX Live's package database (packages the TeX Catalogue files under the
//! "journalpub" topic), plus a lookup of any class or style by name with `kpsewhich`.

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

/// The journal and conference classes of the installed TeX Live, by class name.
/// Empty when there is no TeX Live or its package database can't be found (MiKTeX,
/// for instance); use [`find_files`] for known classes then.
pub fn journal_classes() -> Vec<InstalledClass> {
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
        if !r.topics.contains(&"journalpub") {
            continue;
        }
        let (classes, bib_styles) = if r.runfiles.is_empty() {
            (files_in(&dist.join("tex/latex").join(r.name), "cls", 2), files_in(&dist.join("bibtex/bst").join(r.name), "bst", 1))
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
                class,
                package: r.name.to_string(),
                description: r.shortdesc.to_string(),
                bib_styles: bib_styles.clone(),
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

    /// Lists what this computer's TeX Live offers. Run with --ignored --nocapture.
    #[test]
    #[ignore]
    fn lists_installed_journal_classes() {
        let found = journal_classes();
        for c in &found {
            println!("{:28} {:24} {:?} {}", c.class, c.package, c.bib_styles, c.description);
        }
        assert!(!found.is_empty());
    }
}
