//! Durable state snapshots and recovery of staged library entries.

use crate::{model::{Db, ModEntry}, tr, util::{now, write_json}};
use anyhow::{bail, Context, Result};
use std::path::Path;

pub fn save(data: &Path, db: &Db) -> Result<()> {
    // This snapshot is independently replaceable even when state.json is locked.
    write_json(&data.join("state.recovery.json"), db)?;
    let primary = data.join("state.json");
    if primary.is_file() {
        let bytes = std::fs::read(&primary)?;
        if let Ok(previous) = decode_db(&bytes) {
            write_json(&data.join("state.backup.json"), &previous)?;
        } else {
            // Never destroy the only copy of a damaged state during recovery.
            std::fs::copy(&primary, data.join(format!("state.damaged-{}.json", uuid::Uuid::new_v4().simple())))?;
        }
    }
    write_json(&primary, db).with_context(|| tr!(
        "Speichern fehlgeschlagen; der aktuelle Stand liegt in state.recovery.json",
        "Saving failed; the current state is available in state.recovery.json"
    ))?;
    for entry in &db.mods {
        if !valid_id(&entry.id) {
            bail!("Invalid library mod ID: {}", entry.id);
        }
        let root = data.join("mods").join(&entry.id);
        if root.join("files").is_dir() {
            let path = root.join("mod.json");
            let bytes = serde_json::to_vec_pretty(entry)?;
            if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
                write_json(&path, entry)?;
            }
        }
    }
    Ok(())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

fn decode_db(bytes: &[u8]) -> Result<Db> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    // serde accepts a positional sequence for a struct, and Db's defaults can turn
    // [] or {} into a seemingly valid empty database. Neither is a saved state.
    if !value.is_object() || !value["mods"].is_array() || !value["profiles"].is_array() {
        bail!("Invalid state structure (expected an object with mods and profiles)");
    }
    Ok(serde_json::from_value(value)?)
}

pub fn load(data: &Path, mods: &Path) -> Result<(Db, Vec<String>)> {
    let mut candidates = Vec::new();
    let mut errors = Vec::new();
    for (priority, name) in ["state.json", "state.recovery.json", "state.backup.json"].iter().enumerate() {
        let path = data.join(name);
        if !path.try_exists()? {
            continue;
        }
        match std::fs::read(&path).map_err(anyhow::Error::from).and_then(|bytes| decode_db(&bytes)) {
            Ok(db) => candidates.push((std::fs::metadata(&path)?.modified()?, std::cmp::Reverse(priority), *name, db)),
            Err(e) => errors.push(format!("{name}: {e:#}")),
        }
    }
    let mut warnings = Vec::new();
    let mut db = if candidates.is_empty() {
        if !errors.is_empty() {
            bail!("{}\n{}", tr!(
                "Kein lesbarer Datenstand gefunden. Die Dateien bleiben unverändert; bitte die Sicherung prüfen.",
                "No readable state found. Files have been left unchanged; please check your backup."
            ), errors.join("\n"));
        }
        Db::default()
    } else {
        // The backup is older content copied during save, so its new filesystem
        // timestamp must never outrank a valid primary or current recovery snapshot.
        if candidates.iter().any(|candidate| candidate.2 != "state.backup.json") {
            candidates.retain(|candidate| candidate.2 != "state.backup.json");
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let (_, _, name, db) = candidates.remove(0);
        if name != "state.json" || !errors.is_empty() {
            warnings.push(tr!("Datenstand aus {name} wiederhergestellt. Bitte Profile und Mod-Reihenfolge prüfen.", "State restored from {name}. Please review your profiles and mod order.", name = name));
        }
        db
    };
    db.normalize();
    if mods.is_dir() {
        let mut dirs = std::fs::read_dir(mods)?.collect::<std::io::Result<Vec<_>>>()?;
        dirs.sort_by_key(|d| d.file_name());
        for dir in dirs {
            let id = dir.file_name().to_string_lossy().into_owned();
            if !dir.file_type()?.is_dir() || !valid_id(&id) || db.find_mod(&id).is_some() {
                continue;
            }
            let files = dir.path().join("files");
            if !files.is_dir() || std::fs::read_dir(&files)?.next().is_none() {
                continue;
            }
            let entry = std::fs::read(dir.path().join("mod.json")).ok()
                .and_then(|bytes| serde_json::from_slice::<ModEntry>(&bytes).ok())
                .filter(|entry| entry.id == id);
            let entry = entry.unwrap_or_else(|| {
                let analysis = crate::layout::analyze(&files);
                ModEntry {
                    id: id.clone(), name: analysis.name.unwrap_or_else(|| format!("Recovered mod {}", &id[..id.len().min(8)])),
                    version: analysis.version, author: analysis.author, summary: None, category: None,
                    notes: String::new(), installed_at: now(), updated_at: now(), archive_name: None,
                    nexus: None, picture_url: None, needs_setup: !analysis.layout.groups.is_empty(),
                    layout: analysis.layout, selection: Default::default(), update: None,
                }
            });
            warnings.push(tr!("Mod „{}“ aus der Bibliothek wiederhergestellt (deaktiviert).", "Mod “{}” recovered from the library (disabled).", entry.name));
            db.mods.push(entry);
        }
    }
    db.normalize(); // recovered entries are appended disabled, including in Vanilla
    Ok((db, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModEntry;

    fn temp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("s2mm-recovery-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entry(id: &str) -> ModEntry {
        serde_json::from_value(serde_json::json!({
            "id":id,"name":"Saved mod name","version":"1.2","author":null,"summary":null,
            "installedAt":7,"updatedAt":8,"archiveName":"Original archive.zip","nexus":null,
            "pictureUrl":null,"notes":"Keep my notes","selection":{"g0":["choice"]},
            "layout":{"root":"","fomod":false,"base":[],"groups":[]}
        })).unwrap()
    }

    #[test]
    #[ignore = "requires S2MM_TEST_DATA; reads user data but writes only to a temporary folder"]
    fn real_state_roundtrips_without_touching_original() {
        let data = std::path::PathBuf::from(std::env::var_os("S2MM_TEST_DATA").expect("Set S2MM_TEST_DATA"));
        let original = std::fs::read(data.join("state.json")).unwrap();
        let (db, _) = load(&data, &data.join("mods")).unwrap();
        assert!(!db.mods.is_empty(), "Expected a populated mod library");
        let target = temp();
        save(&target, &db).unwrap();
        let (restored, _) = load(&target, &target.join("mods")).unwrap();
        assert_eq!(serde_json::to_value(&restored).unwrap(), serde_json::to_value(&db).unwrap());
        assert_eq!(std::fs::read(data.join("state.json")).unwrap(), original);
        println!("Round-trip verified: {} mods, {} profiles; original state unchanged", db.mods.len(), db.profiles.len());
        std::fs::remove_dir_all(target).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn failed_replacement_ignores_newer_backup_of_older_content() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = temp();
        let mut db = Db::default();
        db.settings.launch_args = "old".into();
        save(&dir, &db).unwrap();
        let locked = std::fs::OpenOptions::new().read(true).share_mode(1).open(dir.join("state.json")).unwrap();
        db.settings.launch_args = "new".into();
        assert!(save(&dir, &db).is_err());
        drop(locked);
        let (restored, _) = load(&dir, &dir.join("mods")).unwrap();
        assert_eq!(restored.settings.launch_args, "new", "a newly copied backup still contains old data");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_primary_write_keeps_current_settings_in_recovery() {
        let dir = temp();
        std::fs::create_dir(dir.join("state.json")).unwrap();
        let mut db = Db::default();
        db.settings.launch_args = "-saved-in-recovery".into();
        assert!(save(&dir, &db).is_err());
        let recovered: Db = crate::util::read_json(&dir.join("state.recovery.json")).unwrap();
        assert_eq!(recovered.settings.launch_args, "-saved-in-recovery");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_primary_restores_settings_and_profiles_from_recovery() {
        let dir = temp();
        let mut db = Db::default();
        db.settings.language = "en".into();
        db.profiles.push(crate::model::Profile { id: "my-profile".into(), name: "My profile".into(), mods: Vec::new() });
        db.active_profile = "my-profile".into();
        crate::util::write_json(&dir.join("state.recovery.json"), &db).unwrap();
        std::fs::write(dir.join("state.json"), b"{broken").unwrap();
        let (restored, warnings) = load(&dir, &dir.join("mods")).unwrap();
        assert_eq!(restored.settings.language, "en");
        assert!(restored.profiles.iter().any(|p| p.name == "My profile"));
        assert!(!warnings.is_empty());
        assert_eq!(std::fs::read(dir.join("state.json")).unwrap(), b"{broken");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_snapshots_never_reset_to_empty_state() {
        let dir = temp();
        std::fs::write(dir.join("state.json"), b"{broken").unwrap();
        std::fs::write(dir.join("state.recovery.json"), b"[]").unwrap();
        assert!(load(&dir, &dir.join("mods")).is_err());
        assert_eq!(std::fs::read(dir.join("state.json")).unwrap(), b"{broken");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_primary_uses_saved_recovery_instead_of_defaults() {
        let dir = temp();
        let mut db = Db::default();
        db.settings.launch_args = "-recover-me".into();
        crate::util::write_json(&dir.join("state.recovery.json"), &db).unwrap();
        let (restored, _) = load(&dir, &dir.join("mods")).unwrap();
        assert_eq!(restored.settings.launch_args, "-recover-me");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn orphan_metadata_preserves_details_and_recovers_disabled() {
        let dir = temp();
        let root = dir.join("mods/abc");
        std::fs::create_dir_all(root.join("files")).unwrap();
        std::fs::write(root.join("files/Example_P.pak"), b"content").unwrap();
        crate::util::write_json(&root.join("mod.json"), &entry("abc")).unwrap();
        let (db, warnings) = load(&dir, &dir.join("mods")).unwrap();
        let found = db.find_mod("abc").expect("staged mod should be recovered");
        assert_eq!(found.name, "Saved mod name");
        assert_eq!(found.notes, "Keep my notes");
        assert_eq!(found.selection["g0"], vec!["choice"]);
        assert!(db.profiles.iter().all(|p| p.mods.iter().any(|m| m.id == "abc" && !m.enabled)));
        assert!(!warnings.is_empty());
        assert_eq!(std::fs::read(root.join("files/Example_P.pak")).unwrap(), b"content");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn legacy_orphan_is_reanalysed_without_moving_content() {
        let dir = temp();
        let root = dir.join("mods/legacy");
        std::fs::create_dir_all(root.join("files")).unwrap();
        std::fs::write(root.join("files/Legacy_P.pak"), b"old content").unwrap();
        let (db, _) = load(&dir, &dir.join("mods")).unwrap();
        let recovered = db.find_mod("legacy").expect("legacy library content should be recovered");
        assert!(!recovered.layout.base.is_empty());
        assert!(db.profiles.iter().all(|p| p.mods.iter().all(|m| !m.enabled)));
        assert_eq!(std::fs::read(root.join("files/Legacy_P.pak")).unwrap(), b"old content");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
