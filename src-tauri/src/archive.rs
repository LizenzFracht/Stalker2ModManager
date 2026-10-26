//! Extracting mod archives (zip / 7z / rar) or copying loose files into staging.

use crate::tr;

use crate::util::{copy_dir, ext_lower};
use anyhow::{bail, Context, Result};
use std::path::Path;

pub const ARCHIVE_EXTS: [&str; 3] = ["zip", "7z", "rar"];
pub const PAK_EXTS: [&str; 4] = ["pak", "utoc", "ucas", "sig"];

pub fn is_supported(path: &Path) -> bool {
    if path.is_dir() {
        return true;
    }
    let ext = ext_lower(&path.to_string_lossy());
    ARCHIVE_EXTS.contains(&ext.as_str())
        || PAK_EXTS.contains(&ext.as_str())
        || matches!(ext.as_str(), "dll" | "asi")
}

/// Puts the content of `src` (archive, folder or loose file) into `dest`.
pub fn unpack(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    if src.is_dir() {
        let name = src.file_name().context(tr!("Ungültiger Ordner", "Invalid folder"))?;
        return copy_dir(src, &dest.join(name));
    }
    let ext = ext_lower(&src.to_string_lossy());
    match ext.as_str() {
        "zip" => {
            let file = std::fs::File::open(src)?;
            let mut zip = zip::ZipArchive::new(file).context(tr!("ZIP-Archiv ist beschädigt", "ZIP archive is damaged"))?;
            zip.extract(dest).context(tr!("ZIP entpacken", "Extracting ZIP"))?;
        }
        "7z" => {
            sevenz_rust2::decompress_file(src, dest).map_err(|e| anyhow::anyhow!(tr!("7z entpacken: {e}", "Extracting 7z: {e}", e = e)))?;
        }
        "rar" => {
            let mut archive = unrar::Archive::new(src)
                .open_for_processing()
                .map_err(|e| anyhow::anyhow!(tr!("RAR öffnen: {e}", "Opening RAR: {e}", e = e)))?;
            while let Some(header) = archive.read_header().map_err(|e| anyhow::anyhow!(tr!("RAR lesen: {e}", "Reading RAR: {e}", e = e)))? {
                archive = if header.entry().is_file() {
                    header
                        .extract_with_base(dest)
                        .map_err(|e| anyhow::anyhow!(tr!("RAR entpacken: {e}", "Extracting RAR: {e}", e = e)))?
                } else {
                    header.skip().map_err(|e| anyhow::anyhow!(tr!("RAR lesen: {e}", "Reading RAR: {e}", e = e)))?
                };
            }
        }
        e if PAK_EXTS.contains(&e) => {
            // A loose pak: bring its IoStore siblings (same stem) along.
            let stem = src.file_stem().context(tr!("Ungültiger Dateiname", "Invalid file name"))?.to_string_lossy().to_lowercase();
            let dir = src.parent().context(tr!("Ungültiger Pfad", "Invalid path"))?;
            for entry in std::fs::read_dir(dir)?.flatten() {
                let p = entry.path();
                let pe = ext_lower(&p.to_string_lossy());
                let ps = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
                if p.is_file() && ps == stem && PAK_EXTS.contains(&pe.as_str()) {
                    std::fs::copy(&p, dest.join(entry.file_name()))?;
                }
            }
        }
        "dll" | "asi" => {
            std::fs::copy(src, dest.join(src.file_name().unwrap()))?;
        }
        _ => bail!(tr!("Nicht unterstütztes Dateiformat: .{ext}", "Unsupported file format: .{ext}", ext = ext)),
    }
    Ok(())
}
