//! Downloads UE4SS (experimental build) plus the Stalker 2 game config from GitHub and
//! prepares a folder that is installed like any other mod.

use crate::tr;

use crate::model::Platform;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

const RELEASE_API: &str = "https://api.github.com/repos/UE4SS-RE/RE-UE4SS/releases/tags/experimental-latest";

pub struct Prepared {
    pub folder: PathBuf,
    pub version: String,
}

async fn download(client: &reqwest::Client, url: &str, dest: &Path) -> Result<()> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk.context(tr!("Download abgebrochen", "Download interrupted"))?).await?;
    }
    file.flush().await?;
    Ok(())
}

/// Downloads the archives into `work` and returns a folder ready for staging.
pub async fn prepare(work: &Path, platform: Platform) -> Result<Prepared> {
    let client = reqwest::Client::builder().user_agent(concat!("S2MM/", env!("CARGO_PKG_VERSION"))).build()?;
    let release: serde_json::Value = client
        .get(RELEASE_API)
        .send()
        .await
        .context(tr!("GitHub nicht erreichbar", "GitHub not reachable"))?
        .error_for_status()?
        .json()
        .await?;
    let assets = release["assets"].as_array().ok_or_else(|| anyhow!(tr!("Keine UE4SS-Dateien im Release gefunden", "No UE4SS files found in the release")))?;
    let find = |pred: &dyn Fn(&str) -> bool| {
        assets.iter().find_map(|a| {
            let name = a["name"].as_str()?;
            pred(name).then(|| (name.to_string(), a["browser_download_url"].as_str().unwrap_or("").to_string()))
        })
    };
    let (main_name, main_url) =
        find(&|n| n.starts_with("UE4SS_") && n.ends_with(".zip")).ok_or_else(|| anyhow!(tr!("UE4SS-Archiv nicht gefunden", "UE4SS archive not found")))?;
    let configs = find(&|n| n == "zCustomGameConfigs.zip");

    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work)?;
    let main_zip = work.join(&main_name);
    download(&client, &main_url, &main_zip).await?;
    let cfg_zip = match &configs {
        Some((name, url)) => {
            let p = work.join(name);
            download(&client, url, &p).await?;
            Some(p)
        }
        None => None,
    };

    let work = work.to_path_buf();
    let version = main_name.trim_start_matches("UE4SS_").trim_end_matches(".zip").to_string();
    tauri::async_runtime::spawn_blocking(move || -> Result<Prepared> {
        let folder = work.join("UE4SS-Paket");
        crate::archive::unpack(&main_zip, &folder)?;
        if let Some(cfg_zip) = cfg_zip {
            let cfg = work.join("configs");
            crate::archive::unpack(&cfg_zip, &cfg)?;
            let flavor = if platform == Platform::GamePass { "gamepass" } else { "steam" };
            if let Some(src) = crate::util::resolve_ci(&cfg, &format!("Stalker2/{flavor}")) {
                crate::util::copy_dir(&src, &folder.join("ue4ss"))?;
            }
        }
        Ok(Prepared { folder, version })
    })
    .await
    .map_err(|e| anyhow!("{e}"))?
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogStatus {
    /// ok | failed | unknown (no log yet)
    pub state: String,
    /// The fatal error line(s) when UE4SS could not start.
    pub detail: Option<String>,
    pub log_path: Option<String>,
    pub log_time: u64,
}

impl Default for LogStatus {
    fn default() -> Self {
        Self { state: "unknown".into(), detail: None, log_path: None, log_time: 0 }
    }
}

/// Reads UE4SS.log of the last game start and tells whether UE4SS initialised.
pub fn log_status(game: &crate::game::GameInfo) -> LogStatus {
    let bin = game.abs(&game.binaries_dir);
    let Some(log) = [bin.join("ue4ss/UE4SS.log"), bin.join("UE4SS.log")].into_iter().find(|p| p.is_file()) else {
        return LogStatus { state: "unknown".into(), ..Default::default() };
    };
    let (size, mtime) = crate::util::file_meta(&log);
    let mut text = String::new();
    if let Ok(mut f) = std::fs::File::open(&log) {
        use std::io::{Read, Seek, SeekFrom};
        let start = size.saturating_sub(256 * 1024);
        let _ = f.seek(SeekFrom::Start(start));
        let mut buf = Vec::new();
        let _ = f.read_to_end(&mut buf);
        text = String::from_utf8_lossy(&buf).into_owned();
    }
    let lines: Vec<&str> = text.lines().collect();
    let fatal = lines.iter().rposition(|l| l.contains("Fatal Error"));
    let state = if fatal.is_some() { "failed" } else if text.contains("[S2MM LiveMap]") { "ok" } else { "unknown" };
    let detail = fatal.map(|i| {
        lines[i..lines.len().min(i + 4)]
            .iter()
            .map(|l| l.trim().trim_start_matches(|c: char| c == '[' || c.is_ascii_digit() || " -:.]".contains(c)).to_string())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    });
    LogStatus { state: state.into(), detail, log_path: Some(log.to_string_lossy().into_owned()), log_time: mtime }
}

/// Result of checking UE4SS's game-specific AOB signatures against the game executable.
#[derive(serde::Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SigCheck {
    pub checked: usize,
    /// Signature names (file stems) whose byte pattern no longer occurs in the exe.
    pub missing: Vec<String>,
    /// "game" (deployed UE4SS) or "library" (UE4SS staged in S2MM but not applied).
    pub source: String,
}

/// Parses `return "41 57 ?? 8B ..."` from a UE4SS_Signatures/*.lua file.
fn parse_pattern(lua: &str) -> Option<Vec<Option<u8>>> {
    // The first `return "<pattern>"` – comments may mention "returns" earlier.
    let rest = lua
        .match_indices("return")
        .find_map(|(i, _)| lua[i + "return".len()..].trim_start().strip_prefix('"'))?;
    let body = &rest[..rest.find('"')?];
    let mut out = Vec::new();
    for tok in body.split_whitespace() {
        if tok.starts_with('?') {
            out.push(None);
        } else {
            out.push(Some(u8::from_str_radix(tok, 16).ok()?));
        }
    }
    (!out.is_empty()).then_some(out)
}

/// True when the wildcard pattern occurs in `data`. Searches for the longest literal run
/// first and verifies the rest around each hit.
fn pattern_found(data: &[u8], pat: &[Option<u8>]) -> bool {
    let (mut best_start, mut best_len, mut i) = (0, 0, 0);
    while i < pat.len() {
        if pat[i].is_some() {
            let j = (i..pat.len()).find(|&k| pat[k].is_none()).unwrap_or(pat.len());
            if j - i > best_len {
                best_start = i;
                best_len = j - i;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    if best_len == 0 {
        return true;
    }
    let anchor: Vec<u8> = pat[best_start..best_start + best_len].iter().map(|b| b.unwrap()).collect();
    memchr::memmem::find_iter(data, &anchor).any(|hit| {
        let Some(begin) = hit.checked_sub(best_start) else { return false };
        begin + pat.len() <= data.len() && pat.iter().enumerate().all(|(k, b)| b.map_or(true, |v| data[begin + k] == v))
    })
}

static SIG_CACHE: std::sync::Mutex<Option<(String, SigCheck)>> = std::sync::Mutex::new(None);

/// Checks every signature in `sig_dir` against `exe`. Cached until the exe or the
/// signature files change, so it is cheap to call on every state refresh.
pub fn check_signatures(exe: &Path, sig_dir: &Path, source: &str) -> Option<SigCheck> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(sig_dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| crate::util::ext_lower(&p.to_string_lossy()) == "lua")
        .collect();
    files.sort();
    if files.is_empty() {
        return None;
    }
    let (size, mtime) = crate::util::file_meta(exe);
    if size == 0 {
        return None;
    }
    let mut key = format!("{}|{size}|{mtime}", exe.display());
    for f in &files {
        let (s, m) = crate::util::file_meta(f);
        key.push_str(&format!("|{}:{s}:{m}", f.display()));
    }
    if let Some((k, v)) = SIG_CACHE.lock().ok()?.as_ref() {
        if *k == key {
            return Some(v.clone());
        }
    }
    let data = std::fs::read(exe).ok()?;
    let mut result = SigCheck { source: source.into(), ..Default::default() };
    for f in &files {
        let Some(pat) = std::fs::read_to_string(f).ok().as_deref().and_then(parse_pattern) else { continue };
        result.checked += 1;
        if !pattern_found(&data, &pat) {
            result.missing.push(f.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
        }
    }
    *SIG_CACHE.lock().ok()? = Some((key, result.clone()));
    Some(result)
}

/// The deployed UE4SS signatures, or those of the UE4SS copy in the S2MM library.
pub fn signature_status(game: &crate::game::GameInfo, library_copy: Option<&Path>) -> Option<SigCheck> {
    let bin = game.abs(&game.binaries_dir);
    let exe = crate::game::SHIPPING_EXES.iter().map(|e| bin.join(e)).find(|p| p.is_file())?;
    let deployed = bin.join("ue4ss").join("UE4SS_Signatures");
    if deployed.is_dir() {
        return check_signatures(&exe, &deployed, "game");
    }
    let lib = library_copy?;
    let dir = walkdir::WalkDir::new(lib)
        .into_iter()
        .flatten()
        .find(|e| e.file_type().is_dir() && e.file_name().to_string_lossy().eq_ignore_ascii_case("UE4SS_Signatures"))?;
    check_signatures(&exe, dir.path(), "library")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_log_does_not_claim_ue4ss_started() {
        let root = std::env::temp_dir().join(format!("s2mm-log-{}", uuid::Uuid::new_v4()));
        let bin = root.join("Stalker2/Binaries/Win64/ue4ss");
        std::fs::create_dir_all(&bin).unwrap();
        let game = crate::game::inspect(&root, &crate::model::Settings::default());
        for content in ["", "[2026-10-26] Lua Scan attempt 1 (Phase 2)"] {
            std::fs::write(bin.join("UE4SS.log"), content).unwrap();
            assert_eq!(log_status(&game).state, "unknown");
        }
        std::fs::write(bin.join("UE4SS.log"), "[2026-10-26] Fatal Error: AOB scans could not be completed\nWas unable to find AOB for 'GUObjectArray'").unwrap();
        let failed = log_status(&game);
        assert_eq!(failed.state, "failed");
        assert!(failed.detail.unwrap().contains("GUObjectArray"));
        std::fs::write(bin.join("UE4SS.log"), "[S2MM LiveMap] aktiv – Position wird geschrieben").unwrap();
        assert_eq!(log_status(&game).state, "ok");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn signature_patterns_match_with_wildcards() {
        let pat = parse_pattern("function Register()
    return \"41 57 ?? 8B ? C5\"
end").unwrap();
        assert_eq!(pat, vec![Some(0x41), Some(0x57), None, Some(0x8B), None, Some(0xC5)]);
        let commented = parse_pattern("-- this returns the singleton
function Register()
    return \"F6 05 ??\"
end").unwrap();
        assert_eq!(commented, vec![Some(0xF6), Some(0x05), None]);
        assert!(pattern_found(&[0, 0x41, 0x57, 9, 0x8B, 7, 0xC5, 0], &pat));
        assert!(!pattern_found(&[0, 0x41, 0x57, 9, 0x8B, 7, 0xC4, 0], &pat));
        assert!(!pattern_found(&[0x57, 9, 0x8B], &pat), "pattern must not start before the data");

        let dir = std::env::temp_dir().join(format!("s2mm-sig-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Game.exe"), [1u8, 2, 0x41, 0x57, 3, 0x8B, 4, 0xC5, 5]).unwrap();
        std::fs::write(dir.join("GNatives.lua"), "return \"41 57 ?? 8B ?? C5\"").unwrap();
        std::fs::write(dir.join("GUObjectArray.lua"), "return \"39 3D ?? ?? 0F 8E\"").unwrap();
        let r = check_signatures(&dir.join("Game.exe"), &dir, "game").unwrap();
        assert_eq!((r.checked, r.missing.clone()), (2, vec!["GUObjectArray".to_string()]));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Set S2MM_GAME_ROOT to a Stalker 2 install with UE4SS to check the real signatures.
    #[test]
    fn real_game_signatures() {
        let Some(root) = std::env::var_os("S2MM_GAME_ROOT") else { return };
        let game = crate::game::inspect(Path::new(&root), &crate::model::Settings::default());
        let r = signature_status(&game, None).expect("UE4SS signatures not found in game folder");
        println!("UE4SS signatures: {} checked, missing {:?}", r.checked, r.missing);
    }
}
