//! The template model: everything the visual designer edits. A template compiles
//! into a LaTeX class (`.cls`), and also drives the editor's on-screen styling.
//!
//! Units are explicit in field names (`_mm`, `_pt`) so the designer UI and the code
//! generator never have to guess.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Template {
    pub meta: TemplateMeta,
    /// What kind of document the template makes: an article or report (the
    /// default), or a presentation.
    #[serde(default)]
    pub kind: DocumentKind,
    /// For presentations: the beamer theme and slide settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub presentation: Option<PresentationSettings>,
    /// Build on a publisher's official class (IEEEtran, elsarticle…) instead of
    /// Texloom's own layout. The class then decides page layout, fonts, headings and
    /// captions; the rest of the template only styles the on-screen editor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub base: Option<ClassBase>,
    /// The document's language (Arabic is right to left), a second language used in
    /// it, and the digits. Absent means English only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub language: Option<LanguageSettings>,
    pub page: PageSetup,
    pub fonts: Fonts,
    pub structure: Structure,
    pub headings: Headings,
    pub header_footer: HeaderFooter,
    /// How figure and table captions look. Older templates get the defaults.
    #[serde(default)]
    pub captions: CaptionStyle,
    /// How citations and the reference list look. Older templates get the defaults.
    #[serde(default)]
    pub bibliography: BibliographySettings,
    /// Chapters a new document starts with; `required` ones are checked.
    #[serde(default)]
    pub skeleton: Vec<SkeletonSection>,
    pub title_page: TitlePage,
    /// Metadata the document author fills in (title, team members, advisor…).
    pub fields: Vec<MetadataField>,
    /// What comes before the main text, in order.
    pub front_matter: Vec<FrontMatterItem>,
    /// Escape hatch: raw LaTeX appended to the end of the generated class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub raw_preamble: Option<String>,
}

/// The kind of document a template makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DocumentKind {
    /// Articles, reports and theses: flowing pages.
    #[default]
    Article,
    /// Slides (LaTeX beamer).
    Presentation,
}

impl DocumentKind {
    pub fn is_article(&self) -> bool {
        *self == DocumentKind::Article
    }
}

/// How a presentation looks: a beamer theme and slide options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PresentationSettings {
    /// Beamer theme, e.g. `Madrid`, `metropolis`, `default`.
    pub theme: String,
    /// Colour theme (`beaver`, `dolphin`…); empty for the theme's own.
    #[serde(default)]
    pub color_theme: String,
    /// Font theme (`serif`, `structurebold`…); empty for the theme's own.
    #[serde(default)]
    pub font_theme: String,
    /// Main colour (`#rrggbb`) instead of the theme's: title bars, bullets, blocks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub accent_color: Option<String>,
    #[serde(default)]
    pub aspect_ratio: AspectRatio,
    /// Base font size in points (beamer offers 8–20; 11 is its default).
    #[serde(default = "default_slide_font_pt")]
    pub font_size_pt: u8,
    /// An outline slide after the title slide.
    #[serde(default)]
    pub outline: bool,
    /// An outline slide at the start of each section, highlighting it.
    #[serde(default)]
    pub section_slides: bool,
    /// Beamer's navigation symbols in the slide corner.
    #[serde(default)]
    pub navigation_symbols: bool,
    /// Where the logo (the document's `logo` image field) appears.
    #[serde(default)]
    pub logo: LogoPlacement,
    /// What the line at the bottom of each slide shows.
    #[serde(default)]
    pub footer: SlideFooter,
}

/// Where a presentation's logo appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LogoPlacement {
    /// On the title slide and in the corner of every slide.
    #[default]
    EverySlide,
    /// On the title slide only.
    TitleSlide,
    /// Nowhere.
    Off,
}

/// The footer line of a presentation's slides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlideFooter {
    pub style: FooterStyle,
    /// Left, centre and right parts for the custom style: text with `{author}`,
    /// `{title}`, `{date}`, `{institute}`, `{section}`, `{page}`, `{pages}`.
    #[serde(default)]
    pub left: String,
    #[serde(default)]
    pub center: String,
    #[serde(default)]
    pub right: String,
}

impl Default for SlideFooter {
    fn default() -> Self {
        Self { style: FooterStyle::Theme, left: "{author}".into(), center: "{title}".into(), right: "{page} / {pages}".into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FooterStyle {
    /// Whatever the theme shows (Madrid: presenter, title, date and slide number).
    #[default]
    Theme,
    /// Three parts of the author's choosing.
    Custom,
    /// No footer line.
    None,
}

fn default_slide_font_pt() -> u8 {
    11
}

impl Default for PresentationSettings {
    fn default() -> Self {
        Self {
            theme: "Madrid".into(),
            color_theme: String::new(),
            font_theme: String::new(),
            accent_color: None,
            aspect_ratio: AspectRatio::default(),
            font_size_pt: 11,
            outline: false,
            section_slides: false,
            navigation_symbols: false,
            logo: LogoPlacement::default(),
            footer: SlideFooter::default(),
        }
    }
}

/// Slide shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum AspectRatio {
    /// 16:9 widescreen (160 × 90 mm).
    #[default]
    #[serde(rename = "16:9")]
    Wide,
    /// 16:10 (160 × 100 mm).
    #[serde(rename = "16:10")]
    Wide1610,
    /// 4:3, beamer's classic shape (128 × 96 mm).
    #[serde(rename = "4:3")]
    Classic,
}

impl AspectRatio {
    /// Beamer's `aspectratio` class option.
    pub fn beamer_option(self) -> &'static str {
        match self {
            AspectRatio::Wide => "169",
            AspectRatio::Wide1610 => "1610",
            AspectRatio::Classic => "43",
        }
    }

    /// Slide size in millimetres.
    pub fn size_mm(self) -> (f32, f32) {
        match self {
            AspectRatio::Wide => (160.0, 90.0),
            AspectRatio::Wide1610 => (160.0, 100.0),
            AspectRatio::Classic => (128.0, 96.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TemplateMeta {
    /// Lowercase slug; also the generated class name (`<id>.cls`).
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub author: Option<String>,
}

// ---------------------------------------------------------------- publisher classes

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClassBase {
    /// LaTeX class name, e.g. `IEEEtran`, `elsarticle`, or one shipped in `files`.
    pub class: String,
    /// Class options, e.g. `["journal"]` or `["sigconf"]`.
    #[serde(default)]
    pub options: Vec<String>,
    /// How the title block, authors, abstract and keywords are written for this class.
    pub front_matter: FrontMatterStyle,
    /// BibTeX style, e.g. `IEEEtran`, `elsarticle-num`; empty when the class sets it.
    pub bib_style: String,
    /// Extra files from the template's folder copied next to the document when it
    /// compiles (a publisher's .cls/.sty/.bst/logos), e.g. `assets/ieeeaccess.cls`.
    #[serde(default)]
    pub files: Vec<String>,
    /// Print author biographies (with photos) at the end, IEEE style.
    #[serde(default)]
    pub biographies: bool,
    /// For the `custom` style: the title block, as LaTeX with placeholders
    /// (`{title}`, `{#authors| \and }{name}{/authors}`…); see codegen's recipe docs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title_block: Option<String>,
    /// Optional LaTeX (same placeholders) written at the very end of the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub end_block: Option<String>,
    /// When the class isn't part of the TeX distribution: the publisher's kit to download.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub kit: Option<PublisherKit>,
}

/// A publisher's LaTeX kit that has to be downloaded (it can't be bundled).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PublisherKit {
    /// E.g. "IEEE Access LaTeX template".
    pub name: String,
    /// Where to download it.
    pub url: String,
    /// Files that must have been imported, e.g. `["ieeeaccess.cls"]`.
    pub required: Vec<String>,
}

/// Title-block conventions of the supported publisher classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FrontMatterStyle {
    /// IEEE journals: authors in one line, affiliations in a \thanks footnote.
    IeeeJournal,
    /// IEEE conferences: one author block per author.
    IeeeConference,
    /// Elsevier elsarticle: frontmatter environment, numbered affiliations.
    Elsevier,
    /// ACM acmart: per-author affiliation and e-mail.
    Acm,
    /// Springer LNCS: authors joined by \and, \institute with \inst marks.
    Lncs,
    /// APS REVTeX: per-author affiliation, abstract before \maketitle.
    Revtex,
    /// IEEE Access: \history, \doi, \authorrefmark, \address, \corresp, \EOD.
    IeeeAccess,
    /// Written from the template's own `title_block` recipe.
    Custom,
}

// ---------------------------------------------------------------- page

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PageSetup {
    pub paper: Paper,
    pub margins: Margins,
    /// Two-sided printing: inner/outer margins alternate between odd and even pages.
    pub two_sided: bool,
    /// Extra space on the inner edge for binding.
    pub binding_offset_mm: f32,
    pub line_spacing: LineSpacing,
    pub paragraph: ParagraphStyle,
    /// Text columns: 1, or 2 for a two-column journal or conference layout.
    #[serde(default = "one_column", skip_serializing_if = "is_one_column")]
    #[ts(as = "Option<u8>", optional)]
    pub columns: u8,
}

fn one_column() -> u8 {
    1
}

fn is_one_column(n: &u8) -> bool {
    *n == 1
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum Paper {
    A4,
    A5,
    Letter,
    Legal,
    #[serde(rename_all = "camelCase")]
    Custom { width_mm: f32, height_mm: f32 },
}

impl Paper {
    /// Width and height in millimetres.
    pub fn size_mm(&self) -> (f32, f32) {
        match *self {
            Paper::A4 => (210.0, 297.0),
            Paper::A5 => (148.0, 210.0),
            Paper::Letter => (215.9, 279.4),
            Paper::Legal => (215.9, 355.6),
            Paper::Custom { width_mm, height_mm } => (width_mm, height_mm),
        }
    }
}

/// Distances from the paper edge to the text body. With one-sided printing,
/// `inner` is the left margin and `outer` the right.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Margins {
    pub top_mm: f32,
    pub bottom_mm: f32,
    pub inner_mm: f32,
    pub outer_mm: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum LineSpacing {
    Single,
    OneHalf,
    Double,
    /// Multiplier on the font's normal line spacing.
    Custom { stretch: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParagraphStyle {
    /// First-line indent.
    pub indent_mm: f32,
    /// Extra vertical space between paragraphs.
    pub spacing_pt: f32,
}

// ---------------------------------------------------------------- language

/// A language Texloom typesets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Language {
    #[default]
    English,
    Arabic,
}

impl Language {
    /// Written right to left.
    pub fn rtl(self) -> bool {
        matches!(self, Language::Arabic)
    }

    /// The name polyglossia knows it by (`\setotherlanguage{arabic}`, `\textarabic`).
    pub fn polyglossia(self) -> &'static str {
        match self {
            Language::English => "english",
            Language::Arabic => "arabic",
        }
    }

    /// polyglossia's environment for a passage in the language. Arabic's is
    /// `Arabic`, because `\arabic` is LaTeX's counter command.
    pub fn environment(self) -> &'static str {
        match self {
            Language::English => "english",
            Language::Arabic => "Arabic",
        }
    }

    /// BCP 47 tag, for the editor (spell checking) and Word.
    pub fn tag(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Arabic => "ar",
        }
    }
}

/// How numbers are written in Arabic documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Digits {
    /// 0123456789, as in the Maghreb and most of the world.
    #[default]
    Western,
    /// ٠١٢٣٤٥٦٧٨٩, as in the Gulf, the Levant and Egypt.
    ArabicIndic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LanguageSettings {
    #[serde(default)]
    pub main: Language,
    /// A second language with passages of its own (an English abstract, English
    /// references). Latin words in an Arabic document always work without it.
    #[serde(default)]
    pub other: Option<Language>,
    /// Digits for page, chapter, figure and equation numbers (Arabic documents).
    #[serde(default)]
    pub digits: Digits,
}

impl LanguageSettings {
    /// The languages the document uses besides the main one. An Arabic document
    /// always has English (Latin words, references, URLs).
    pub fn others(&self) -> Vec<Language> {
        let mut out: Vec<Language> = self.other.into_iter().filter(|l| *l != self.main).collect();
        if self.main == Language::Arabic && !out.contains(&Language::English) {
            out.push(Language::English);
        }
        out
    }

    /// The document needs polyglossia: anything but English alone.
    pub fn multilingual(&self) -> bool {
        self.main != Language::English || !self.others().is_empty()
    }

    /// Arabic text can only be typeset by XeTeX and LuaTeX (not pdfLaTeX).
    pub fn needs_unicode_engine(&self) -> bool {
        self.main == Language::Arabic || self.others().contains(&Language::Arabic)
    }
}

impl Template {
    /// The language settings (English only when the template has none).
    pub fn lang(&self) -> LanguageSettings {
        self.language.unwrap_or_default()
    }
}

// ---------------------------------------------------------------- fonts

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Fonts {
    /// Body text size: 10, 11 or 12 (the sizes LaTeX's standard classes support).
    pub base_size_pt: u8,
    pub main: FontChoice,
    pub sans: FontChoice,
    pub mono: FontChoice,
    /// The font for Arabic text (Amiri when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub arabic: Option<FontChoice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum FontChoice {
    /// A font shipped with the TeX distribution: always available, on every machine.
    Builtin { font: BuiltinFont },
    /// A font installed on the system, by family name.
    System { family: String },
    /// A font from Texloom's Arabic font library (shipped with the app, see
    /// [`crate::fonts`]): its files go with the document.
    Library { id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BuiltinFont {
    /// LaTeX's default (Computer Modern look).
    LatinModern,
    /// Times-like serif.
    Termes,
    /// Palatino-like serif.
    Pagella,
    /// Bookman-like serif.
    Bonum,
    /// Century Schoolbook-like serif.
    Schola,
    /// Helvetica-like sans.
    Heros,
    /// Avant Garde-like sans.
    Adventor,
    /// Courier-like monospace.
    Cursor,
    /// Arabic naskh in the style of Bulaq press books (with Latin letters too).
    Amiri,
}

// ---------------------------------------------------------------- structure

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Structure {
    /// What a level-1 heading in the editor becomes.
    pub top_level: TopLevel,
    /// How many heading levels get numbers (0–4).
    pub numbered_depth: u8,
    /// How many heading levels appear in the table of contents (0–4).
    pub toc_depth: u8,
    pub front_matter_page_numbers: PageNumberStyle,
    pub main_page_numbers: PageNumberStyle,
    /// With chapters: number equations, figures and tables per chapter ("2.3")
    /// instead of continuously ("7").
    pub per_chapter_numbering: bool,
    /// Label for appendix chapters (or top-level sections); `{n}` is the letter.
    #[serde(default = "default_appendix_label")]
    pub appendix_label: String,
    /// With chapters: each chapter opens on a cover page of its own (its number and
    /// title, centred), and so do the reference list and the appendices, as some
    /// university reports require.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub cover_pages: bool,
}

fn default_appendix_label() -> String {
    "Appendix {n}".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TopLevel {
    Chapter,
    Section,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PageNumberStyle {
    Arabic,
    LowerRoman,
    UpperRoman,
}

// ---------------------------------------------------------------- headings

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Headings {
    /// Styles for editor heading levels 1–4, in order. Must have exactly 4 entries.
    pub levels: Vec<HeadingStyle>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HeadingStyle {
    pub text: TextStyle,
    pub layout: HeadingLayout,
    /// Number label; `{n}` is the number. E.g. `"Chapter {n}"` or `"{n}."`.
    pub label: String,
    /// Gap between label and title: vertical for `display`, horizontal for `inline`.
    pub label_gap_pt: f32,
    pub space_before_pt: f32,
    pub space_after_pt: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HeadingLayout {
    /// Label and title on one line: "1.2  Background".
    Inline,
    /// Label on its own line above the title: "CHAPTER 1" / "Introduction".
    Display,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextStyle {
    pub size_pt: f32,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub small_caps: bool,
    #[serde(default)]
    pub uppercase: bool,
    #[serde(default)]
    pub align: Align,
    /// `#RRGGBB`; black when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<String>,
    /// A font of its own (a title or a heading level in another face); the text's
    /// font when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub font: Option<FontChoice>,
}

impl Template {
    /// Every font the template uses: the text fonts, then those of heading levels and
    /// of items on the title page and fixed pages (to load, send or embed them).
    pub fn all_fonts(&self) -> Vec<&FontChoice> {
        let f = &self.fonts;
        let mut out: Vec<&FontChoice> = vec![&f.main, &f.sans, &f.mono];
        out.extend(f.arabic.as_ref());
        out.extend(self.headings.levels.iter().filter_map(|h| h.text.font.as_ref()));
        for (_, item) in self.placed_items() {
            if let TitleContent::Text { style, .. } = &item.content {
                out.extend(style.font.as_ref());
            }
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

// ---------------------------------------------------------------- headers & footers

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HeaderFooter {
    pub front_matter: PageStyle,
    pub main: PageStyle,
    /// Pages where a chapter starts (and other "plain" pages).
    pub chapter_opening: PageStyle,
}

/// Header and footer slots. Text may contain placeholders: `{page}`, `{pages}`,
/// `{chapter}`, `{section}`, or any metadata field id such as `{title}`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PageStyle {
    #[serde(default)]
    pub header: Slots,
    #[serde(default)]
    pub footer: Slots,
    #[serde(default)]
    pub header_rule_pt: f32,
    #[serde(default)]
    pub footer_rule_pt: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Slots {
    #[serde(default)]
    pub left: String,
    #[serde(default)]
    pub center: String,
    #[serde(default)]
    pub right: String,
}

// ---------------------------------------------------------------- captions

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CaptionStyle {
    /// Word before figure numbers: "Figure", "Fig.", "Abbildung"…
    pub figure_label: String,
    /// Word before table numbers.
    pub table_label: String,
    pub separator: CaptionSeparator,
    pub label_bold: bool,
    /// Caption text size relative to body text.
    pub size: CaptionSize,
    pub align: CaptionAlign,
}

impl Default for CaptionStyle {
    fn default() -> Self {
        Self {
            figure_label: "Figure".into(),
            table_label: "Table".into(),
            separator: CaptionSeparator::Colon,
            label_bold: true,
            size: CaptionSize::Small,
            align: CaptionAlign::Center,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CaptionSeparator {
    /// "Figure 1: Text"
    Colon,
    /// "Figure 1. Text"
    Period,
    /// "Figure 1 — Text"
    Dash,
    /// "Figure 1" on its own line, text below.
    Newline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CaptionSize {
    Footnote,
    Small,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CaptionAlign {
    Center,
    Left,
    /// Centered when the caption fits on one line, justified otherwise.
    Justified,
}

// ---------------------------------------------------------------- bibliography

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BibliographySettings {
    pub style: BibStyle,
    /// Heading of the reference list.
    pub title: String,
    pub in_toc: bool,
    /// Two lists under the heading: entries in the document's language, then the
    /// others ("المراجع العربية" then "المراجع الأجنبية", as Arabic theses do).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub split_by_language: bool,
}

impl Default for BibliographySettings {
    fn default() -> Self {
        Self { style: BibStyle::Ieee, title: "References".into(), in_toc: true, split_by_language: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BibStyle {
    /// [1], [2] in order of first citation; IEEE reference format.
    Ieee,
    /// [1], [2] with the list sorted by author.
    Numeric,
    /// (Author, 2020), APA-like list sorted by author.
    AuthorYear,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SkeletonSection {
    pub title: String,
    #[serde(default)]
    pub required: bool,
}

// ---------------------------------------------------------------- title page

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TitlePage {
    pub items: Vec<TitleItem>,
    /// An article's title block: the items are drawn at the top of the first page
    /// and the text starts this far below the paper's top edge, instead of the title
    /// having a page of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub block_height_mm: Option<f32>,
}

/// An item placed at an exact position on the title page. `x_mm`/`y_mm` locate the
/// item's `anchor` point, measured from the top-left corner of the paper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TitleItem {
    pub x_mm: f32,
    pub y_mm: f32,
    pub anchor: Anchor,
    pub content: TitleContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum TitleContent {
    /// Text with placeholders, e.g. `"{title}"` or `"Advisor: {advisor}"`.
    #[serde(rename_all = "camelCase")]
    Text {
        text: String,
        style: TextStyle,
        /// Wrap width; text doesn't wrap when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        width_mm: Option<f32>,
    },
    /// An image file from the template's assets, e.g. `"assets/logo.png"`, or an
    /// image field of the document, e.g. `"{logo}"` (a university's logo the author
    /// picks, which a built-in template can't ship).
    #[serde(rename_all = "camelCase")]
    Image { asset: String, width_mm: f32 },
    /// A horizontal line.
    #[serde(rename_all = "camelCase")]
    Rule { length_mm: f32, thickness_pt: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

// ---------------------------------------------------------------- fields & front matter

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MetadataField {
    /// Lowercase id used in placeholders: `[a-z][a-z0-9_]*`.
    pub id: String,
    pub label: String,
    pub kind: FieldKind,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FieldKind {
    /// One line of text.
    Text,
    /// Several paragraphs (e.g. an abstract).
    Paragraphs,
    /// Paper authors with affiliations, e-mail, ORCID, membership, biography.
    Authors,
    /// A list of entries (e.g. team members). Shown one per line on the title page,
    /// comma-separated in headers and footers.
    List,
    /// An image chosen by the author (a presentation's logo): a path relative to the
    /// document's folder, like figures.
    Image,
    /// A date, stored as `YYYY-MM-DD` and written in the document's language;
    /// `{id_hijri}` gives the same day in the Hijri calendar.
    Date,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum FrontMatterItem {
    TitlePage,
    /// An unnumbered page showing a `paragraphs` field, e.g. the abstract.
    /// Skipped when an optional field is empty.
    #[serde(rename_all = "camelCase")]
    FieldPage {
        title: String,
        field: String,
        in_toc: bool,
        /// Written in another language than the document (an English abstract in an
        /// Arabic thesis): the page is set in that language and direction.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        language: Option<Language>,
        /// Under the title on the first page (an article's abstract) instead of on a
        /// page of its own.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[ts(as = "Option<bool>", optional)]
        inline: bool,
    },
    TableOfContents { title: String },
    ListOfFigures { title: String },
    ListOfTables { title: String },
    /// A page laid out like the title page, with items at exact positions: an
    /// approval or signature page, a declaration of originality…
    #[serde(rename_all = "camelCase")]
    FixedPage {
        /// Its name in the designer, and its entry in the table of contents.
        title: String,
        #[serde(default)]
        items: Vec<TitleItem>,
        #[serde(default)]
        in_toc: bool,
    },
}

/// The image field a title-page image shows (`"{logo}"` → `logo`), if it names one.
pub fn image_field(asset: &str) -> Option<&str> {
    asset.strip_prefix('{')?.strip_suffix('}')
}

impl Template {
    pub fn field(&self, id: &str) -> Option<&MetadataField> {
        self.fields.iter().find(|f| f.id == id)
    }

    /// Every item placed at an exact position (the title page and fixed pages), with
    /// its path for issues: `titlePage.items[0]`, `frontMatter[2].items[1]`.
    pub fn placed_items(&self) -> Vec<(String, &TitleItem)> {
        let mut out: Vec<(String, &TitleItem)> =
            self.title_page.items.iter().enumerate().map(|(i, it)| (format!("titlePage.items[{i}]"), it)).collect();
        for (p, page) in self.front_matter.iter().enumerate() {
            if let FrontMatterItem::FixedPage { items, .. } = page {
                out.extend(items.iter().enumerate().map(|(i, it)| (format!("frontMatter[{p}].items[{i}]"), it)));
            }
        }
        out
    }
}
