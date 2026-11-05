//! Image files used by documents (figures) and templates (title-page logos):
//! importing them into a folder, and the document's own folder.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::AppError;

/// Image formats XeTeX's graphicx can include.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "pdf"];

/// True for paths like `figures/a.png` that stay inside a folder.
pub fn is_safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path.split('/').all(|s| !s.is_empty() && s != "..")
}

/// Copies an image into `<root>/<subdir>/` and returns its relative path
/// (`figures/plot.png`). An existing file with different content is never replaced;
/// the copy gets a numbered name instead. 16-bit PNGs are converted to 8-bit, which
/// the TeX engine's PDF writer handles reliably.
pub fn import_image(root: &Path, subdir: &str, source: &Path) -> Result<String, AppError> {
    import_file(root, subdir, source, IMAGE_EXTENSIONS, "Images must be PNG, JPEG or PDF")
}

/// Files a publisher's LaTeX kit may contain: class, packages, styles, logos, fonts.
pub const CLASS_KIT_EXTENSIONS: &[&str] = &[
    "cls", "sty", "bst", "clo", "cfg", "def", "fd", "png", "jpg", "jpeg", "pdf", "eps", "otf", "ttf", "map", "tfm",
    "pfb", "afm", "enc", "vf",
];

/// Largest single file accepted from a kit zip (guards against zip bombs).
const MAX_KIT_FILE: u64 = 50 * 1024 * 1024;

/// Imports a publisher's kit `.zip` into `<template folder>/assets/`: class, style,
/// font and logo files. The kit's sample paper (`.tex`, its PDF and build output)
/// is skipped. Returns the imported paths (`assets/ieeeaccess.cls`…).
pub fn import_kit_zip(template_file: &Path, zip_path: &Path) -> Result<Vec<String>, AppError> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AppError::other(format!("Not a zip file: {e}")))?;
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().and_then(|f| f.name().ok().map(|n| n.to_string())))
        .collect();
    // Stems of sample documents, so "access.tex" also skips "access.pdf".
    let samples: Vec<String> = names
        .iter()
        .filter(|n| n.to_lowercase().ends_with(".tex"))
        .filter_map(|n| Path::new(n).file_stem().and_then(|s| s.to_str()).map(str::to_lowercase))
        .collect();
    let dir = template_file.parent().unwrap_or(Path::new(".")).join("assets");
    std::fs::create_dir_all(&dir)?;
    let mut imported = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(AppError::other)?;
        if entry.is_dir() || entry.size() > MAX_KIT_FILE {
            continue;
        }
        let entry_name = entry.name().map(|n| n.to_string()).unwrap_or_default();
        let Some(name) = Path::new(&entry_name).file_name().and_then(|n| n.to_str()).map(str::to_string) else {
            continue;
        };
        let lower = name.to_lowercase();
        let ext = Path::new(&lower).extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        let stem = Path::new(&lower).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        // Only the sample's compiled PDF; a class may well share its name ("journal.cls"
        // next to "journal.tex").
        let is_sample_output = ext == "pdf" && samples.contains(&stem);
        if !CLASS_KIT_EXTENSIONS.contains(&ext.as_str()) || is_sample_output || name.starts_with('.') {
            continue;
        }
        let safe: String = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '-' })
            .collect();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut (&mut entry).take(MAX_KIT_FILE), &mut bytes)?;
        std::fs::write(dir.join(&safe), bytes)?;
        imported.push(format!("assets/{safe}"));
    }
    if imported.is_empty() {
        return Err(AppError::other("No LaTeX class or style files found in that zip"));
    }
    imported.sort();
    Ok(imported)
}

/// Like [`import_image`] for any of the `allowed` extensions (names are kept as-is
/// apart from unsafe characters, since LaTeX loads class files by name).
pub fn import_file(root: &Path, subdir: &str, source: &Path, allowed: &[&str], what: &str) -> Result<String, AppError> {
    let ext = source.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
    if !allowed.contains(&ext.as_str()) {
        return Err(AppError::other(format!("{what} (got .{ext})")));
    }
    let mut bytes = std::fs::read(source)?;
    if ext == "png" {
        bytes = png_to_8bit(bytes);
    }
    let dir = root.join(subdir);
    std::fs::create_dir_all(&dir)?;
    let stem: String = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    for n in 1.. {
        let name = if n == 1 { format!("{stem}.{ext}") } else { format!("{stem}-{n}.{ext}") };
        let dest = dir.join(&name);
        match std::fs::read(&dest) {
            Ok(existing) if existing == bytes => return Ok(format!("{subdir}/{name}")),
            Ok(_) => continue,
            Err(_) => {
                std::fs::write(&dest, &bytes)?;
                return Ok(format!("{subdir}/{name}"));
            }
        }
    }
    unreachable!()
}

/// Re-encodes 16-bit PNGs as 8-bit; anything else (or anything unreadable) is
/// returned unchanged.
fn png_to_8bit(bytes: Vec<u8>) -> Vec<u8> {
    let convert = || -> Option<Vec<u8>> {
        let mut decoder = png::Decoder::new(Cursor::new(&bytes));
        decoder.set_transformations(png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().ok()?;
        if reader.info().bit_depth != png::BitDepth::Sixteen {
            return None;
        }
        let mut buf = vec![0; reader.output_buffer_size()?];
        let frame = reader.next_frame(&mut buf).ok()?;
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, frame.width, frame.height);
            encoder.set_color(frame.color_type);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&buf[..frame.buffer_size()]).ok()?;
        }
        Some(out)
    };
    convert().unwrap_or(bytes)
}

/// Copies `files` (relative paths) from `from` to `to`, e.g. a document's figures on
/// Save As. Files that already exist with the same content are skipped; a different
/// file with the same name is an error rather than being overwritten.
pub fn copy_relative(files: &[String], from: &Path, to: &Path) -> Result<(), AppError> {
    if std::fs::canonicalize(from).ok() == std::fs::canonicalize(to).ok() {
        return Ok(());
    }
    for rel in files.iter().filter(|r| is_safe_relative(r)) {
        let (src, dest) = (from.join(rel), to.join(rel));
        let Ok(bytes) = std::fs::read(&src) else { continue };
        match std::fs::read(&dest) {
            Ok(existing) if existing == bytes => {}
            Ok(_) => {
                return Err(AppError::other(format!(
                    "{} already exists with different content; choose another folder",
                    dest.display()
                )));
            }
            Err(_) => {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, bytes)?;
            }
        }
    }
    Ok(())
}

/// Copies the files that don't exist at the destination yet (figure names are never
/// reused for different images, so an existing file is already the right one).
pub fn copy_missing(files: &[String], from: &Path, to: &Path) -> std::io::Result<()> {
    for rel in files.iter().filter(|r| is_safe_relative(r)) {
        let (src, dest) = (from.join(rel), to.join(rel));
        if dest.exists() || !src.is_file() {
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dest)?;
    }
    Ok(())
}

/// The folder the open document's figures are resolved against: the `.loom` file's
/// folder once saved, a temporary folder before that.
pub struct DocumentFolder {
    dir: Mutex<PathBuf>,
    /// Temporary folders for unsaved documents, kept until the app exits.
    scratch: Mutex<Vec<tempfile::TempDir>>,
}

impl DocumentFolder {
    /// Starts with a temporary folder holding the sample document's images.
    pub fn new(seed: &[(&str, &[u8])]) -> std::io::Result<Self> {
        let scratch = tempfile::Builder::new().prefix("texloom-unsaved-").tempdir()?;
        for (rel, bytes) in seed {
            let dest = scratch.path().join(rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(dest, bytes)?;
        }
        Ok(Self { dir: Mutex::new(scratch.path().to_path_buf()), scratch: Mutex::new(vec![scratch]) })
    }

    /// Switches to a fresh temporary folder (for a new, unsaved document).
    pub fn reset(&self) -> std::io::Result<()> {
        let scratch = tempfile::Builder::new().prefix("texloom-unsaved-").tempdir()?;
        self.set(scratch.path().to_path_buf());
        self.scratch.lock().expect("folder lock").push(scratch);
        Ok(())
    }

    pub fn get(&self) -> PathBuf {
        self.dir.lock().expect("folder lock").clone()
    }

    pub fn set(&self, dir: PathBuf) {
        *self.dir.lock().expect("folder lock") = dir;
    }

    /// Reads the images a document uses, by their relative paths.
    pub fn load(&self, files: &[String]) -> Result<Vec<(String, Vec<u8>)>, AppError> {
        let dir = self.get();
        files
            .iter()
            .map(|rel| {
                if !is_safe_relative(rel) {
                    return Err(AppError::other(format!("Invalid image path: {rel}")));
                }
                let bytes = std::fs::read(dir.join(rel))
                    .map_err(|_| AppError::other(format!("Image not found: {rel} (in {})", dir.display())))?;
                Ok((rel.clone(), bytes))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(depth: png::BitDepth) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(depth);
        let data = if depth == png::BitDepth::Sixteen { vec![255u8; 16] } else { vec![255u8; 8] };
        enc.write_header().unwrap().write_image_data(&data).unwrap();
        out
    }

    #[test]
    fn sixteen_bit_pngs_become_eight_bit() {
        let converted = png_to_8bit(png(png::BitDepth::Sixteen));
        let info = png::Decoder::new(Cursor::new(&converted)).read_info().unwrap().info().clone();
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        let eight = png(png::BitDepth::Eight);
        assert_eq!(png_to_8bit(eight.clone()), eight, "8-bit files are left alone");
        assert_eq!(png_to_8bit(b"not a png".to_vec()), b"not a png");
    }

    #[test]
    fn kit_zip_import_skips_the_sample_paper() {
        use std::io::Write as _;
        let root = std::env::temp_dir().join(format!("texloom-kit-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let zip_path = root.join("kit.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
            let opts = zip::write::SimpleFileOptions::default();
            for (name, body) in [
                ("KIT_2026/journal.cls", "cls"),
                ("KIT_2026/journal.tex", "sample"),
                ("KIT_2026/journal.pdf", "sample output"),
                ("KIT_2026/logo.png", "png"),
                ("KIT_2026/t1-font.map", "map"),
                ("KIT_2026/README.txt", "readme"),
            ] {
                z.start_file(name, opts).unwrap();
                z.write_all(body.as_bytes()).unwrap();
            }
            z.finish().unwrap();
        }
        let imported = import_kit_zip(&root.join("template.json"), &zip_path).unwrap();
        assert_eq!(imported, vec!["assets/journal.cls", "assets/logo.png", "assets/t1-font.map"]);
        assert_eq!(std::fs::read(root.join("assets/journal.cls")).unwrap(), b"cls");
        std::fs::remove_dir_all(root).unwrap();
    }

    /// With IEEE's real kit: TEXLOOM_KIT_ZIP=/path/to/ieee-access.zip cargo test -p texloom -- --ignored
    #[test]
    #[ignore]
    fn real_ieee_access_kit_imports() {
        let zip_path = std::env::var("TEXLOOM_KIT_ZIP").expect("TEXLOOM_KIT_ZIP");
        let root = std::env::temp_dir().join(format!("texloom-realkit-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let imported = import_kit_zip(&root.join("template.json"), Path::new(&zip_path)).unwrap();
        for needed in ["assets/ieeeaccess.cls", "assets/spotcolor.sty", "assets/t1-formata.map", "assets/logo.png"] {
            assert!(imported.iter().any(|f| f == needed), "{needed} in {imported:?}");
        }
        assert!(!imported.iter().any(|f| f.ends_with(".tex") || f == "assets/access.pdf"), "{imported:?}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copies_figures_without_overwriting() {
        let root = std::env::temp_dir().join(format!("texloom-copy-{}", std::process::id()));
        let (a, b) = (root.join("a"), root.join("b"));
        std::fs::create_dir_all(a.join("figures")).unwrap();
        std::fs::write(a.join("figures/x.png"), b"x").unwrap();
        copy_relative(&["figures/x.png".into()], &a, &b).unwrap();
        assert_eq!(std::fs::read(b.join("figures/x.png")).unwrap(), b"x");
        std::fs::write(a.join("figures/x.png"), b"changed").unwrap();
        assert!(copy_relative(&["figures/x.png".into()], &a, &b).is_err());
        assert!(!is_safe_relative("../x.png") && !is_safe_relative("/etc/x"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
