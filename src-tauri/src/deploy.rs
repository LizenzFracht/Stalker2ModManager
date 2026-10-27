//! Applying a plan to the game folder. Only files we placed are ever removed; game files
//! that a mod replaces are moved to a backup folder first and restored on removal.

use crate::tr;

use crate::game::{self, GameInfo};
use crate::model::{DeployedFile, Deployment};
use crate::plan::Plan;
use crate::util::{file_meta, join_rel, now};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ApplyResult {
    pub deployment: Deployment,
    pub placed: usize,
    pub removed: usize,
    pub kept: usize,
}

fn ensure_not_running() -> Result<()> {
    if !cfg!(test) && game::is_running() {
        bail!(tr!("Stalker 2 läuft gerade. Bitte das Spiel schließen und erneut versuchen.", "Stalker 2 is running. Please close the game and try again."));
    }
    Ok(())
}

/// Removes empty parent folders up to (but excluding) the protected roots.
fn prune_dirs(game: &GameInfo, target: &Path) {
    let root = game.root();
    let stop: Vec<PathBuf> = [game.mods_dir.as_str(), game.binaries_dir.as_str(), game.ue4ss_mods_dir.as_str(), "Stalker2/Content", "Stalker2/Mods", "Stalker2"]
        .iter()
        .map(|r| join_rel(&root, r))
        .collect();
    let mut dir = target.parent();
    while let Some(d) = dir {
        if d == root || stop.iter().any(|s| s == d) || !d.starts_with(&root) {
            break;
        }
        if std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

fn remove_deployed(game: &GameInfo, f: &DeployedFile) -> Result<()> {
    let target = game.abs(&f.target);
    if target.exists() {
        std::fs::remove_file(&target).with_context(|| tr!("{} entfernen", "Removing {}", f.target))?;
    }
    if let Some(backup) = &f.backup {
        let b = PathBuf::from(backup);
        if b.exists() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::util::move_path(&b, &target)
                .with_context(|| tr!("Original {} wiederherstellen", "Restoring original {}", f.target))?;
            return Ok(());
        }
    }
    prune_dirs(game, &target);
    Ok(())
}

fn place(src: &Path, target: &Path, hardlink: bool) -> Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if hardlink && std::fs::hard_link(src, target).is_ok() {
        return Ok(());
    }
    std::fs::copy(src, target).with_context(|| tr!("{} kopieren", "Copying {}", target.display()))?;
    Ok(())
}

pub fn apply(
    game: &GameInfo,
    plan: &Plan,
    old: &Deployment,
    backups: &Path,
    hardlinks: bool,
) -> Result<ApplyResult> {
    ensure_not_running()?;
    if !game.valid {
        bail!(tr!("Spielordner ist ungültig", "Invalid game folder"));
    }
    let same_game = old.game_path.eq_ignore_ascii_case(&game.path);
    let mut old_by_target: HashMap<String, &DeployedFile> = HashMap::new();
    if same_game {
        for f in &old.files {
            old_by_target.insert(f.target.to_lowercase(), f);
        }
    }

    let mut result_files: Vec<DeployedFile> = Vec::new();
    let mut removed = 0;
    let mut kept = 0;
    let mut placed = 0;

    // 1. Remove what is no longer wanted (or changed).
    let wanted: HashMap<String, &crate::plan::PlannedFile> =
        plan.files.iter().map(|f| (f.target.to_lowercase(), f)).collect();
    let mut keep: HashMap<String, DeployedFile> = HashMap::new();
    for f in old.files.iter().filter(|_| same_game) {
        let key = f.target.to_lowercase();
        let unchanged = wanted.get(&key).map(|p| {
            p.src.to_string_lossy() == f.source.as_str() && p.size == f.size && p.mtime == f.mtime && file_meta(&game.abs(&f.target)).0 == f.size
        });
        if unchanged == Some(true) {
            keep.insert(key, f.clone());
        } else if wanted.contains_key(&key) && f.backup.is_some() {
            // Replaced by another mod file: drop ours but keep the original's backup.
            let target = game.abs(&f.target);
            if target.exists() {
                std::fs::remove_file(&target).with_context(|| tr!("{} entfernen", "Removing {}", f.target))?;
            }
            keep.insert(key, DeployedFile { size: u64::MAX, ..f.clone() });
            removed += 1;
        } else {
            remove_deployed(game, f)?;
            removed += 1;
        }
    }
    if !same_game {
        // Clean up a deployment into a different (old) game folder if it still exists.
        if !old.game_path.is_empty() && Path::new(&old.game_path).exists() {
            let old_game = GameInfo { path: old.game_path.clone(), ..game.clone() };
            for f in &old.files {
                let _ = remove_deployed(&old_game, f);
            }
        }
    }

    // 2. Place new files.
    std::fs::create_dir_all(game.abs(&game.mods_dir))?;
    for p in &plan.files {
        let key = p.target.to_lowercase();
        if let Some(k) = keep.get(&key) {
            if k.size != u64::MAX {
                result_files.push(k.clone());
                kept += 1;
                continue;
            }
        }
        let target = game.abs(&p.target);
        let mut backup = keep.get(&key).and_then(|k| k.backup.clone());
        if target.exists() && backup.is_none() && !old_by_target.contains_key(&key) {
            let b = join_rel(backups, &p.target);
            if let Some(parent) = b.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if b.exists() {
                std::fs::remove_file(&b)?;
            }
            // The backup folder usually lives on another drive than the game.
            crate::util::move_path(&target, &b).with_context(|| tr!("Original {} sichern", "Backing up original {}", p.target))?;
            backup = Some(b.to_string_lossy().into_owned());
        } else if target.exists() {
            std::fs::remove_file(&target).with_context(|| tr!("{} ersetzen", "Replacing {}", p.target))?;
        }
        place(&p.src, &target, hardlinks)?;
        placed += 1;
        result_files.push(DeployedFile {
            target: p.target.clone(),
            source: p.src.to_string_lossy().into_owned(),
            mod_id: p.mod_id.clone(),
            size: p.size,
            mtime: p.mtime,
            backup,
        });
    }

    Ok(ApplyResult {
        deployment: Deployment {
            game_path: game.path.clone(),
            fingerprint: plan.fingerprint.clone(),
            applied_at: now(),
            game_build: game.build.clone(),
            files: result_files,
        },
        placed,
        removed,
        kept,
    })
}

/// Removes everything we deployed and restores original files.
pub fn purge(game: &GameInfo, old: &Deployment) -> Result<usize> {
    ensure_not_running()?;
    let g = GameInfo { path: if old.game_path.is_empty() { game.path.clone() } else { old.game_path.clone() }, ..game.clone() };
    let mut n = 0;
    for f in &old.files {
        remove_deployed(&g, f)?;
        n += 1;
    }
    Ok(n)
}

/// Files in ~mods that we did not put there.
pub fn unmanaged(game: &GameInfo, dep: &Deployment) -> Vec<String> {
    let dir = game.abs(&game.mods_dir);
    let ours: std::collections::HashSet<String> = dep.files.iter().map(|f| f.target.to_lowercase()).collect();
    let mut out: Vec<String> = walkdir::WalkDir::new(&dir)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| format!("{}/{}", game.mods_dir, crate::util::rel_path(&dir, e.path())))
        .filter(|t| !ours.contains(&t.to_lowercase()))
        .collect();
    out.sort();
    out
}
