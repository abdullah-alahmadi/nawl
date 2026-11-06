//! Reads the SyncTeX file TeX writes next to the PDF (`main.synctex.gz`), which says
//! which line of which input made each box on each page. Texloom only needs the
//! main `.tex` file's lines: they map to the editor's blocks.
//!
//! Positions are in PDF points (1/72 in) from the page's top-left corner, the same
//! as pdf.js's viewport at scale 1.

use std::io::Read;
use std::path::Path;

/// TeX's scaled points per PDF point (65536 sp per TeX point, 72.27 TeX points per inch).
const SP_PER_BP: f32 = 65536.0 * 72.27 / 72.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Record {
    page: u32,
    line: u32,
    x: f32,
    y: f32,
    /// Width, height and depth for boxes; zero for points (glue, kerns, math).
    w: f32,
    h: f32,
    d: f32,
}

/// A place in the PDF.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct PdfSpot {
    /// 1-based.
    pub page: u32,
    pub x: f32,
    /// Top of the box.
    pub y: f32,
    pub height: f32,
}

/// The main file's records from a SyncTeX file.
#[derive(Debug, Clone, Default)]
pub struct SyncTex {
    records: Vec<Record>,
}

impl SyncTex {
    /// Reads `main.synctex.gz` (or an uncompressed `main.synctex`) from a build folder.
    pub fn read(dir: &Path, main: &str) -> Option<Self> {
        let gz = dir.join(format!("{main}.synctex.gz"));
        let text = if gz.is_file() {
            let mut s = String::new();
            flate2::read::GzDecoder::new(std::fs::File::open(gz).ok()?).read_to_string(&mut s).ok()?;
            s
        } else {
            std::fs::read_to_string(dir.join(format!("{main}.synctex"))).ok()?
        };
        Some(Self::parse(&text, &format!("{main}.tex")))
    }

    /// Parses SyncTeX text, keeping the records of the input whose path ends with
    /// `main_file`.
    pub fn parse(text: &str, main_file: &str) -> Self {
        let mut main_tags: Vec<u32> = Vec::new();
        let (mut unit, mut magnification, mut x_offset, mut y_offset) = (1.0f32, 1000.0f32, 0.0f32, 0.0f32);
        let mut page = 0u32;
        let mut raw: Vec<(u32, Record)> = Vec::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("Input:") {
                if let Some((tag, path)) = rest.split_once(':') {
                    let path = path.replace('\\', "/");
                    if (path == main_file || path.ends_with(&format!("/{main_file}"))) && let Ok(tag) = tag.parse() {
                        main_tags.push(tag);
                    }
                }
                continue;
            }
            let number = |prefix: &str| line.strip_prefix(prefix).and_then(|v| v.trim().parse::<f32>().ok());
            if let Some(v) = number("Unit:") {
                unit = v;
            } else if let Some(v) = number("Magnification:") {
                magnification = v;
            } else if let Some(v) = number("X Offset:") {
                x_offset = v;
            } else if let Some(v) = number("Y Offset:") {
                y_offset = v;
            }
            let mut chars = line.chars();
            match chars.next() {
                Some('{') => page = chars.as_str().parse().unwrap_or(page + 1),
                Some(c @ ('[' | '(' | 'h' | 'v' | 'x' | 'k' | 'g' | '$')) if page > 0 => {
                    if let Some((tag, r)) = parse_record(chars.as_str(), page, matches!(c, '[' | '(' | 'h' | 'v')) {
                        raw.push((tag, r));
                    }
                }
                _ => {}
            }
        }
        let scale = unit * magnification / 1000.0 / SP_PER_BP;
        let records = raw
            .into_iter()
            .filter(|(tag, _)| main_tags.contains(tag))
            .map(|(_, r)| Record {
                x: (r.x + x_offset) * scale,
                y: (r.y + y_offset) * scale,
                w: r.w * scale,
                h: r.h * scale,
                d: r.d * scale,
                ..r
            })
            .collect();
        Self { records }
    }

    /// The main file's line that made what's at (`x`, `y`) on `page`: the smallest box
    /// around the point, else the nearest record on the page.
    pub fn line_at(&self, page: u32, x: f32, y: f32) -> Option<u32> {
        let on_page = || self.records.iter().filter(move |r| r.page == page);
        let inside = on_page()
            .filter(|r| r.w > 0.0 && x >= r.x && x <= r.x + r.w && y >= r.y - r.h && y <= r.y + r.d)
            .min_by(|a, b| (a.w * (a.h + a.d)).total_cmp(&(b.w * (b.h + b.d))));
        let nearest = || {
            on_page().min_by(|a, b| {
                let dist = |r: &Record| {
                    // Distance to the box (or point): vertical distance counts more, so a
                    // click in a margin finds the line beside it.
                    let dx = if x < r.x { r.x - x } else if x > r.x + r.w { x - r.x - r.w } else { 0.0 };
                    let dy = if y < r.y - r.h { r.y - r.h - y } else if y > r.y + r.d { y - r.y - r.d } else { 0.0 };
                    dx * dx + 16.0 * dy * dy
                };
                dist(a).total_cmp(&dist(b))
            })
        };
        inside.or_else(nearest).map(|r| r.line)
    }

    /// Where lines `first..end` of the main file first appear in the PDF.
    pub fn spot_of(&self, first: u32, end: u32) -> Option<PdfSpot> {
        let mut found = self.records.iter().filter(|r| r.line >= first && r.line < end).peekable();
        found.peek()?;
        let mut all: Vec<&Record> = found.collect();
        all.sort_by(|a, b| a.page.cmp(&b.page).then((a.y - a.h).total_cmp(&(b.y - b.h))));
        let top = all[0];
        // The block's height on its first page (paragraphs are several line boxes).
        let bottom = all.iter().filter(|r| r.page == top.page).map(|r| r.y + r.d).fold(top.y + top.d, f32::max);
        let left = all.iter().filter(|r| r.page == top.page && r.w > 0.0).map(|r| r.x).fold(top.x, f32::min);
        Some(PdfSpot { page: top.page, x: left, y: top.y - top.h, height: (bottom - (top.y - top.h)).max(1.0) })
    }
}

/// `tag,line:x,y[:w,h,d]` (boxes) or `tag,line:x,y[:w]` (kerns).
fn parse_record(s: &str, page: u32, boxed: bool) -> Option<(u32, Record)> {
    let mut parts = s.split(':');
    let (tag, line) = parts.next()?.split_once(',')?;
    let (x, y) = parts.next()?.split_once(',')?;
    let num = |v: &str| v.trim().parse::<f32>().ok();
    let mut r = Record { page, line: line.parse().ok()?, x: num(x)?, y: num(y)?, w: 0.0, h: 0.0, d: 0.0 };
    if boxed && let Some(dims) = parts.next() {
        let d: Vec<f32> = dims.split(',').filter_map(num).collect();
        if let [w, h, depth] = d[..] {
            (r.w, r.h, r.d) = (w, h, depth);
        }
    }
    Some((tag.parse().ok()?, r))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "SyncTeX Version:1
Input:1:./main.tex
Input:2:/usr/share/texmf/tex/latex/base/article.cls
Output:pdf
Magnification:1000
Unit:1
X Offset:0
Y Offset:0
Content:
!100
{1
[1,5:4736286,4736286:30000000,40000000,0
(1,12:4736286,6000000:20000000,500000,100000
g1,12:4736286,6000000
)
(1,14:4736286,9000000:20000000,500000,100000
x2,99:4736286,9000000
)
]
}1
{2
(1,20:4736286,5000000:20000000,500000,100000
)
}2
Postamble:
";

    #[test]
    fn maps_points_to_lines_and_back() {
        let s = SyncTex::parse(SAMPLE, "main.tex");
        let bp = |sp: f32| sp / SP_PER_BP;
        // Inside the line box of line 12 (smaller than the page's vbox from line 5).
        assert_eq!(s.line_at(1, bp(5_000_000.0), bp(5_800_000.0)), Some(12));
        assert_eq!(s.line_at(1, bp(5_000_000.0), bp(8_900_000.0)), Some(14));
        // A click in the margin of page 2 finds its only line.
        assert_eq!(s.line_at(2, 5.0, 5.0), Some(20));
        assert_eq!(s.line_at(3, 5.0, 5.0), None);
        // Records from other inputs (the class file) are ignored.
        assert!(s.records.iter().all(|r| r.line != 99));

        let spot = s.spot_of(12, 20).unwrap();
        assert_eq!(spot.page, 1);
        assert!((spot.y - bp(5_500_000.0)).abs() < 0.01, "top of line 12's box");
        assert!((spot.height - bp(9_100_000.0 - 5_500_000.0)).abs() < 0.01, "down to line 14's box");
        assert_eq!(s.spot_of(20, 30).unwrap().page, 2);
        assert_eq!(s.spot_of(30, 40), None);
    }
}
