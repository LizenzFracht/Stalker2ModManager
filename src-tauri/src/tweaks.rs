//! Engine.ini [SystemSettings] tweaks. The original file is backed up once before the
//! first change and can be restored at any time.

use crate::tr;

use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const SECTION: &str = "[SystemSettings]";
const BACKUP: &str = "Engine.ini.s2mm-backup";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TweakState {
    pub config_dir: Option<String>,
    pub engine_ini_exists: bool,
    pub backup_exists: bool,
    pub read_only: bool,
    /// Current key/value pairs of [SystemSettings].
    pub values: BTreeMap<String, String>,
}

fn engine_ini(dir: &Path) -> PathBuf {
    dir.join("Engine.ini")
}

fn section_bounds(lines: &[String]) -> Option<(usize, usize)> {
    let start = lines.iter().position(|l| l.trim().eq_ignore_ascii_case(SECTION))?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.trim().starts_with('['))
        .map(|i| start + 1 + i)
        .unwrap_or(lines.len());
    Some((start, end))
}

fn read_lines(p: &Path) -> Vec<String> {
    std::fs::read(p)
        .map(|b| {
            let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&b).to_vec();
            String::from_utf8_lossy(&b).lines().map(|l| l.to_string()).collect()
        })
        .unwrap_or_default()
}

pub fn read(dir: Option<&Path>) -> TweakState {
    let Some(dir) = dir else {
        return TweakState { config_dir: None, engine_ini_exists: false, backup_exists: false, read_only: false, values: BTreeMap::new() };
    };
    let ini = engine_ini(dir);
    let lines = read_lines(&ini);
    let mut values = BTreeMap::new();
    if let Some((s, e)) = section_bounds(&lines) {
        for l in &lines[s + 1..e] {
            let t = l.trim();
            if t.starts_with(';') || t.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = t.split_once('=') {
                values.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    TweakState {
        config_dir: Some(dir.to_string_lossy().into_owned()),
        engine_ini_exists: ini.exists(),
        backup_exists: dir.join(BACKUP).exists(),
        read_only: std::fs::metadata(&ini).map(|m| m.permissions().readonly()).unwrap_or(false),
        values,
    }
}

/// Sets (Some) or removes (None) keys in [SystemSettings]; `protect` leaves the file
/// read-only afterwards so the game cannot overwrite it on exit.
pub fn write(dir: &Path, changes: &BTreeMap<String, Option<String>>, protect: bool) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let ini = engine_ini(dir);
    let backup = dir.join(BACKUP);
    if ini.exists() && !backup.exists() {
        std::fs::copy(&ini, &backup).context(tr!("Sicherung von Engine.ini anlegen", "Backing up Engine.ini"))?;
    }
    let mut lines = read_lines(&ini);
    let (start, mut end) = match section_bounds(&lines) {
        Some(b) => b,
        None => {
            if lines.last().map(|l| !l.trim().is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            lines.push(SECTION.to_string());
            (lines.len() - 1, lines.len())
        }
    };
    for (key, value) in changes {
        let pos = lines[start + 1..end].iter().position(|l| {
            l.split_once('=').map(|(k, _)| k.trim().eq_ignore_ascii_case(key)).unwrap_or(false)
        });
        match (pos, value) {
            (Some(i), Some(v)) => lines[start + 1 + i] = format!("{key}={v}"),
            (Some(i), None) => {
                lines.remove(start + 1 + i);
                end -= 1;
            }
            (None, Some(v)) => {
                // Insert before trailing blank lines of the section.
                let mut at = end;
                while at > start + 1 && lines[at - 1].trim().is_empty() {
                    at -= 1;
                }
                lines.insert(at, format!("{key}={v}"));
                end += 1;
            }
            (None, None) => {}
        }
    }
    set_readonly(&ini, false)?;
    std::fs::write(&ini, lines.join("\r\n") + "\r\n").context(tr!("Engine.ini schreiben", "Writing Engine.ini"))?;
    set_readonly(&ini, protect)?;
    Ok(())
}

pub fn set_readonly(p: &Path, ro: bool) -> Result<()> {
    if !p.exists() {
        return Ok(());
    }
    let mut perm = std::fs::metadata(p)?.permissions();
    perm.set_readonly(ro);
    std::fs::set_permissions(p, perm)?;
    Ok(())
}

pub fn export(dir: &Path, dest: &Path) -> Result<()> {
    let ini = engine_ini(dir);
    if !ini.exists() {
        anyhow::bail!(tr!("Engine.ini existiert noch nicht", "Engine.ini does not exist yet"));
    }
    std::fs::copy(&ini, dest).context(tr!("Engine.ini exportieren", "Exporting Engine.ini"))?;
    Ok(())
}

/// Parses an ini into (section, key, value) triples in file order.
fn parse_ini(lines: &[String]) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut section = String::new();
    for l in lines {
        let t = l.trim();
        if t.starts_with('[') && t.ends_with(']') {
            section = t.to_string();
        } else if !t.is_empty() && !t.starts_with(';') && !t.starts_with('#') {
            if let Some((k, v)) = t.split_once('=') {
                if !section.is_empty() {
                    out.push((section.clone(), k.trim().to_string(), v.trim().to_string()));
                }
            }
        }
    }
    out
}

/// Imports an ini file. `replace` overwrites Engine.ini; otherwise every key of every
/// section is merged in (array entries starting with + - . are appended once).
pub fn import(dir: &Path, src: &Path, replace: bool, protect: bool) -> Result<usize> {
    let incoming = read_lines(src);
    let entries = parse_ini(&incoming);
    if entries.is_empty() {
        anyhow::bail!(tr!("Die Datei enthält keine ini-Einträge", "The file contains no ini entries"));
    }
    std::fs::create_dir_all(dir)?;
    let ini = engine_ini(dir);
    let backup = dir.join(BACKUP);
    if ini.exists() && !backup.exists() {
        std::fs::copy(&ini, &backup).context(tr!("Sicherung von Engine.ini anlegen", "Backing up Engine.ini"))?;
    }
    set_readonly(&ini, false)?;
    if replace {
        std::fs::write(&ini, incoming.join("\r\n") + "\r\n")?;
    } else {
        let mut lines = read_lines(&ini);
        for (section, key, value) in &entries {
            let start = match lines.iter().position(|l| l.trim().eq_ignore_ascii_case(section)) {
                Some(s) => s,
                None => {
                    if lines.last().map(|l| !l.trim().is_empty()).unwrap_or(false) {
                        lines.push(String::new());
                    }
                    lines.push(section.clone());
                    lines.len() - 1
                }
            };
            let end = lines[start + 1..].iter().position(|l| l.trim().starts_with('[')).map(|i| start + 1 + i).unwrap_or(lines.len());
            let line = format!("{key}={value}");
            let array = key.starts_with(['+', '-', '.']);
            let existing = lines[start + 1..end].iter().position(|l| {
                if array {
                    l.trim() == line
                } else {
                    l.split_once('=').map(|(k, _)| k.trim().eq_ignore_ascii_case(key)).unwrap_or(false)
                }
            });
            match existing {
                Some(i) => lines[start + 1 + i] = line,
                None => {
                    let mut at = end;
                    while at > start + 1 && lines[at - 1].trim().is_empty() {
                        at -= 1;
                    }
                    lines.insert(at, line);
                }
            }
        }
        std::fs::write(&ini, lines.join("\r\n") + "\r\n")?;
    }
    set_readonly(&ini, protect)?;
    Ok(entries.len())
}

pub fn restore(dir: &Path) -> Result<()> {
    let backup = dir.join(BACKUP);
    if !backup.exists() {
        anyhow::bail!(tr!("Keine Sicherung vorhanden", "No backup available"));
    }
    let ini = engine_ini(dir);
    set_readonly(&ini, false)?;
    std::fs::copy(&backup, &ini)?;
    std::fs::remove_file(&backup)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_ini_can_be_edited_imported_and_restored() {
        let dir = std::env::temp_dir().join(format!("s2mm-ini-protection-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let ini = dir.join("Engine.ini");
        let original = "[Core.System]\r\nPaths=original\r\n";
        std::fs::write(&ini, original).unwrap();
        write(&dir, &BTreeMap::from([("r.FilmGrain".into(), Some("0".into()))]), true).unwrap();
        assert!(std::fs::metadata(&ini).unwrap().permissions().readonly());
        write(&dir, &BTreeMap::from([("r.FilmGrain".into(), Some("1".into()))]), true).unwrap();
        assert_eq!(read(Some(&dir)).values["r.FilmGrain"], "1");
        let imported = dir.join("import.ini");
        std::fs::write(&imported, "[SystemSettings]\nr.FilmGrain=0\n").unwrap();
        import(&dir, &imported, true, true).unwrap();
        assert!(read(Some(&dir)).read_only);
        assert_eq!(read(Some(&dir)).values["r.FilmGrain"], "0");
        restore(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), original);
        assert!(!read(Some(&dir)).read_only);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn merges_imported_ini() {
        let dir = std::env::temp_dir().join(format!("s2mm-ini-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Engine.ini"), "[Core.System]\r\nPaths=../x\r\n\r\n[SystemSettings]\r\nr.FilmGrain=1\r\n").unwrap();
        let src = dir.join("guide.ini");
        std::fs::write(&src, "[SystemSettings]\nr.FilmGrain=0\nr.Tonemapper.Sharpen=0.5\n\n[/Script/Engine.RendererSettings]\nr.DefaultFeature.MotionBlur=False\n").unwrap();
        assert_eq!(import(&dir, &src, false, false).unwrap(), 3);
        let text = std::fs::read_to_string(dir.join("Engine.ini")).unwrap();
        assert!(text.contains("Paths=../x"));
        assert!(text.contains("r.FilmGrain=0") && !text.contains("r.FilmGrain=1"));
        assert!(text.contains("r.Tonemapper.Sharpen=0.5"));
        assert!(text.contains("[/Script/Engine.RendererSettings]\r\nr.DefaultFeature.MotionBlur=False"));
        assert!(dir.join(BACKUP).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
