//! Reading and editing Unreal SaveGame (GVAS) files such as MCM_Settings.sav.
//! The save is converted to JSON (values separated from type schemas), edited by JSON
//! pointer and written back byte-exact for untouched data.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SaveFileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: u64,
    pub is_mcm: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Leaf {
    /// JSON pointer below the save JSON.
    pub pointer: String,
    /// Human readable path segments (struct/array labels).
    pub path: Vec<String>,
    pub label: String,
    /// bool | int | float | string | key
    pub kind: String,
    pub value: Value,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SaveView {
    pub save_type: String,
    pub leaves: Vec<Leaf>,
}

#[derive(Deserialize, Debug)]
pub struct Edit {
    pub pointer: String,
    pub value: Value,
}

/// `%LOCALAPPDATA%\Stalker2\Saved\SaveGames` — where MCM and other mods keep their saves.
pub fn save_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join("Stalker2/Saved/SaveGames"))
}

pub fn list() -> Vec<SaveFileInfo> {
    let Some(dir) = save_dir() else { return Vec::new() };
    let mut out: Vec<SaveFileInfo> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| crate::util::ext_lower(&e.file_name().to_string_lossy()) == "sav")
                .map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let (size, modified) = crate::util::file_meta(&e.path());
                    SaveFileInfo {
                        is_mcm: name.to_lowercase().starts_with("mcm"),
                        name,
                        path: e.path().to_string_lossy().into_owned(),
                        size,
                        modified,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| b.is_mcm.cmp(&a.is_mcm).then(a.name.cmp(&b.name)));
    out
}

pub fn load(path: &Path) -> Result<Value> {
    let bytes = std::fs::read(path).with_context(|| format!("{} lesen", path.display()))?;
    let save = uesave::SaveReader::new()
        .error_to_raw(true)
        .read(std::io::Cursor::new(&bytes))
        .map_err(|e| anyhow!("SaveGame nicht lesbar: {e}"))?;
    Ok(serde_json::to_value(&save)?)
}

pub fn write(path: &Path, json: &Value) -> Result<()> {
    let save: uesave::Save = serde_json::from_value(json.clone()).context("SaveGame-Daten ungültig")?;
    let mut out = Vec::new();
    save.write(&mut out).map_err(|e| anyhow!("SaveGame schreiben: {e}"))?;
    let backup = path.with_extension("sav.s2mm-backup");
    if path.exists() && !backup.exists() {
        std::fs::copy(path, &backup)?;
    }
    let tmp = path.with_extension("sav.tmp");
    std::fs::write(&tmp, out)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn apply_edits(path: &Path, edits: &[Edit]) -> Result<()> {
    let mut json = load(path)?;
    for e in edits {
        let slot = json.pointer_mut(&e.pointer).ok_or_else(|| anyhow!("Eintrag {} nicht gefunden", e.pointer))?;
        // Keep the JSON type so the schema still matches.
        let ok = matches!(
            (&*slot, &e.value),
            (Value::Bool(_), Value::Bool(_)) | (Value::String(_), Value::String(_)) | (Value::Number(_), Value::Number(_))
        );
        if !ok {
            bail!("Typ von {} darf nicht geändert werden", e.pointer);
        }
        if slot.is_i64() || slot.is_u64() {
            let n = e.value.as_f64().unwrap_or(0.0).round();
            *slot = if slot.is_u64() && n >= 0.0 { Value::from(n as u64) } else { Value::from(n as i64) };
        } else {
            *slot = e.value.clone();
        }
    }
    write(path, &json)
}

/// Blueprint struct members look like `SettingName_12_4F3C...` plus uesave's `_0` index.
pub fn clean_name(key: &str) -> String {
    let mut s = key.to_string();
    if let Some(stripped) = s.strip_suffix("_0") {
        s = stripped.to_string();
    }
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() >= 3 {
        let last = parts[parts.len() - 1];
        let num = parts[parts.len() - 2];
        if last.len() == 32 && last.chars().all(|c| c.is_ascii_hexdigit()) && num.chars().all(|c| c.is_ascii_digit()) {
            s = parts[..parts.len() - 2].join("_");
        }
    }
    s
}

fn escape(seg: &str) -> String {
    seg.replace('~', "~0").replace('/', "~1")
}

/// A label for an array element: its first string member that looks like a name.
fn element_label(v: &Value, index: usize) -> String {
    if let Value::Object(map) = v {
        let mut fallback = None;
        for (k, val) in map {
            if let Value::String(s) = val {
                let n = clean_name(k).to_lowercase();
                if (n.contains("name") || n.contains("title") || n.contains("id") || n.contains("mod")) && !s.is_empty() {
                    return s.clone();
                }
                if fallback.is_none() && !s.is_empty() && s.len() < 60 {
                    fallback = Some(s.clone());
                }
            }
        }
        if let Some(f) = fallback {
            return f;
        }
    }
    format!("#{}", index + 1)
}

fn walk(v: &Value, pointer: &str, path: &mut Vec<String>, out: &mut Vec<Leaf>) {
    match v {
        Value::Object(map) => {
            // An FKey struct: { KeyName: "F5" }
            if map.len() == 1 {
                if let Some((k, Value::String(s))) = map.iter().next() {
                    if clean_name(k).eq_ignore_ascii_case("KeyName") {
                        out.push(Leaf {
                            pointer: format!("{pointer}/{}", escape(k)),
                            label: path.last().cloned().unwrap_or_else(|| "Taste".into()),
                            path: path[..path.len().saturating_sub(1)].to_vec(),
                            kind: "key".into(),
                            value: Value::String(s.clone()),
                        });
                        return;
                    }
                }
            }
            for (k, val) in map {
                path.push(clean_name(k));
                walk(val, &format!("{pointer}/{}", escape(k)), path, out);
                path.pop();
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                path.push(element_label(item, i));
                walk(item, &format!("{pointer}/{i}"), path, out);
                path.pop();
            }
        }
        Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            let kind = match v {
                Value::Bool(_) => "bool",
                Value::Number(n) if n.is_f64() => "float",
                Value::Number(_) => "int",
                _ => "string",
            };
            out.push(Leaf {
                pointer: pointer.to_string(),
                label: path.last().cloned().unwrap_or_default(),
                path: path[..path.len().saturating_sub(1)].to_vec(),
                kind: kind.into(),
                value: v.clone(),
            });
        }
        Value::Null => {}
    }
}

pub fn view(json: &Value) -> SaveView {
    let mut leaves = Vec::new();
    if let Some(props) = json.pointer("/root/properties") {
        walk(props, "/root/properties", &mut Vec::new(), &mut leaves);
    }
    SaveView {
        save_type: json.pointer("/root/save_game_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        leaves,
    }
}

pub fn restore_backup(path: &Path) -> Result<()> {
    let backup = path.with_extension("sav.s2mm-backup");
    if !backup.exists() {
        bail!("Keine Sicherung vorhanden");
    }
    std::fs::copy(&backup, path)?;
    std::fs::remove_file(&backup)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_blueprint_names() {
        assert_eq!(clean_name("SettingName_12_4F3C8A9B0C1D2E3F4A5B6C7D8E9F0A1B_0"), "SettingName");
        assert_eq!(clean_name("VersionNumber_0"), "VersionNumber");
    }

    #[test]
    fn edits_sample_save() {
        let dir = std::fs::read_dir(std::path::PathBuf::from(std::env::var("CARGO_HOME").unwrap_or_else(|_| {
            format!("{}/.cargo", std::env::var("USERPROFILE").unwrap_or_default())
        })).join("registry/src"));
        let Some(sample) = dir.ok().and_then(|rd| {
            rd.flatten().map(|e| e.path().join("uesave-0.7.1/drg-save-test.sav")).find(|p| p.exists())
        }) else {
            return;
        };
        let tmp = std::env::temp_dir().join(format!("s2mm-gvas-{}.sav", std::process::id()));
        std::fs::copy(&sample, &tmp).unwrap();
        let v = view(&load(&tmp).unwrap());
        let leaf = v.leaves.iter().find(|l| l.label == "VersionNumber").unwrap().clone();
        apply_edits(&tmp, &[Edit { pointer: leaf.pointer.clone(), value: serde_json::json!(7) }]).unwrap();
        let again = load(&tmp).unwrap();
        assert_eq!(again.pointer(&leaf.pointer).unwrap(), &serde_json::json!(7));
        assert!(tmp.with_extension("sav.s2mm-backup").exists());
        let _ = std::fs::remove_file(tmp.with_extension("sav.s2mm-backup"));
        let _ = std::fs::remove_file(&tmp);
    }
}
