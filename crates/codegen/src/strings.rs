//! Words Texloom writes itself (not from the template), in the document's language.

use texloom_core::Language;

pub struct Words {
    pub chapter: &'static str,
    pub section: &'static str,
    pub appendix: &'static str,
    pub no_image: &'static str,
    pub outline: &'static str,
    pub and: &'static str,
    pub et_al: &'static str,
    pub no_date: &'static str,
    pub abstract_name: &'static str,
    pub corresponding_author: &'static str,
    pub edition: &'static str,
    pub pages: &'static str,
    pub page: &'static str,
    pub volume: &'static str,
    pub number: &'static str,
    pub phd_thesis: &'static str,
    pub masters_thesis: &'static str,
    pub technical_report: &'static str,
    pub in_: &'static str,
    /// Headings of split reference lists: entries in the document's language, then
    /// the others.
    pub own_references: &'static str,
    pub other_references: &'static str,
}

pub fn words(lang: Language) -> &'static Words {
    match lang {
        Language::English => &ENGLISH,
        Language::Arabic => &ARABIC,
    }
}

static ENGLISH: Words = Words {
    chapter: "Chapter",
    section: "Section",
    appendix: "Appendix",
    no_image: "No image",
    outline: "Outline",
    and: "and",
    et_al: "et al.",
    no_date: "n.d.",
    abstract_name: "Abstract",
    corresponding_author: "Corresponding author",
    edition: "ed.",
    pages: "pp.",
    page: "p.",
    volume: "vol.",
    number: "no.",
    phd_thesis: "Ph.D. dissertation",
    masters_thesis: "Master's thesis",
    technical_report: "Tech. Rep.",
    in_: "In",
    own_references: "English references",
    other_references: "Arabic references",
};

static ARABIC: Words = Words {
    chapter: "الفصل",
    section: "القسم",
    appendix: "الملحق",
    no_image: "لا توجد صورة",
    outline: "المحتويات",
    and: "و",
    et_al: "وآخرون",
    no_date: "د.ت.",
    abstract_name: "المستخلص",
    corresponding_author: "المؤلف المراسل",
    edition: "ط.",
    pages: "ص ص.",
    page: "ص.",
    volume: "مج.",
    number: "ع.",
    phd_thesis: "أطروحة دكتوراه",
    masters_thesis: "رسالة ماجستير",
    technical_report: "تقرير فني",
    in_: "في",
    own_references: "المراجع العربية",
    other_references: "المراجع الأجنبية",
};
