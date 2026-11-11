//! Nawl's Arabic font library: free fonts (SIL Open Font License) shipped with the
//! app (`fonts/catalog.json`, downloaded by `scripts/fetch-fonts.mjs`). A template
//! chooses one with [`FontChoice::Library`](crate::FontChoice::Library); its files go
//! into the build folder (`fonts/<id>-Regular.ttf`, `-Bold.ttf`), so a document
//! compiles the same on every machine.

use std::sync::OnceLock;

use serde::Deserialize;

/// A family in the library.
#[derive(Debug, Clone, Deserialize)]
pub struct LibraryFont {
    pub id: String,
    pub name: String,
    /// naskh, sans, kufi, calligraphic, display or nastaliq.
    pub category: String,
    #[serde(default)]
    regular: Option<String>,
    #[serde(default)]
    bold: Option<String>,
    #[serde(default)]
    variable: Option<String>,
}

impl LibraryFont {
    /// Whether it has a bold (variable fonts are cut into a Regular and a Bold).
    pub fn has_bold(&self) -> bool {
        self.variable.is_some() || self.bold.is_some()
    }

    /// The regular file, as it's shipped and placed in the build folder's `fonts/`.
    pub fn regular_file(&self) -> String {
        format!("{}-Regular.ttf", self.id)
    }

    /// The bold file, when the family has one.
    pub fn bold_file(&self) -> Option<String> {
        self.has_bold().then(|| format!("{}-Bold.ttf", self.id))
    }

    /// Both files, relative to the build folder (`fonts/…`).
    pub fn files(&self) -> Vec<String> {
        let _ = (&self.regular, &self.bold);
        std::iter::once(self.regular_file()).chain(self.bold_file()).map(|f| format!("fonts/{f}")).collect()
    }
}

#[derive(Deserialize)]
struct Catalog {
    fonts: Vec<LibraryFont>,
}

/// Every family in the library, in catalog order (grouped by category).
pub fn library() -> &'static [LibraryFont] {
    static FONTS: OnceLock<Vec<LibraryFont>> = OnceLock::new();
    FONTS.get_or_init(|| serde_json::from_str::<Catalog>(include_str!("../../../fonts/catalog.json")).expect("font catalog parses").fonts)
}

/// A family by id.
pub fn find(id: &str) -> Option<&'static LibraryFont> {
    library().iter().find(|f| f.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog() {
        assert!(library().len() >= 30);
        let cairo = find("cairo").unwrap();
        assert_eq!(cairo.files(), ["fonts/cairo-Regular.ttf", "fonts/cairo-Bold.ttf"], "variable: cut into both");
        assert_eq!(find("lalezar").unwrap().files(), ["fonts/lalezar-Regular.ttf"], "no bold");
        let ids: std::collections::HashSet<&str> = library().iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids.len(), library().len(), "ids are unique");
        for f in library() {
            assert!(["naskh", "sans", "kufi", "calligraphic", "display", "nastaliq"].contains(&f.category.as_str()), "{}", f.id);
        }
    }
}
