//! Turns the active profile into a concrete list of files to place in the game folder,
//! and finds conflicts between mods (same target file or same asset inside paks).

use crate::game::GameInfo;
use crate::layout::{resolve, Classifier, TargetKind};
use crate::model::Db;
use crate::pak;
use crate::util::{file_meta, slugify};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct PlannedFile {
    pub mod_id: String,
    pub src: PathBuf,
    pub logical: String,
    pub target: String,
    pub kind: TargetKind,
    pub size: u64,
    pub mtime: u64,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModPlanInfo {
    pub kinds: BTreeSet<TargetKind>,
    pub files: usize,
    pub unmapped: Vec<String>,
}

#[derive(Default)]
pub struct Plan {
    pub files: Vec<PlannedFile>,
    /// (target, overridden mod, winning mod)
    pub overrides: Vec<(String, String, String)>,
    pub per_mod: HashMap<String, ModPlanInfo>,
    pub fingerprint: String,
}

pub fn mod_files_dir(mods_root: &Path, id: &str) -> PathBuf {
    mods_root.join(id).join("files")
}

fn has_p_suffix(stem: &str) -> bool {
    let s = stem.trim_end_matches(|c: char| c.is_ascii_digit()).to_lowercase();
    s.ends_with("_p")
}

pub fn pak_target(game: &GameInfo, prio: usize, mod_name: &str, file: &str, add_p: bool) -> String {
    let path = Path::new(file);
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = crate::util::ext_lower(file);
    let suffix = if add_p && !has_p_suffix(&stem) { "_P" } else { "" };
    format!("{}/{:03}_{}_{}{}.{}", game.mods_dir, prio, slugify(mod_name, 20), stem, suffix, ext)
}

/// Plans one mod as if it were enabled (used for details view too).
pub fn plan_mod(db: &Db, game: &GameInfo, mods_root: &Path, mod_id: &str, prio: usize) -> (Vec<PlannedFile>, ModPlanInfo) {
    let mut info = ModPlanInfo::default();
    let mut out = Vec::new();
    let Some(entry) = db.find_mod(mod_id) else { return (out, info) };
    let files = resolve(&mod_files_dir(mods_root, mod_id), entry);
    let classifier = Classifier::new(game, &files, &entry.name);
    for f in files {
        match classifier.classify(&f.logical) {
            Some((target, kind)) => {
                // Zone Kit paks load from ~mods like any other pak (Stalker2/Mods is only
                // used by Workshop/mod.io), so they get the same load-order prefix.
                let zone_kit_pak = kind == TargetKind::ZoneKit
                    && crate::archive::PAK_EXTS.contains(&crate::util::ext_lower(&target).as_str());
                let target = if kind == TargetKind::Pak || zone_kit_pak {
                    let file = target.rsplit('/').next().unwrap_or(&target).to_string();
                    pak_target(game, prio, &entry.name, &file, db.settings.add_p_suffix)
                } else {
                    target
                };
                let (size, mtime) = file_meta(&f.src);
                info.kinds.insert(kind);
                info.files += 1;
                out.push(PlannedFile { mod_id: mod_id.into(), src: f.src, logical: f.logical, target, kind, size, mtime });
            }
            None => info.unmapped.push(f.logical),
        }
    }
    (out, info)
}

pub fn build(db: &Db, game: &GameInfo, mods_root: &Path) -> Plan {
    let mut plan = Plan::default();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, pm) in db.profile().mods.iter().enumerate() {
        let (files, info) = plan_mod(db, game, mods_root, &pm.id, i + 1);
        plan.per_mod.insert(pm.id.clone(), info);
        if !pm.enabled {
            continue;
        }
        for f in files {
            let key = f.target.to_lowercase();
            match index.get(&key) {
                Some(&at) => {
                    let loser = plan.files[at].mod_id.clone();
                    if loser != f.mod_id {
                        plan.overrides.push((f.target.clone(), loser, f.mod_id.clone()));
                    }
                    plan.files[at] = f;
                }
                None => {
                    index.insert(key, plan.files.len());
                    plan.files.push(f);
                }
            }
        }
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut keys: Vec<_> = plan.files.iter().map(|f| (&f.target, &f.src, f.size, f.mtime)).collect();
    keys.sort();
    keys.hash(&mut h);
    plan.fingerprint = format!("{:016x}", h.finish());
    plan
}

/// Cached pak indexes, keyed by path and invalidated by size/mtime.
#[derive(Default)]
pub struct PakCache {
    entries: HashMap<PathBuf, (u64, u64, Result<Arc<Vec<String>>, String>)>,
}

impl PakCache {
    pub fn assets(&mut self, path: &Path) -> Result<Arc<Vec<String>>, String> {
        let (size, mtime) = file_meta(path);
        if let Some((s, m, r)) = self.entries.get(path) {
            if *s == size && *m == mtime {
                return r.clone();
            }
        }
        let r = pak::list_assets(path).map(Arc::new).map_err(|e| format!("{e:#}"));
        self.entries.insert(path.to_path_buf(), (size, mtime, r.clone()));
        r
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConflictPair {
    pub winner: String,
    pub loser: String,
    pub count: usize,
    /// Up to 500 example paths.
    pub items: Vec<String>,
    /// "asset" (inside paks) or "file" (same file in the game folder).
    pub kind: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConflictError {
    pub mod_id: String,
    pub file: String,
    pub error: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModConflicts {
    /// Assets/files this mod overrides in other mods.
    pub wins: usize,
    /// Assets/files of this mod that are overridden by other mods.
    pub loses: usize,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConflictReport {
    pub pairs: Vec<ConflictPair>,
    pub errors: Vec<ConflictError>,
    pub per_mod: BTreeMap<String, ModConflicts>,
}

fn ignored_asset(a: &str) -> bool {
    a.ends_with("assetregistry.bin") || a.ends_with(".ushaderbytecode") || a.ends_with("/pipelinecaches")
}

pub fn conflicts(plan: &Plan, cache: &mut PakCache) -> ConflictReport {
    let mut report = ConflictReport::default();
    // asset (lowercase) -> (display path, mods in priority order)
    let mut assets: HashMap<String, (String, Vec<String>)> = HashMap::new();
    for f in plan.files.iter().filter(|f| matches!(f.kind, TargetKind::Pak | TargetKind::ZoneKit)) {
        match cache.assets(&f.src) {
            Ok(list) => {
                for a in list.iter() {
                    let key = a.to_lowercase();
                    if ignored_asset(&key) {
                        continue;
                    }
                    let e = assets.entry(key).or_insert_with(|| (a.clone(), Vec::new()));
                    if e.1.last() != Some(&f.mod_id) {
                        e.1.retain(|m| m != &f.mod_id);
                        e.1.push(f.mod_id.clone());
                    }
                }
            }
            Err(error) => report.errors.push(ConflictError {
                mod_id: f.mod_id.clone(),
                file: f.logical.clone(),
                error,
            }),
        }
    }

    let mut pairs: BTreeMap<(String, String, &'static str), Vec<String>> = BTreeMap::new();
    for (display, mods) in assets.into_values() {
        if mods.len() < 2 {
            continue;
        }
        let winner = mods.last().unwrap();
        for loser in &mods[..mods.len() - 1] {
            pairs.entry((winner.clone(), loser.clone(), "asset")).or_default().push(display.clone());
        }
    }
    for (target, loser, winner) in &plan.overrides {
        pairs.entry((winner.clone(), loser.clone(), "file")).or_default().push(target.clone());
    }
    for ((winner, loser, kind), mut items) in pairs {
        items.sort();
        let count = items.len();
        report.per_mod.entry(winner.clone()).or_default().wins += count;
        report.per_mod.entry(loser.clone()).or_default().loses += count;
        items.truncate(500);
        report.pairs.push(ConflictPair { winner, loser, count, items, kind: kind.into() });
    }
    report
}
