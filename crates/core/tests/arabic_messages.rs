//! Messages follow the app's interface language. A test binary of its own: the
//! language is global, and other tests compare English messages.

use nawl_core::Template;

#[test]
fn validation_messages_in_arabic() {
    let mut t: Template = serde_json::from_str(include_str!("../../../templates/general-article/template.json")).unwrap();
    t.page.columns = 3;
    t.meta.id = "article".into();
    let english: Vec<String> = t.validate().into_iter().map(|i| i.message).collect();
    assert!(english.iter().any(|m| m == "must be 1 or 2"), "{english:?}");

    nawl_core::msg::set_arabic(true);
    let arabic: Vec<String> = t.validate().into_iter().map(|i| i.message).collect();
    nawl_core::msg::set_arabic(false);
    assert_eq!(arabic.len(), english.len(), "the same problems");
    for m in &arabic {
        assert!(m.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)), "not Arabic: {m}");
    }
    assert!(arabic.iter().any(|m| m.contains("\"article\"")), "the id stays as typed: {arabic:?}");
}
