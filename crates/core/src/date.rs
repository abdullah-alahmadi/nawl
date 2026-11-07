//! Date fields: stored as ISO dates (`2026-05-14`), written out in the document's
//! language, and also in the Hijri calendar (Umm al-Qura, as Saudi and Gulf
//! universities use), for `{field_hijri}` placeholders.

use icu_calendar::cal::Hijri;
use icu_calendar::Date;

use crate::{Digits, Language};

/// A date field's value: `YYYY-MM-DD`, as the date picker stores it.
pub fn parse_iso(s: &str) -> Option<(i32, u8, u8)> {
    let mut parts = s.trim().splitn(3, '-');
    let y = parts.next()?.parse().ok()?;
    let m = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    Date::try_new_iso(y, m, d).ok()?;
    Some((y, m, d))
}

/// The same day in the Umm al-Qura calendar (year, month, day).
pub fn hijri(y: i32, m: u8, d: u8) -> Option<(i32, u8, u8)> {
    let date = Date::try_new_iso(y, m, d).ok()?.to_calendar(Hijri::new_umm_al_qura());
    Some((date.year().extended_year(), date.month().ordinal, date.day_of_month().0))
}

const MONTHS_EN: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
// The names used in Egypt and the Gulf (the Levant's كانون الثاني… aren't covered).
const MONTHS_AR: [&str; 12] = ["يناير", "فبراير", "مارس", "أبريل", "مايو", "يونيو", "يوليو", "أغسطس", "سبتمبر", "أكتوبر", "نوفمبر", "ديسمبر"];
const HIJRI_EN: [&str; 12] = [
    "Muharram",
    "Safar",
    "Rabi al-Awwal",
    "Rabi al-Thani",
    "Jumada al-Ula",
    "Jumada al-Akhirah",
    "Rajab",
    "Shaban",
    "Ramadan",
    "Shawwal",
    "Dhu al-Qadah",
    "Dhu al-Hijjah",
];
const HIJRI_AR: [&str; 12] =
    ["محرم", "صفر", "ربيع الأول", "ربيع الآخر", "جمادى الأولى", "جمادى الآخرة", "رجب", "شعبان", "رمضان", "شوال", "ذو القعدة", "ذو الحجة"];

fn digits(s: String, d: Digits) -> String {
    match d {
        Digits::Western => s,
        Digits::ArabicIndic => s.chars().map(|c| c.to_digit(10).and_then(|n| char::from_u32(0x0660 + n)).unwrap_or(c)).collect(),
    }
}

/// A date field as text: "14 May 2026" / "14 مايو 2026". A value that isn't an ISO
/// date (typed before the field was a date) is kept as it is.
pub fn format(value: &str, lang: Language, d: Digits) -> String {
    let Some((y, m, day)) = parse_iso(value) else { return value.to_string() };
    let months = if lang == Language::Arabic { &MONTHS_AR } else { &MONTHS_EN };
    digits(format!("{day} {} {y}", months[m as usize - 1]), d)
}

/// The Hijri date for a date field: "26 Dhu al-Qadah 1447 AH" / "26 ذو القعدة 1447هـ";
/// `None` when the value isn't an ISO date.
pub fn format_hijri(value: &str, lang: Language, d: Digits) -> Option<String> {
    let (y, m, day) = parse_iso(value)?;
    let (hy, hm, hd) = hijri(y, m, day)?;
    Some(match lang {
        Language::Arabic => digits(format!("{hd} {} {hy}هـ", HIJRI_AR[hm as usize - 1]), d),
        Language::English => format!("{hd} {} {hy} AH", HIJRI_EN[hm as usize - 1]),
    })
}

/// The suffix that asks for a date field's Hijri form: `{defense_date_hijri}`.
pub const HIJRI_SUFFIX: &str = "_hijri";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umm_al_qura_dates() {
        // Ramadan 1447 begins on 18 February 2026 (Umm al-Qura); 1 Muharram 1447 was
        // 26 June 2025.
        assert_eq!(hijri(2026, 2, 18), Some((1447, 9, 1)));
        assert_eq!(hijri(2025, 6, 26), Some((1447, 1, 1)));
        assert_eq!(format_hijri("2026-02-18", Language::Arabic, Digits::ArabicIndic).as_deref(), Some("١ رمضان ١٤٤٧هـ"));
        assert_eq!(format_hijri("2026-02-18", Language::English, Digits::Western).as_deref(), Some("1 Ramadan 1447 AH"));
    }

    #[test]
    fn gregorian_text() {
        assert_eq!(format("2026-05-14", Language::English, Digits::Western), "14 May 2026");
        assert_eq!(format("2026-05-14", Language::Arabic, Digits::ArabicIndic), "١٤ مايو ٢٠٢٦");
        assert_eq!(format("Spring 2026", Language::English, Digits::Western), "Spring 2026", "free text stays");
        assert_eq!(format_hijri("Spring 2026", Language::Arabic, Digits::Western), None);
        assert_eq!(parse_iso("2026-02-30"), None);
    }
}
