//! Staging a mod: unpack it into the library, analyse it and produce a ModEntry.

use crate::tr;

use crate::archive;
use crate::layout::{analyze, Analysis};
use crate::model::{ModEntry, NexusRef};
use crate::util::{now, resolve_ci};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default, Clone)]
pub struct InstallMeta {
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub picture_url: Option<String>,
    pub nexus: Option<NexusRef>,
}

/// Parses Nexus download names like `Better Weapons-664-1-2-1732400000`.
pub fn parse_nexus_filename(stem: &str) -> Option<(String, u64, Option<String>)> {
    let parts: Vec<&str> = stem.split('-').collect();
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let t = parts.iter().rposition(|p| digits(p) && p.len() >= 9)?;
    if t < 2 {
        return None;
    }
    let (i, id) = (1..t)
        .filter(|&i| digits(parts[i]))
        .filter_map(|i| parts[i].parse::<u64>().ok().map(|v| (i, v)))
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))?;
    let name = parts[..i].join("-").trim().to_string();
    if name.is_empty() {
        return None;
    }
    let version = (i + 1 < t).then(|| parts[i + 1..t].join("."));
    Some((name, id, version))
}

fn ensure_ue4ss_enabled(files: &Path) {
    for e in walkdir::WalkDir::new(files).into_iter().flatten() {
        if e.file_type().is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case("main.lua") {
            let scripts = e.path().parent();
            if scripts.and_then(|s| s.file_name()).map(|n| n.to_string_lossy().eq_ignore_ascii_case("scripts")) != Some(true) {
                continue;
            }
            if let Some(mod_dir) = scripts.and_then(|s| s.parent()) {
                // Mods next to a mods.txt (e.g. UE4SS's bundled ones) are controlled by that list.
                let listed = mod_dir.parent().map(|p| resolve_ci(p, "mods.txt").is_some()).unwrap_or(false);
                if !listed && resolve_ci(mod_dir, "enabled.txt").is_none() {
                    let _ = std::fs::write(mod_dir.join("enabled.txt"), b"");
                }
            }
        }
    }
}

/// Unpacks `src` into `<mods_root>/<id>/files` and builds the entry. When `existing` is
/// given the mod is replaced in place (update) and its option choices are carried over.
pub fn stage(mods_root: &Path, src: &Path, meta: InstallMeta, existing: Option<&ModEntry>) -> Result<ModEntry> {
    if !src.exists() {
        bail!(tr!("{} existiert nicht", "{} does not exist", src.display()));
    }
    if !archive::is_supported(src) {
        bail!(tr!("{}: Format wird nicht unterstützt (erlaubt: zip, 7z, rar, pak/utoc/ucas, dll/asi, Ordner)", "{}: unsupported format (allowed: zip, 7z, rar, pak/utoc/ucas, dll/asi, folders)", src.display()));
    }
    let id = existing.map(|e| e.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());
    let tmp = mods_root.join(format!("{id}.new"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    let files = tmp.join("files");
    if let Err(e) = archive::unpack(src, &files) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    if walkdir::WalkDir::new(&files).into_iter().flatten().all(|e| !e.file_type().is_file()) {
        let _ = std::fs::remove_dir_all(&tmp);
        bail!(tr!("{} enthält keine Dateien", "{} contains no files", src.display()));
    }
    ensure_ue4ss_enabled(&files);

    let Analysis { layout, name, author, version } = analyze(&files);
    let file_name = src.file_name().map(|s| s.to_string_lossy().into_owned());
    let stem = if src.is_dir() {
        file_name.clone().unwrap_or_default()
    } else {
        src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    };
    let parsed = parse_nexus_filename(&stem);

    let nexus = meta.nexus.clone().or_else(|| existing.and_then(|e| e.nexus.clone())).or_else(|| {
        parsed.as_ref().map(|(_, id, _)| NexusRef {
            game_domain: crate::game::NEXUS_DOMAIN.into(),
            mod_id: *id,
            file_id: None,
        })
    });
    let display_name = existing
        .map(|e| e.name.clone())
        .or(meta.name.clone())
        .or(name)
        .or(parsed.as_ref().map(|p| p.0.clone()))
        .unwrap_or_else(|| stem.replace(['_', '.'], " ").trim().to_string());

    // Carry option choices over by group/option name.
    let mut selection = BTreeMap::new();
    let mut mapped_all = true;
    if let Some(old) = existing {
        for g in &layout.groups {
            let old_group = old.layout.groups.iter().find(|og| og.name == g.name && og.step == g.step);
            match old_group.and_then(|og| old.selection.get(&og.id).map(|s| (og, s))) {
                Some((og, chosen)) => {
                    let names: Vec<&String> = og.options.iter().filter(|o| chosen.contains(&o.id)).map(|o| &o.name).collect();
                    let ids: Vec<String> = g.options.iter().filter(|o| names.contains(&&o.name)).map(|o| o.id.clone()).collect();
                    if ids.len() != names.len() {
                        mapped_all = false;
                    }
                    selection.insert(g.id.clone(), ids);
                }
                None => mapped_all = false,
            }
        }
    }
    let needs_setup = !layout.groups.is_empty() && (existing.is_none() || !mapped_all);

    let t = now();
    let entry = ModEntry {
        id: id.clone(),
        name: display_name,
        version: meta.version.or(version).or(parsed.and_then(|p| p.2)).or(existing.and_then(|e| e.version.clone())),
        author: meta.author.or(author).or(existing.and_then(|e| e.author.clone())),
        summary: meta.summary.or(existing.and_then(|e| e.summary.clone())),
        category: existing.and_then(|e| e.category.clone()),
        notes: existing.map(|e| e.notes.clone()).unwrap_or_default(),
        installed_at: existing.map(|e| e.installed_at).unwrap_or(t),
        updated_at: t,
        archive_name: file_name,
        nexus,
        picture_url: meta.picture_url.or(existing.and_then(|e| e.picture_url.clone())),
        layout,
        selection,
        needs_setup,
        update: None,
    };
    crate::util::write_json(&tmp.join("mod.json"), &entry)?;
    let final_dir = mods_root.join(&id);
    if final_dir.exists() {
        let old = mods_root.join(format!("{id}.old"));
        if old.exists() {
            std::fs::remove_dir_all(&old)?;
        }
        std::fs::rename(&final_dir, &old).context(tr!("Alte Mod-Version verschieben (Dateien in Benutzung?)", "Moving the old mod version (files in use?)"))?;
        std::fs::rename(&tmp, &final_dir)?;
        let _ = std::fs::remove_dir_all(&old);
    } else {
        std::fs::rename(&tmp, &final_dir)?;
    }

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nexus_names() {
        assert_eq!(
            parse_nexus_filename("Better Weapons-664-1-2-1732400000"),
            Some(("Better Weapons".into(), 664, Some("1.2".into())))
        );
        assert_eq!(
            parse_nexus_filename("Stalker-2-Fix-123-1-0-1732400000"),
            Some(("Stalker-2-Fix".into(), 123, Some("1.0".into())))
        );
        assert_eq!(parse_nexus_filename("MyMod_P"), None);
    }
}
