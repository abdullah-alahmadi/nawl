//! Small helpers for writing WordprocessingML by hand.

/// Escapes text and attribute values.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            // Characters XML 1.0 doesn't allow.
            c if (c as u32) < 0x20 && c != '\t' && c != '\n' && c != '\r' => {}
            c => o.push(c),
        }
    }
    o
}

/// Millimetres to twentieths of a point (Word's page measures).
pub fn twips(mm: f32) -> i64 {
    (mm * 1440.0 / 25.4).round() as i64
}

/// Millimetres to English Metric Units (drawing sizes).
pub fn emu(mm: f32) -> i64 {
    (mm * 36000.0).round() as i64
}

/// Points to half-points (font sizes).
pub fn half_points(pt: f32) -> i64 {
    (pt * 2.0).round() as i64
}

/// Points to twips (spacing).
pub fn pt_twips(pt: f32) -> i64 {
    (pt * 20.0).round() as i64
}

pub const DOCUMENT_NS: &str = concat!(
    r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" "#,
    r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" "#,
    r#"xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" "#,
    r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" "#,
    r#"xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" "#,
    r#"xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math""#
);
