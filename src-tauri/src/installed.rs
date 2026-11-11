//! Journal templates from the installed TeX distribution: every journal or
//! conference class TeX Live lists (TeX Catalogue topic "journalpub") becomes a
//! template in the gallery, built on the class as installed, so nothing is copied.
//! Classes Nawl knows get a tuned title block; others the generic one.

use serde::Serialize;
use nawl_compile::installed::{self, InstalledClass};
use nawl_core::{tr, Template};

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

/// Classes Nawl knows. Also looked up by name when TeX has no package database
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
    p("scrartcl", "KOMA-Script article"),
    p("scrreprt", "KOMA-Script report"),
    p("scrbook", "KOMA-Script book"),
    p("memoir", "Memoir (books and reports)"),
    p("extarticle", "Article with more font sizes"),
    p("extreport", "Report with more font sizes"),
    p("extbook", "Book with more font sizes"),
    p("elegantbook", "ElegantBook"),
    p("IEEEtran", "IEEE (IEEEtran)"),
    p("elsarticle", "Elsevier (elsarticle)"),
    p("llncs", "Springer LNCS"),
    p("acmart", "ACM (acmart)"),
];

/// Installed classes that don't compile with Nawl's standard title block and sample
/// document (pdfLaTeX, full TeX Live): they need commands of their own, particular
/// fonts or files (logos, Chinese fonts), another engine, or aren't for documents
/// at all (issues, covers, superseded versions). Found with
/// `installed_journal_templates_compile`; the gallery leaves them out.
const SKIP: &[&str] = &[
    // Journals and conferences
    "aalok", "acmconf", "apa", "apa6e", "articleingud", "asmejour", "cc", "cjs-rcs-article", "csbulobalka",
    "csbulv1", "dtk", "ejpecp", "emulateapj", "fhjpaper", "gammas", "gzt", "gztarticle", "idcc", "IEEEconf",
    "ijdc-v14", "ijdc-v9", "jmlrbook", "lps", "mucproc", "nwejm", "nwejm-2024-02-08", "phimisci", "quantumview",
    "schwalbe",
    // Theses
    "afthesis", "aucklandthesis", "bangorcsthesis", "bitbeamer", "bithesis", "bitreport", "bjfuthesis",
    "cquthesis", "disser", "droit-fr", "easybook", "elteiktdk", "elteikthesis", "fcavtex", "fdudoc",
    "fduthesis", "fduthesis-en", "fei", "gost732", "hduthesis", "hecthese", "hepthesis", "hgbarticle",
    "hgbreport", "hgbthesis", "hitszthesis", "hpsdiss", "hustthesis", "IMTEKda", "infufrgs", "kdgcoursetext",
    "kdgmasterthesis", "ksp-thesis", "maine-thesis", "mcmthesis", "mitthesis", "msu-thesis", "nddiss2e",
    "njurepo", "njuthesis", "njuthesis-doc", "nwafuthesis", "nxuthesis", "pkuthss", "ryersonSGSThesis",
    "sapthesis", "sduthesis", "se2thesis", "seuthesix", "shtthesis", "sjtuarticle", "sjtureport", "sjtuthesis",
    "sysuthesis", "tabriz-thesis", "thesis-gwu", "thuthesis", "tudabeamer", "tudaexercise", "tudaleaflet",
    "tudaletter", "tudaposter", "tudapub", "tudscrartcl", "tudscrbook", "tudscrmanual", "tudscrposter",
    "tudscrreprt", "tui", "ualberta", "uantwerpenbamathesis", "uantwerpenletter", "uantwerpenphdthesis",
    "uantwerpenreport", "ucsmonograph", "udes-genie-these", "udesoftec", "udiss", "ulthese", "unam-thesis",
    "unbtex", "unifith", "univie-ling-handout", "univie-ling-wlg", "xmuthesis", "yathesis",
    // Books
    "beaulivre", "beautybook", "beautynote", "bgteubner", "colorbook", "ctexart", "ctexbeamer", "ctexbook",
    "ctexrep", "ctxdoc", "extletter", "juraurtl", "mentis", "minimbook", "mwart", "nostarch", "octavo", "qyxf-
    book", "scrlttr2", "simplivre", "tufte-book", "tufte-handout", "xtufte-book", "xtufte-handout",
    // Articles and reports
    "amnestyreport", "BMSTU-IU8", "cs-techrep", "einfart", "erdc", "grant", "grant-afosr", "grant-aro", "grant-
    darpa", "grant-doe", "grant-nih", "grant-nrl", "grant-nsf", "grant-onr", "h2020proposal", "hausarbeit-jura",
    "heria", "hitreport", "jlreq", "jwjournal", "korigamik", "lebhart", "mynsfc", "omgtudoc-asoiu", "Q-and-A",
    "spimubephdthesis", "spimumlpphdthesis", "spimutbmphdthesis", "uol-physics-report", "utbmciadreport", "uwa-
    pcf", "uwa-pif",
];

/// Classes that compile but aren't for writing a document of their kind: a package's
/// documentation class, compatibility aliases, covers.
const NOT_DOCUMENTS: &[&str] = &["koma-script-source-doc", "scrarticle", "scrreport", "kdpcover"];

/// Classes that compile with pdfLaTeX only (not XeLaTeX or LuaLaTeX): 8-bit input
/// or fonts that only pdfTeX finds.
const PDFLATEX_ONLY: &[&str] = &["cidarticle", "lni", "nwejmart", "sugconf"];

/// What kind of document an installed class makes, from its TeX Catalogue topics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    /// Journal and conference papers.
    Journal,
    /// Theses and dissertations.
    Thesis,
    Book,
    /// General articles, reports, essays and proposals.
    General,
    // Kinds of document Nawl can't write yet: listed, not offered.
    Letter,
    Cv,
    Poster,
    Exam,
    Presentation,
}

impl Category {
    /// Nawl can make a template around classes of this kind.
    pub fn supported(self) -> bool {
        matches!(self, Category::Journal | Category::Thesis | Category::Book | Category::General)
    }

}

/// The kind of document from the Catalogue topics; `None` for classes that aren't
/// for writing a document of one of these kinds (language support, internals…).
pub fn category(c: &InstalledClass) -> Option<Category> {
    let has = |t: &str| c.topics.iter().any(|x| x == t);
    // The most specific topic wins: a thesis class may also be "article-like".
    Some(if has("journalpub") || has("confproc") {
        Category::Journal
    } else if has("dissertation") {
        Category::Thesis
    } else if has("book-pub") {
        Category::Book
    } else if has("letter") {
        Category::Letter
    } else if has("cv") {
        Category::Cv
    } else if has("poster") {
        Category::Poster
    } else if has("exam") || has("exercise") {
        Category::Exam
    } else if has("presentation") {
        Category::Presentation
    } else if ["article-like", "report-like", "essay", "proposal", "std-conform"].iter().any(|t| has(t)) {
        Category::General
    } else if c.topics.is_empty() {
        // Found without a package database (MiKTeX): one of Nawl's journal classes.
        Category::Journal
    } else {
        return None;
    })
}

/// A template offered by the installed TeX, for the gallery.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledTemplate {
    pub class: String,
    pub name: String,
    pub description: String,
    pub package: String,
    pub category: Category,
    /// Fails under XeLaTeX and LuaLaTeX (and Tectonic).
    pub pdflatex_only: bool,
}

/// Installed classes of a kind Nawl can't write yet, for a note in the gallery.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Waiting {
    pub category: Category,
    pub count: usize,
}

/// What the installed TeX offers: templates by category, and what's waiting.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledList {
    pub templates: Vec<InstalledTemplate>,
    pub waiting: Vec<Waiting>,
}

fn profile(class: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.class == class)
}

/// What the installed TeX offers, by name. Blocking (runs kpsewhich, reads the
/// package database): about a tenth of a second.
pub fn find() -> Vec<InstalledClass> {
    let mut found = installed::document_classes();
    if found.is_empty() {
        // No package database: look for the classes Nawl knows.
        let files: Vec<String> = PROFILES.iter().map(|p| format!("{}.cls", p.class)).collect();
        let names: Vec<&str> = files.iter().map(String::as_str).collect();
        found = installed::find_files(&names)
            .into_iter()
            .map(|(file, path)| {
                let class = file.trim_end_matches(".cls").to_string();
                InstalledClass { package: class.clone(), description: String::new(), bib_styles: Vec::new(), topics: Vec::new(), path: Some(path), class }
            })
            .collect();
    }
    found.retain(|c| category(c).is_some());
    found
}

/// The templates Nawl can offer (supported kinds, minus classes known not to work
/// with it), and how many classes of the other kinds are installed.
pub fn list(found: &[InstalledClass]) -> InstalledList {
    let mut templates: Vec<InstalledTemplate> = found
        .iter()
        .filter_map(|c| Some((c, category(c)?)))
        .filter(|(c, cat)| cat.supported() && !SKIP.contains(&c.class.as_str()) && !NOT_DOCUMENTS.contains(&c.class.as_str()))
        // Letter classes inside packages filed under other kinds (KOMA's scrletter…).
        .filter(|(c, _)| !c.class.contains("letter") && !c.class.contains("lttr"))
        .map(|(c, category)| InstalledTemplate {
            class: c.class.clone(),
            name: display_name(c),
            description: if c.description.is_empty() { tr!("Class {} from your TeX installation", "الصنف {} من تثبيت TeX لديك", c.class) } else { c.description.clone() },
            package: c.package.clone(),
            category,
            pdflatex_only: PDFLATEX_ONLY.contains(&c.class.as_str()),
        })
        .collect();
    // Classes of one package share its description: tell them apart by class name.
    let names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
    for t in &mut templates {
        if names.iter().filter(|n| **n == t.name).count() > 1 {
            t.name = format!("{} ({})", t.name, t.class);
        }
    }
    templates.sort_by_key(|t| t.name.to_lowercase());
    let mut waiting: Vec<Waiting> = Vec::new();
    for c in found {
        // Beamer itself is how Nawl makes presentations.
        if c.package == "beamer" {
            continue;
        }
        if let Some(cat) = category(c).filter(|cat| !cat.supported()) {
            match waiting.iter_mut().find(|w| w.category == cat) {
                Some(w) => w.count += 1,
                None => waiting.push(Waiting { category: cat, count: 1 }),
            }
        }
    }
    InstalledList { templates, waiting }
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
            if c.description.is_empty() { "Template" } else { &c.description },
            c.class,
            c.package
        ),
        class: c.class.clone(),
        options: prof.map(|p| p.options.iter().map(|o| o.to_string()).collect()).unwrap_or_default(),
        bib_style,
        files: Vec::new(),
        title_block: prof.and_then(|p| p.title_block).map(str::to_string),
        // By the class itself: a package filed under books may hold article classes too.
        chapters: category(c).is_some_and(|k| k != Category::Journal) && c.has_chapters(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use nawl_compile::{Asset, LatexProgram, SystemLatex};
    use nawl_core::DocumentFile;

    #[test]
    fn templates_are_valid_for_any_class() {
        let c = InstalledClass { class: "someclass".into(), package: "somepkg".into(), description: "Some journal".into(), bib_styles: vec!["a".into(), "somepkg".into()], topics: vec!["class".into(), "journalpub".into()], path: None };
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

    /// Prints what the gallery offers on this computer. Run with --ignored --nocapture.
    #[test]
    #[ignore]
    fn prints_installed_list() {
        let l = list(&find());
        for t in &l.templates {
            println!("{:?}\t{}\t{}", t.category, t.class, t.name);
        }
        println!("waiting: {:?}", l.waiting.iter().map(|w| (w.category, w.count)).collect::<Vec<_>>());
    }

    /// Compiles the sample article with every installed journal class and prints
    /// what fails. Run with: cargo test -p nawl installed -- --ignored --nocapture
    /// (NAWL_ENGINE=xelatex|lualatex to try another engine; NAWL_ONLY=class).
    #[test]
    #[ignore]
    fn installed_journal_templates_compile() {
        // NAWL_ENGINE=tectonic needs NAWL_TECTONIC (the binary's path).
        let choice = std::env::var("NAWL_ENGINE").unwrap_or_default();
        let program = match choice.as_str() {
            "xelatex" | "tectonic" => LatexProgram::XeLatex,
            "lualatex" => LatexProgram::LuaLatex,
            _ => LatexProgram::PdfLatex,
        };
        let engine = match program {
            LatexProgram::PdfLatex => nawl_codegen::Engine::Pdf,
            LatexProgram::XeLatex => nawl_codegen::Engine::Xe,
            LatexProgram::LuaLatex => nawl_codegen::Engine::Lua,
        };
        let system = SystemLatex::find(program).expect("installed TeX");
        let tectonic = (choice == "tectonic").then(|| nawl_compile::Compiler::locate(&[]).expect("tectonic"));
        let compile = |tex: &str, assets: &[Asset]| match &tectonic {
            Some(t) => t.compile(tex, assets),
            None => system.compile(tex, assets),
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let file: DocumentFile =
            serde_json::from_str(&std::fs::read_to_string(root.join("examples/journal-article.loom")).unwrap()).unwrap();
        let only = std::env::var("NAWL_ONLY").ok();
        let mut failed = Vec::new();
        // Every class of a supported kind, skipped ones included (to see if they still fail).
        let found: Vec<InstalledClass> =
            find().into_iter().filter(|c| category(c).is_some_and(Category::supported)).collect();
        for c in found.iter().filter(|c| only.as_deref().is_none_or(|o| o == c.class)) {
            let g = nawl_codegen::generate_for(&template(c), &file.document, engine);
            let figures: Vec<(String, Vec<u8>)> =
                g.figures.iter().map(|f| (f.clone(), std::fs::read(root.join("examples").join(f)).unwrap())).collect();
            let support = g.support_files();
            let assets: Vec<Asset> = support.iter().chain(figures.iter()).map(|(p, b)| Asset { path: p, bytes: b }).collect();
            if let Some(dir) = std::env::var_os("NAWL_DUMP") {
                std::fs::write(Path::new(&dir).join(format!("{}.tex", c.class)), &g.tex).unwrap();
                std::fs::write(Path::new(&dir).join(g.cls_file_name()), &g.cls).unwrap();
            }
            match compile(&g.tex, &assets) {
                Ok(_) => println!("ok    {:24} {:?} {}", c.class, category(c).unwrap(), display_name(c)),
                Err(nawl_compile::CompileError::Failed { errors, .. }) => {
                    let first = errors.first().map(|e| format!("l.{:?} {}", e.line, e.message.lines().next().unwrap_or(""))).unwrap_or_default();
                    println!("FAIL  {:24} {:?} {}", c.class, category(c).unwrap(), first);
                    failed.push(c.class.clone());
                }
                Err(e) => {
                    println!("FAIL  {:24} {:?} {e}", c.class, category(c).unwrap());
                    failed.push(c.class.clone());
                }
            }
        }
        println!("{} of {} compiled; failed: {failed:?}", found.len() - failed.len(), found.len());
    }
}
