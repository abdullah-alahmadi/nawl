//! Authors with different affiliations: numbers pair each name with its
//! affiliations ("Name¹, Name²" over "¹Affiliation", "²Affiliation").

use nawl_core::{author_entries, AuthorInfo, DocumentFile, FieldValue, Template};

fn author(name: &str, name_en: &str, affiliations: &[(&str, &str)]) -> AuthorInfo {
    AuthorInfo {
        name: name.into(),
        name_en: name_en.into(),
        affiliations: affiliations.iter().map(|(a, _)| a.to_string()).collect(),
        affiliations_en: affiliations.iter().map(|(_, e)| e.to_string()).collect(),
        ..Default::default()
    }
}

const EE: (&str, &str) = ("قسم الهندسة الكهربائية، جامعة المجمعة", "Department of Electrical Engineering, Majmaah University");
const ECE: (&str, &str) = ("قسم الهندسة الكهربائية وهندسة الحاسبات، جامعة الملك عبدالعزيز", "Department of Electrical and Computer Engineering, King Abdulaziz University");

#[test]
fn numbers_only_when_affiliations_differ() {
    // Same affiliation: no numbers anywhere.
    let same = [author("أ", "A", &[EE]), author("ب", "B", &[EE])];
    assert!(author_entries(&same, "").iter().all(|(_, n)| n.is_none()));
    assert_eq!(author_entries(&same, "_affiliations"), vec![(EE.0.to_string(), None)]);
    // Both authors in the same two places: still nothing to tell apart.
    let both = [author("أ", "A", &[EE, ECE]), author("ب", "B", &[EE, ECE])];
    assert!(author_entries(&both, "").iter().all(|(_, n)| n.is_none()));

    // Different affiliations: each name carries its numbers, each affiliation its own.
    let differ = [author("أ", "A", &[EE]), author("ب", "B", &[ECE]), author("ج", "C", &[EE, ECE])];
    assert_eq!(
        author_entries(&differ, ""),
        vec![("أ".to_string(), Some(vec![1])), ("ب".to_string(), Some(vec![2])), ("ج".to_string(), Some(vec![1, 2]))]
    );
    assert_eq!(author_entries(&differ, "_affiliations"), vec![(EE.0.to_string(), Some(vec![1])), (ECE.0.to_string(), Some(vec![2]))]);
    assert_eq!(author_entries(&differ, "_affiliations_en"), vec![(EE.1.to_string(), Some(vec![1])), (ECE.1.to_string(), Some(vec![2]))]);
    assert_eq!(author_entries(&differ, "_en")[2], ("C".to_string(), Some(vec![1, 2])));
}

#[test]
fn numbered_authors_in_latex() {
    let t: Template = serde_json::from_str(include_str!("../../../templates/arabic-article/template.json")).unwrap();
    let mut doc: DocumentFile = serde_json::from_str(include_str!("../../../examples/arabic-load-forecasting.loom")).unwrap();
    doc.document.metadata.insert("authors".into(), FieldValue::Authors(vec![author("عبدالله الأحمدي", "Abdullah Al-Ahmadi", &[EE]), author("سارة القحطاني", "Sarah Al-Qahtani", &[ECE])]));
    let tex = nawl_codegen::document_to_tex(&t, &doc.document);
    let field = |id: &str| tex.lines().find(|l| l.starts_with(&format!("\\texloomfield{{{id}}}"))).unwrap_or_else(|| panic!("{id} missing")).to_string();
    // Arabic names and affiliations: the template's Arabic-Indic digits.
    assert!(field("authors").contains("عبدالله الأحمدي\\textsuperscript{١}\\texloomsep سارة القحطاني\\textsuperscript{٢}"), "{}", field("authors"));
    assert!(field("authors_affiliations").starts_with("\\texloomfield{authors_affiliations}{\\textsuperscript{١}قسم الهندسة الكهربائية"), "{}", field("authors_affiliations"));
    // English names: Western digits inside the English run.
    assert!(field("authors_en").contains("\\textenglish{Abdullah Al-Ahmadi\\textsuperscript{1}\\texloomsepen Sarah Al-Qahtani\\textsuperscript{2}}"), "{}", field("authors_en"));

    // One shared affiliation: the names stay as they were.
    doc.document.metadata.insert("authors".into(), FieldValue::Authors(vec![author("عبدالله الأحمدي", "", &[EE]), author("سارة القحطاني", "", &[EE])]));
    let tex = nawl_codegen::document_to_tex(&t, &doc.document);
    let authors: Vec<&str> = tex.lines().filter(|l| l.starts_with("\\texloomfield{authors")).collect();
    assert!(!authors.is_empty() && authors.iter().all(|l| !l.contains("\\textsuperscript")), "{authors:#?}");
}
