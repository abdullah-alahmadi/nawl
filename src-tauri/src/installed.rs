//! Journal templates from the installed TeX distribution: every journal or
//! conference class TeX Live lists (TeX Catalogue topic "journalpub") becomes a
//! template in the gallery, built on the class as installed, so nothing is copied.
//! Classes Texloom knows get a tuned title block; others the generic one.

use serde::Serialize;
use texloom_compile::installed::{self, InstalledClass};
use texloom_core::Template;

use crate::library::{journal_template, JournalSpec};

/// How to use a class: a readable name, class options, BibTeX style and title block.
/// Anything left out falls back to the class defaults and the generic title block.
struct Profile {
    class: &'static str,
    name: &'static str,
    options: &'static [&'static str],
    bib: Option<&'static str>,
    title_block: Option<&'static str>,
}

const fn p(class: &'static str, name: &'static str) -> Profile {
    Profile { class, name, options: &[], bib: None, title_block: None }
}

/// A class that sets its own BibTeX style.
const fn own_bib(class: &'static str, name: &'static str) -> Profile {
    Profile { class, name, options: &[], bib: Some(""), title_block: None }
}

const fn recipe(class: &'static str, name: &'static str, options: &'static [&'static str], title_block: &'static str) -> Profile {
    Profile { class, name, options, bib: None, title_block: Some(title_block) }
}

/// AASTeX: one \author with its \affiliation and \email each (AASTeX 7 requires both).
const AASTEX: &str = r"\title{{title}}
{#authors|
}\author{{name}}
\affiliation{{affiliation}}
\email{{email}}{/authors}
\begin{abstract}
{abstract}
\end{abstract}
{?keywords}\keywords{{keywords}}{/keywords}";

/// Oxford University Press: numbered \address entries, \corresp, \abstract.
const OUP: &str = r"\journaltitle{}\DOI{}\copyrightyear{}\pubyear{}\access{}\appnotes{}\def\societylogo{}
\title[{title}]{{title}}
{#authors|
}\author[{affnums}]{{name}}{/authors}
{#affiliations|
}\address[{n}]{{name}}{/affiliations}
{?corresponding_email}\corresp[$\ast$]{Corresponding author. {corresponding_email}}{/corresponding_email}
\abstract{{abstract}}
{?keywords}\keywords{{#keywords|, }{keyword}{/keywords}}{/keywords}
\maketitle";

/// The PracTeX Journal: \abstract{…} before \maketitle.
const PRACJOURN: &str = r"\title{{title}}
\author{{#authors|\texloomand }{name}\textsuperscript{{affnums}}{/authors}}
\abstract{{abstract}}
\maketitle
\begin{center}\small
{#affiliations|\\}\textsuperscript{{n}}{name}{/affiliations}
\end{center}
{?keywords}\noindent\textbf{Keywords:} {keywords}{/keywords}";

/// ACS (achemso): title, authors and affiliations only work in the preamble; the
/// title is printed at \begin{document}; \email marks the corresponding author.
const ACHEMSO: &str = r"\title{{title}}
{#authors|
}\author{{name}}
\affiliation{{affiliation}}{?corresponding}
\email{{email}}{/corresponding}{/authors}
\begin{document}
\begin{abstract}
{abstract}
\end{abstract}";

/// Annals of Mathematics (aomart, like amsart): preamble-only author data, the
/// abstract before \maketitle.
const AOMART: &str = r"\title{{title}}
{#authors|
}\author{{name}}
\address{{affiliation}}{?email}
\email{{email}}{/email}{/authors}
{?keywords}\keyword{{keywords}}{/keywords}
\begin{document}
\begin{abstract}
{abstract}
\end{abstract}
\maketitle";

/// MNRAS: affiliations as numbered lines under the authors, abstract and keywords
/// after \maketitle.
const MNRAS: &str = r"\title{{title}}
\author{{#authors|, }{name}\textsuperscript{{affnums}}{/authors}
\\{#affiliations|\\}$^{{n}}${name}{/affiliations}}
\date{}
\maketitle
\begin{abstract}
{abstract}
\end{abstract}
{?keywords}\begin{keywords}
{keywords}
\end{keywords}{/keywords}";

/// Classes Texloom knows. Also looked up by name when TeX has no package database
/// (MiKTeX), so these show up there too.
const PROFILES: &[Profile] = &[
    recipe("mnras", "Monthly Notices of the RAS", &["usenatbib"], MNRAS),
    p("nature", "Nature"),
    Profile { class: "achemso", name: "ACS (American Chemical Society)", options: &[], bib: Some(""), title_block: Some(ACHEMSO) },
    recipe("aomart", "Annals of Mathematics", &[], AOMART),
    p("cas-sc", "Elsevier CAS, single column"),
    p("cas-dc", "Elsevier CAS, double column"),
    recipe("oup-authoring-template", "Oxford University Press journals", &["unnumsec", "webpdf", "contemporary", "large"], OUP),
    recipe("aastex701", "AAS Journals (AASTeX 7)", &[], AASTEX),
    p("aiaa-tc", "AIAA Conference paper"),
    own_bib("ascelike", "ASCE Journals"),
    own_bib("jmlr", "Journal of Machine Learning Research"),
    own_bib("elbioimp", "Journal of Electrical Bioimpedance"),
    own_bib("resphilosophica", "Res Philosophica"),
    // It sets \pdfoutput=0 to catch arXiv submissions missing \pdfoutput=1.
    // quantum.bst stops on references without a DOI or URL; plainnat is the class's
    // own fallback.
    Profile { class: "quantumarticle", name: "Quantum", options: &["a4paper", "onecolumn", "nopdfoutputerror"], bib: Some("plainnat"), title_block: None },
    p("lni", "Lecture Notes in Informatics (GI)"),
    recipe("pracjourn", "The PracTeX Journal", &[], PRACJOURN),
    p("apa7", "APA 7th edition"),
    p("apa6", "APA 6th edition"),
    p("revtex4-2", "REVTeX 4.2 (APS, AIP)"),
    p("csbulletin", "CSTUG Bulletin (Zpravodaj)"),
    p("delvierre", "Delvierre"),
    p("journal", "Scholarly journal article (paper bundle)"),
    p("paper", "Scholarly paper (paper bundle)"),
    p("ltugboat", "TUGboat"),
    p("smfart", "Société Mathématique de France"),
    p("sugconf", "SAS User Group conference"),
    p("sageep", "SAGEEP (EEGS annual meeting)"),
    p("SFEE", "Smart Factory and Energy Efficiency (SFEE)"),
    p("IEEEtran", "IEEE (IEEEtran)"),
    p("elsarticle", "Elsevier (elsarticle)"),
    p("llncs", "Springer LNCS"),
    p("acmart", "ACM (acmart)"),
];

/// Classes in the catalogue Texloom doesn't offer: books, issue and cover classes and
/// superseded versions; classes that need fonts or an engine of their own; and
/// classes whose title commands Texloom can't write yet (each fails on the sample
/// article; see `installed_journal_templates_compile`).
const SKIP: &[&str] = &[
    // Not for an article, or superseded
    "acmconf", "apa", "apa6e", "csbulobalka", "csbulv1", "emulateapj", "fhjbook", "gzt", "ijdc-v9", "jmlrbook",
    "ltugproc", "nwejm", "nwejm-2024-02-08", "quantumview", "revtex4", "revtex4-1", "schwalbe", "smfbook",
    // Particular fonts, XeLaTeX or LuaLaTeX only
    "aalok", "deutschesmuseum", "dtk", "ijsra", "phimisci",
    // Front matter Texloom can't write yet
    "aguplus", "articleingud", "asmejour", "beilstein", "cascadilla", "cc", "cje", "cjs-rcs-article", "ejpecp",
    "fhjpaper", "gammas", "gztarticle", "iacrj", "idcc", "ijdc-v14", "jpsj2", "lps", "matapli", "nrc1", "nrc2",
];

/// Classes that compile with pdfLaTeX only (not XeLaTeX or LuaLaTeX): 8-bit input
/// or fonts that only pdfTeX finds.
const PDFLATEX_ONLY: &[&str] = &["cidarticle", "lni", "nwejmart", "sugconf"];

/// A journal template offered by the installed TeX, for the gallery.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledTemplate {
    pub class: String,
    pub name: String,
    pub description: String,
    pub package: String,
    /// Fails under XeLaTeX and LuaLaTeX (and Tectonic).
    pub pdflatex_only: bool,
}

fn profile(class: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.class == class)
}

/// What the installed TeX offers, by name. Blocking (runs kpsewhich, reads the
/// package database): about a tenth of a second.
pub fn find() -> Vec<InstalledClass> {
    let mut found = installed::journal_classes();
    if found.is_empty() {
        // No package database: look for the classes Texloom knows.
        let files: Vec<String> = PROFILES.iter().map(|p| format!("{}.cls", p.class)).collect();
        let names: Vec<&str> = files.iter().map(String::as_str).collect();
        found = installed::find_files(&names)
            .into_iter()
            .map(|(file, _)| {
                let class = file.trim_end_matches(".cls").to_string();
                InstalledClass { package: class.clone(), description: String::new(), bib_styles: Vec::new(), class }
            })
            .collect();
    }
    found.retain(|c| !SKIP.contains(&c.class.as_str()));
    found
}

pub fn list(found: &[InstalledClass]) -> Vec<InstalledTemplate> {
    let mut out: Vec<InstalledTemplate> = found
        .iter()
        .map(|c| InstalledTemplate {
            class: c.class.clone(),
            name: display_name(c),
            description: if c.description.is_empty() { format!("Class {} from your TeX installation", c.class) } else { c.description.clone() },
            package: c.package.clone(),
            pdflatex_only: PDFLATEX_ONLY.contains(&c.class.as_str()),
        })
        .collect();
    out.sort_by_key(|t| t.name.to_lowercase());
    out
}

fn display_name(c: &InstalledClass) -> String {
    match profile(&c.class) {
        Some(p) => p.name.to_string(),
        None => name_from_description(&c.description).unwrap_or_else(|| c.class.clone()),
    }
}

/// A journal name from a catalogue description: the quoted title if there is one
/// ("Template for the GAMM Archive…" stays, "…the journal 'Aalok'" → "Aalok"),
/// else the description without "A LaTeX class for…" and the like.
fn name_from_description(desc: &str) -> Option<String> {
    for q in ['"', '\'', '“'] {
        let close = if q == '“' { '”' } else { q };
        if let Some(start) = desc.find(q) {
            let rest = &desc[start + q.len_utf8()..];
            if let Some(end) = rest.find(close).filter(|&e| e > 2) {
                return Some(rest[..end].to_string());
            }
        }
    }
    const LEADS: &[&str] = &[
        "a latex document class for ", "latex document class for ", "a latex class for ", "latex class for ",
        "a document class for ", "document class for ", "a class to typeset ", "a class for ", "class for ",
        "typeset articles for ", "typeset papers for ", "format papers for ", "prepare papers for ", "a template for ",
        "template for ", "support for ", "macros for ", "styles for ", "typeset ",
    ];
    let lower = desc.to_lowercase();
    let mut s = LEADS.iter().find(|l| lower.starts_with(*l)).map_or(desc, |l| &desc[l.len()..]);
    for article in ["the journal ", "the "] {
        if s.to_lowercase().starts_with(article) {
            s = &s[article.len()..];
        }
    }
    let s = s.trim();
    if s.is_empty() || s.len() > 60 {
        return None;
    }
    let mut chars = s.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect())
}

/// The template for an installed class.
pub fn template(c: &InstalledClass) -> Template {
    let prof = profile(&c.class);
    let bib_style = prof
        .and_then(|p| p.bib)
        .map(str::to_string)
        .or_else(|| match c.bib_styles.as_slice() {
            [one] => Some(one.clone()),
            many => many.iter().find(|s| **s == c.class || **s == c.package).cloned(),
        })
        .unwrap_or_else(|| "plain".into());
    journal_template(JournalSpec {
        // Not the class's own name: the generated wrapper class is called this.
        id: format!("tl-{}", c.class.to_lowercase().replace(|ch: char| !ch.is_ascii_alphanumeric(), "-")),
        name: display_name(c),
        description: format!(
            "{} — class {} from your TeX installation (package {})",
            if c.description.is_empty() { "Journal template" } else { &c.description },
            c.class,
            c.package
        ),
        class: c.class.clone(),
        options: prof.map(|p| p.options.iter().map(|o| o.to_string()).collect()).unwrap_or_default(),
        bib_style,
        files: Vec::new(),
        title_block: prof.and_then(|p| p.title_block).map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use texloom_compile::{Asset, LatexProgram, SystemLatex};
    use texloom_core::DocumentFile;

    #[test]
    fn templates_are_valid_for_any_class() {
        let c = InstalledClass { class: "someclass".into(), package: "somepkg".into(), description: "Some journal".into(), bib_styles: vec!["a".into(), "somepkg".into()] };
        let t = template(&c);
        assert!(t.validate().is_empty(), "{:?}", t.validate());
        let base = t.base.unwrap();
        assert_eq!((base.class.as_str(), base.bib_style.as_str()), ("someclass", "somepkg"));
        assert!(base.title_block.is_some());
        assert_eq!(t.meta.id, "tl-someclass");
    }

    #[test]
    fn names_come_from_catalogue_descriptions() {
        let n = |d: &str| name_from_description(d);
        assert_eq!(n("A class to typeset La Gaceta de la RSME").as_deref(), Some("La Gaceta de la RSME"));
        assert_eq!(n("LaTeX class file for the Marathi journal 'Aalok'").as_deref(), Some("Aalok"));
        assert_eq!(n("Typeset articles for the journal Res Philosophica").as_deref(), Some("Res Philosophica"));
        assert_eq!(n("Monthly Notices of the Royal Astronomical Society").as_deref(), Some("Monthly Notices of the Royal Astronomical Society"));
        assert_eq!(n("Support for submissions to the \"Beilstein Journal of Nanotechnology\"").as_deref(), Some("Beilstein Journal of Nanotechnology"));
        assert_eq!(n(""), None);
    }

    /// Compiles the sample article with every installed journal class and prints
    /// what fails. Run with: cargo test -p texloom installed -- --ignored --nocapture
    /// (TEXLOOM_ENGINE=xelatex|lualatex to try another engine; TEXLOOM_ONLY=class).
    #[test]
    #[ignore]
    fn installed_journal_templates_compile() {
        // TEXLOOM_ENGINE=tectonic needs TEXLOOM_TECTONIC (the binary's path).
        let choice = std::env::var("TEXLOOM_ENGINE").unwrap_or_default();
        let program = match choice.as_str() {
            "xelatex" | "tectonic" => LatexProgram::XeLatex,
            "lualatex" => LatexProgram::LuaLatex,
            _ => LatexProgram::PdfLatex,
        };
        let engine = match program {
            LatexProgram::PdfLatex => texloom_codegen::Engine::Pdf,
            LatexProgram::XeLatex => texloom_codegen::Engine::Xe,
            LatexProgram::LuaLatex => texloom_codegen::Engine::Lua,
        };
        let system = SystemLatex::find(program).expect("installed TeX");
        let tectonic = (choice == "tectonic").then(|| texloom_compile::Compiler::locate(&[]).expect("tectonic"));
        let compile = |tex: &str, assets: &[Asset]| match &tectonic {
            Some(t) => t.compile(tex, assets),
            None => system.compile(tex, assets),
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let file: DocumentFile =
            serde_json::from_str(&std::fs::read_to_string(root.join("examples/journal-article.loom")).unwrap()).unwrap();
        let only = std::env::var("TEXLOOM_ONLY").ok();
        let mut failed = Vec::new();
        let found = find();
        for c in found.iter().filter(|c| only.as_deref().is_none_or(|o| o == c.class)) {
            let g = texloom_codegen::generate_for(&template(c), &file.document, engine);
            let figures: Vec<(String, Vec<u8>)> =
                g.figures.iter().map(|f| (f.clone(), std::fs::read(root.join("examples").join(f)).unwrap())).collect();
            let support = g.support_files();
            let assets: Vec<Asset> = support.iter().chain(figures.iter()).map(|(p, b)| Asset { path: p, bytes: b }).collect();
            if let Some(dir) = std::env::var_os("TEXLOOM_DUMP") {
                std::fs::write(Path::new(&dir).join(format!("{}.tex", c.class)), &g.tex).unwrap();
                std::fs::write(Path::new(&dir).join(g.cls_file_name()), &g.cls).unwrap();
            }
            match compile(&g.tex, &assets) {
                Ok(_) => println!("ok    {:24} {}", c.class, display_name(c)),
                Err(texloom_compile::CompileError::Failed { errors, .. }) => {
                    let first = errors.first().map(|e| format!("l.{:?} {}", e.line, e.message.lines().next().unwrap_or(""))).unwrap_or_default();
                    println!("FAIL  {:24} {}", c.class, first);
                    failed.push(c.class.clone());
                }
                Err(e) => {
                    println!("FAIL  {:24} {e}", c.class);
                    failed.push(c.class.clone());
                }
            }
        }
        println!("{} of {} compiled; failed: {failed:?}", found.len() - failed.len(), found.len());
    }
}
