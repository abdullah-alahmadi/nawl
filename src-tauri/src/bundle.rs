//! Single-file documents: a `.loom` that is a zip holding the document
//! (`document.loom`, the usual JSON) and the images it uses, so it can be emailed or
//! uploaded as one file. Plain JSON `.loom` files (images in a folder next to them)
//! stay the default; both open the same way.

use std::io::{Read, Write};
use std::path::Path;

use texloom_core::tr;

use crate::{files, AppError};

/// The document inside a single-file `.loom`.
const DOCUMENT: &str = "document.loom";
/// Largest image taken from a single-file document.
const MAX_FILE: u64 = 100 * 1024 * 1024;

/// Whether `bytes` start like a zip file (a single-file document).
pub fn is_bundle(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
}

/// Writes the document's JSON and its images (relative paths) into one zip file.
/// Written next to `path` first and then moved, so a failed save never leaves a
/// broken file behind.
pub fn write(path: &Path, json: &str, images: &[(String, Vec<u8>)]) -> Result<(), AppError> {
    let tmp = path.with_extension("loom.part");
    let result = (|| -> Result<(), AppError> {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp)?);
        let deflate = zip::write::SimpleFileOptions::default();
        zip.start_file(DOCUMENT, deflate).map_err(AppError::other)?;
        zip.write_all(json.as_bytes())?;
        // Images are compressed already: stored as they are.
        let stored = deflate.compression_method(zip::CompressionMethod::Stored);
        for (rel, bytes) in images {
            zip.start_file(rel.as_str(), stored).map_err(AppError::other)?;
            zip.write_all(bytes)?;
        }
        zip.finish().map_err(AppError::other)?;
        Ok(())
    })();
    match result {
        Ok(()) => Ok(std::fs::rename(&tmp, path)?),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// The document's JSON and its images from a single-file `.loom`.
pub fn read(path: &Path) -> Result<(String, Vec<(String, Vec<u8>)>), AppError> {
    let bad = |e: zip::result::ZipError| AppError::other(tr!("Not a valid Texloom document: {e}", "ليس مستند Texloom صالحًا: {e}"));
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?).map_err(bad)?;
    let mut json = None;
    let mut images = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(bad)?;
        let name: String = entry.name().map(|n| n.to_string()).unwrap_or_default();
        if entry.is_dir() || entry.size() > MAX_FILE {
            continue;
        }
        let mut bytes = Vec::new();
        (&mut entry).take(MAX_FILE).read_to_end(&mut bytes)?;
        if name == DOCUMENT {
            json = Some(String::from_utf8(bytes).map_err(|_| AppError::other(tr!("Not a valid Texloom document", "ليس مستند Texloom صالحًا")))?);
        } else if files::is_safe_relative(&name) {
            images.push((name, bytes));
        }
    }
    let json = json.ok_or_else(|| AppError::other(tr!("Not a Texloom document (no {DOCUMENT} inside)", "ليس مستند Texloom (لا يحتوي على {DOCUMENT})")))?;
    Ok((json, images))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_skips_unsafe_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.loom");
        let images = vec![("figures/a.png".to_string(), b"png".to_vec())];
        write(&path, "{\"format\":\"texloom\"}", &images).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(is_bundle(&bytes));
        assert!(!dir.path().join("report.loom.part").exists());
        let (json, read_images) = read(&path).unwrap();
        assert_eq!(json, "{\"format\":\"texloom\"}");
        assert_eq!(read_images, images);
        assert!(!is_bundle(b"{\"format\""));

        // A hand-made zip with a path outside the folder: that entry is ignored.
        let evil = dir.path().join("evil.loom");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
        z.start_file("document.loom", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(b"{}").unwrap();
        z.start_file("../outside.png", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(b"x").unwrap();
        z.finish().unwrap();
        assert!(read(&evil).unwrap().1.is_empty());
    }
}
