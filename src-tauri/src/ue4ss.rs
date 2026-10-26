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
}
