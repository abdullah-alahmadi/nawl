//! Reads the SyncTeX file TeX writes next to the PDF (`main.synctex.gz`), which says
//! which line of which input made each box on each page. Nawl needs the main
//! `.tex` file's lines (they map to the editor's blocks), and for a project also its
//! chapter files' lines.
//!
//! Positions are in PDF points (1/72 in) from the page's top-left corner, the same
//! as pdf.js's viewport at scale 1.

use std::io::Read;
use std::path::Path;

/// TeX's scaled points per PDF point (65536 sp per TeX point, 72.27 TeX points per inch).
const SP_PER_BP: f32 = 65536.0 * 72.27 / 72.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Record {
    /// Which of the files asked for (0: the main file).
    file: usize,
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

/// The records of the main file (and of any other files asked for) from a SyncTeX file.
#[derive(Debug, Clone, Default)]
pub struct SyncTex {
    records: Vec<Record>,
}

impl SyncTex {
    /// Reads `main.synctex.gz` (or an uncompressed `main.synctex`) from a build folder.
    pub fn read(dir: &Path, main: &str) -> Option<Self> {
        Some(Self::parse(&Self::read_text(dir, main)?, &format!("{main}.tex")))
    }

    /// Like [`Self::read`], also keeping the records of `others` (paths relative to the
    /// build folder, like `chapters/ch01.tex`), which become files 1, 2….
    pub fn read_with(dir: &Path, main: &str, others: &[String]) -> Option<Self> {
        let main_file = format!("{main}.tex");
        let mut files = vec![main_file.as_str()];
        files.extend(others.iter().map(String::as_str));
        Some(Self::parse_files(&Self::read_text(dir, main)?, &files))
    }

    fn read_text(dir: &Path, main: &str) -> Option<String> {
        let gz = dir.join(format!("{main}.synctex.gz"));
        if gz.is_file() {
            let mut s = String::new();
            flate2::read::GzDecoder::new(std::fs::File::open(gz).ok()?).read_to_string(&mut s).ok()?;
            Some(s)
        } else {
            std::fs::read_to_string(dir.join(format!("{main}.synctex"))).ok()
        }
    }

    /// Parses SyncTeX text, keeping the records of the input whose path ends with
    /// `main_file`.
    pub fn parse(text: &str, main_file: &str) -> Self {
        Self::parse_files(text, &[main_file])
    }

    /// Parses SyncTeX text, keeping the records of the inputs whose paths end with one
    /// of `files` (file `i` is `files[i]`).
    pub fn parse_files(text: &str, files: &[&str]) -> Self {
        // SyncTeX tag → index into `files`.
        let mut tags: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        let (mut unit, mut magnification, mut x_offset, mut y_offset) = (1.0f32, 1000.0f32, 0.0f32, 0.0f32);
        let mut page = 0u32;
        let mut raw: Vec<(u32, Record)> = Vec::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("Input:") {
                if let Some((tag, path)) = rest.split_once(':') {
                    let path = path.replace('\\', "/");
                    let file = files.iter().position(|f| path == *f || path.ends_with(&format!("/{f}")));
                    if let (Some(file), Ok(tag)) = (file, tag.parse()) {
                        tags.insert(tag, file);
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
            .filter_map(|(tag, r)| Some((*tags.get(&tag)?, r)))
            .map(|(file, r)| Record {
                file,
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
        self.find_at(page, x, y, Some(0)).map(|(_, line)| line)
    }

    /// Like [`Self::line_at`] among all the files read: (file, line).
    pub fn place_at(&self, page: u32, x: f32, y: f32) -> Option<(usize, u32)> {
        self.find_at(page, x, y, None)
    }

    fn find_at(&self, page: u32, x: f32, y: f32, file: Option<usize>) -> Option<(usize, u32)> {
        let on_page = || self.records.iter().filter(move |r| r.page == page && file.is_none_or(|f| r.file == f));
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
        inside.or_else(nearest).map(|r| (r.file, r.line))
    }

    /// Where lines `first..end` of the main file first appear in the PDF.
    pub fn spot_of(&self, first: u32, end: u32) -> Option<PdfSpot> {
        self.spot_in(0, first, end)
    }

    /// Where lines `first..end` of file `file` first appear in the PDF.
    pub fn spot_in(&self, file: usize, first: u32, end: u32) -> Option<PdfSpot> {
        let mut found = self.records.iter().filter(|r| r.file == file && r.line >= first && r.line < end).peekable();
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
    let mut r = Record { file: 0, page, line: line.parse().ok()?, x: num(x)?, y: num(y)?, w: 0.0, h: 0.0, d: 0.0 };
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

    #[test]
    fn keeps_chapter_files_of_a_project() {
        // Page 1 from the main file (title page), page 2 from a chapter file.
        let text = SAMPLE
            .replace("Input:2:/usr/share/texmf/tex/latex/base/article.cls", "Input:2:/usr/share/texmf/tex/latex/base/article.cls\nInput:3:./chapters/ch01.tex")
            .replace("(1,20:", "(3,7:");
        let s = SyncTex::parse_files(&text, &["main.tex", "chapters/ch01.tex"]);
        assert_eq!(s.place_at(2, 5.0, 5.0), Some((1, 7)));
        assert_eq!(s.line_at(2, 5.0, 5.0), None, "line_at is the main file's");
        assert_eq!(s.place_at(1, 5.0, 5.0).map(|(f, _)| f), Some(0));
        assert_eq!(s.spot_in(1, 7, 8).unwrap().page, 2);
        assert_eq!(s.spot_of(7, 8), None);
        // Read on its own, the main file ignores the chapter.
        assert!(SyncTex::parse(&text, "main.tex").records.iter().all(|r| r.page == 1));
    }
}
