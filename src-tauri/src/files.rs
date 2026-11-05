//! Image files used by documents (figures) and templates (title-page logos):
//! importing them into a folder, and the document's own folder.

use std::io::Cursor;
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
    let ext = source.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
    if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::other(format!("Images must be PNG, JPEG or PDF (got .{ext})")));
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

/// The folder the open document's figures are resolved against: the `.loom` file's
/// folder once saved, a temporary folder before that.
pub struct DocumentFolder {
    dir: Mutex<PathBuf>,
    _scratch: tempfile::TempDir,
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
        Ok(Self { dir: Mutex::new(scratch.path().to_path_buf()), _scratch: scratch })
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
