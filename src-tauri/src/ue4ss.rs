//! Downloads UE4SS (experimental build) plus the Stalker 2 game config from GitHub and
//! prepares a folder that is installed like any other mod.

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
        file.write_all(&chunk.context("Download abgebrochen")?).await?;
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
        .context("GitHub nicht erreichbar")?
        .error_for_status()?
        .json()
        .await?;
    let assets = release["assets"].as_array().ok_or_else(|| anyhow!("Keine UE4SS-Dateien im Release gefunden"))?;
    let find = |pred: &dyn Fn(&str) -> bool| {
        assets.iter().find_map(|a| {
            let name = a["name"].as_str()?;
            pred(name).then(|| (name.to_string(), a["browser_download_url"].as_str().unwrap_or("").to_string()))
        })
    };
    let (main_name, main_url) =
        find(&|n| n.starts_with("UE4SS_") && n.ends_with(".zip")).ok_or_else(|| anyhow!("UE4SS-Archiv nicht gefunden"))?;
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
