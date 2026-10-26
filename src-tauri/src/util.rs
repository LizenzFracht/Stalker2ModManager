use crate::tr;
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn slugify(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
        if out.len() >= max {
            break;
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "mod".into()
    } else {
        out
    }
}

/// Forward-slash relative path of `p` below `base`.
pub fn rel_path(base: &Path, p: &Path) -> String {
    let r = p.strip_prefix(base).unwrap_or(p);
    r.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn join_rel(base: &Path, rel: &str) -> PathBuf {
    let mut p = base.to_path_buf();
    for seg in rel.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        p.push(seg);
    }
    p
}

/// Joins two logical paths with a single slash.
pub fn join_logical(a: &str, b: &str) -> String {
    let a = a.trim_matches(['/', '\\']);
    let b = b.trim_matches(['/', '\\']);
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.replace('\\', "/"),
        (_, true) => a.replace('\\', "/"),
        _ => format!("{}/{}", a, b).replace('\\', "/"),
    }
}

pub fn file_meta(p: &Path) -> (u64, u64) {
    match std::fs::metadata(p) {
        Ok(m) => {
            let mtime = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (m.len(), mtime)
        }
        Err(_) => (0, 0),
    }
}

pub fn read_json<T: DeserializeOwned + Default>(p: &Path) -> Result<T> {
    if !p.exists() {
        return Ok(T::default());
    }
    let data = std::fs::read(p).with_context(|| tr!("{} lesen", "Reading {}", p.display()))?;
    serde_json::from_slice(&data).with_context(|| tr!("{} ist beschädigt", "{} is damaged", p.display()))
}

/// Atomic JSON write. Flush the unique temporary file before replacement and tolerate
/// short Windows sharing locks (e.g. another process reading/scanning the old file).
pub fn write_json<T: Serialize>(p: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    write_bytes(p, &bytes)
}

pub fn write_bytes(p: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = p.with_extension(format!("json.{}.tmp", uuid::Uuid::new_v4().simple()));
    let result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        for attempt in 0..6 {
            match std::fs::rename(&tmp, p) {
                Ok(()) => return Ok(()),
                Err(e) if cfg!(windows) && attempt < 5 && matches!(e.raw_os_error(), Some(5 | 32 | 33)) => {
                    std::thread::sleep(std::time::Duration::from_millis(20 << attempt));
                }
                Err(e) => return Err(e).with_context(|| tr!("{} schreiben", "Writing {}", p.display())),
            }
        }
        unreachable!()
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Resolves a relative path case-insensitively below `root` (FOMOD paths are often sloppy).
pub fn resolve_ci(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for seg in rel.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        let direct = cur.join(seg);
        if direct.exists() {
            cur = direct;
            continue;
        }
        let lower = seg.to_lowercase();
        let found = std::fs::read_dir(&cur).ok()?.flatten().find(|e| {
            e.file_name().to_string_lossy().to_lowercase() == lower
        })?;
        cur = found.path();
    }
    Some(cur)
}

pub fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let target = dst.join(entry.path().strip_prefix(src)?);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub fn dir_size(p: &Path) -> u64 {
    walkdir::WalkDir::new(p)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Runs a command without flashing a console window.
#[allow(dead_code)]
pub fn hidden_command(program: &str) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

pub fn is_text_ext(ext: &str) -> bool {
    matches!(
        ext,
        "ini" | "cfg" | "lua" | "json" | "txt" | "toml" | "yaml" | "yml" | "xml" | "md" | "log"
    )
}

pub fn ext_lower(p: &str) -> String {
    Path::new(p)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn json_write_waits_for_temporary_windows_sharing_lock() {
        let dir = std::env::temp_dir().join(format!("s2mm-sharing-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(&path, b"{\"generation\":1}").unwrap();
        let locked = std::fs::OpenOptions::new().read(true).share_mode(1).open(&path).unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            drop(locked);
        });
        let result = write_json(&path, &serde_json::json!({"generation":2}));
        release.join().unwrap();
        result.expect("temporary sharing locks must be retried");
        let stored: serde_json::Value = read_json(&path).unwrap();
        assert_eq!(stored["generation"], 2);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "temporary files must be cleaned up");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
