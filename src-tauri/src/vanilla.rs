//! "Vanilla herstellen": brings the game back to an unmodded state. Nothing is deleted –
//! everything that is not part of the game is moved into a dated quarantine folder.

use crate::game::GameInfo;
use crate::model::Deployment;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Engine.ini sections the game writes itself; everything else comes from tweaks.
const GAME_SECTIONS: [&str; 3] = ["[core.system]", "[/script/engine.renderersettings]", "[/script/engine.gameengine]"];

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VanillaOptions {
    pub ue4ss: bool,
    pub engine_ini: bool,
    pub foreign_mods: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VanillaReport {
    pub removed_deployed: usize,
    /// Game-relative or config paths moved into quarantine.
    pub moved: Vec<String>,
    pub engine_ini_reset: Vec<String>,
    pub quarantine: Option<String>,
}

struct Quarantine {
    dir: PathBuf,
    used: bool,
}

impl Quarantine {
    fn take(&mut self, src: &Path, rel: &str) -> Result<()> {
        let dest = crate::util::join_rel(&self.dir, rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if std::fs::rename(src, &dest).is_err() {
            // Different volume: copy, then remove.
            if src.is_dir() {
                crate::util::copy_dir(src, &dest)?;
                std::fs::remove_dir_all(src)?;
            } else {
                std::fs::copy(src, &dest)?;
                std::fs::remove_file(src)?;
            }
        }
        self.used = true;
        Ok(())
    }
}

/// Paths of a manually installed UE4SS (game-relative).
fn ue4ss_paths(game: &GameInfo) -> Vec<String> {
    let bin = game.abs(&game.binaries_dir);
    let mut out = Vec::new();
    let has_new = bin.join("ue4ss").is_dir();
    let has_old = bin.join("UE4SS.dll").is_file();
    if !has_new && !has_old {
        return out;
    }
    let mut push = |name: &str| {
        if bin.join(name).exists() {
            out.push(format!("{}/{name}", game.binaries_dir));
        }
    };
    push("ue4ss");
    push("dwmapi.dll");
    if has_old {
        for f in ["UE4SS.dll", "UE4SS-settings.ini", "xinput1_3.dll", "Mods", "UE4SS.log"] {
            push(f);
        }
    }
    out
}

/// Every entry in ~mods and Stalker2/Mods (after our own deployment was removed).
fn foreign_mod_paths(game: &GameInfo) -> Vec<String> {
    let mut out = Vec::new();
    for dir in [game.mods_dir.as_str(), "Stalker2/Mods"] {
        if let Ok(rd) = std::fs::read_dir(game.abs(dir)) {
            for e in rd.flatten() {
                out.push(format!("{dir}/{}", e.file_name().to_string_lossy()));
            }
        }
    }
    out.sort();
    out
}

/// Keeps only the game's own sections. Returns true when something was removed.
fn reset_engine_ini(dir: &Path, q: &mut Quarantine, label: &str) -> Result<bool> {
    let ini = dir.join("Engine.ini");
    if !ini.exists() {
        return Ok(false);
    }
    let text = String::from_utf8_lossy(&std::fs::read(&ini)?).into_owned();
    let mut keep = true;
    let mut out: Vec<&str> = Vec::new();
    let mut removed = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            keep = GAME_SECTIONS.contains(&t.to_lowercase().as_str());
        }
        if keep {
            out.push(line);
        } else if !t.is_empty() {
            removed = true;
        }
    }
    if !removed {
        return Ok(false);
    }
    while out.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        out.pop();
    }
    let qdest = q.dir.join("Config").join(label);
    std::fs::create_dir_all(&qdest)?;
    std::fs::copy(&ini, qdest.join("Engine.ini"))?;
    let s2mm_backup = dir.join("Engine.ini.s2mm-backup");
    if s2mm_backup.exists() {
        std::fs::rename(&s2mm_backup, qdest.join("Engine.ini.s2mm-backup")).ok();
    }
    q.used = true;
    crate::tweaks::set_readonly(&ini, false)?;
    std::fs::write(&ini, out.join("\r\n") + "\r\n").context("Engine.ini zurücksetzen")?;
    Ok(true)
}

/// Every Engine.ini location the game may use.
pub fn config_dirs(extra: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        for p in ["Windows", "WinGDK"] {
            dirs.push(PathBuf::from(&local).join("Stalker2/Saved/Config").join(p));
        }
    }
    if let Some(e) = extra {
        if !dirs.iter().any(|d| d == e) {
            dirs.push(e.to_path_buf());
        }
    }
    dirs.into_iter().filter(|d| d.is_dir()).collect()
}

pub fn restore(
    game: &GameInfo,
    deployment: &Deployment,
    opts: VanillaOptions,
    quarantine_root: &Path,
    config: &[PathBuf],
) -> Result<VanillaReport> {
    if !cfg!(test) && crate::game::is_running() {
        anyhow::bail!("Stalker 2 läuft gerade. Bitte das Spiel schließen und erneut versuchen.");
    }
    let mut report = VanillaReport::default();
    report.removed_deployed = crate::deploy::purge(game, deployment)?;

    let stamp = chrono_like_stamp();
    let mut q = Quarantine { dir: quarantine_root.join(stamp), used: false };
    if opts.ue4ss {
        for rel in ue4ss_paths(game) {
            q.take(&game.abs(&rel), &rel).with_context(|| format!("{rel} verschieben"))?;
            report.moved.push(rel);
        }
    }
    if opts.foreign_mods {
        for rel in foreign_mod_paths(game) {
            q.take(&game.abs(&rel), &rel).with_context(|| format!("{rel} verschieben"))?;
            report.moved.push(rel);
        }
    }
    if opts.engine_ini {
        for dir in config {
            let label = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Config".into());
            if reset_engine_ini(dir, &mut q, &label)? {
                report.engine_ini_reset.push(dir.join("Engine.ini").to_string_lossy().into_owned());
            }
        }
    }
    if q.used {
        report.quarantine = Some(q.dir.to_string_lossy().into_owned());
    }
    Ok(report)
}

/// YYYY-MM-DD_HH-MM-SS (UTC) without pulling in a date crate.
fn chrono_like_stamp() -> String {
    let secs = crate::util::now();
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (h, m, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if mo <= 2 { 1 } else { 0 };
    format!("{y:04}-{mo:02}-{d:02}_{h:02}-{m:02}-{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Platform, Settings};

    #[test]
    fn restores_vanilla_state() {
        let tmp = std::env::temp_dir().join(format!("s2mm-vanilla-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let root = tmp.join("game");
        let bin = root.join("Stalker2/Binaries/Win64");
        std::fs::create_dir_all(bin.join("ue4ss/Mods")).unwrap();
        std::fs::write(bin.join("dwmapi.dll"), b"x").unwrap();
        std::fs::write(bin.join("Stalker2-Win64-Shipping.exe"), b"game").unwrap();
        std::fs::create_dir_all(root.join("Stalker2/Content/Paks/~mods")).unwrap();
        std::fs::write(root.join("Stalker2/Content/Paks/~mods/Manual_P.pak"), b"x").unwrap();
        std::fs::create_dir_all(root.join("Stalker2/Mods/KitMod/Content")).unwrap();
        let cfg = tmp.join("Config/Windows");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::write(
            cfg.join("Engine.ini"),
            "[Core.System]\r\nPaths=a\r\n\r\n[/Script/Engine.GameEngine]\r\nDisplayGamma=2\r\n\r\n[SystemSettings]\r\nr.FilmGrain=0\r\n\r\n[ConsoleVariables]\r\nx=1\r\n",
        )
        .unwrap();

        let game = crate::game::inspect(&root, &Settings { platform: Platform::Steam, ..Settings::default() });
        let opts = VanillaOptions { ue4ss: true, engine_ini: true, foreign_mods: true };
        let r = restore(&game, &Deployment::default(), opts, &tmp.join("quarantine"), &[cfg.clone()]).unwrap();

        assert!(!bin.join("ue4ss").exists() && !bin.join("dwmapi.dll").exists());
        assert!(bin.join("Stalker2-Win64-Shipping.exe").exists(), "game files must stay");
        assert_eq!(std::fs::read_dir(root.join("Stalker2/Content/Paks/~mods")).unwrap().count(), 0);
        assert!(!root.join("Stalker2/Mods/KitMod").exists());
        let ini = std::fs::read_to_string(cfg.join("Engine.ini")).unwrap();
        assert!(ini.contains("[Core.System]") && ini.contains("DisplayGamma=2"));
        assert!(!ini.contains("SystemSettings") && !ini.contains("ConsoleVariables"));
        let q = PathBuf::from(r.quarantine.unwrap());
        assert!(q.join("Stalker2/Binaries/Win64/dwmapi.dll").exists());
        assert!(q.join("Stalker2/Content/Paks/~mods/Manual_P.pak").exists());
        assert!(std::fs::read_to_string(q.join("Config/Windows/Engine.ini")).unwrap().contains("r.FilmGrain=0"));
        assert_eq!(r.moved.len(), 4, "{:?}", r.moved);
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn stamp_format() {
        let s = chrono_like_stamp();
        assert_eq!(s.len(), 19);
        assert!(s.starts_with("20"));
    }
}
