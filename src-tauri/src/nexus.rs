//! Nexus Mods API v1 client, nxm:// link handling and protocol registration.

use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::AsyncWriteExt;

const API: &str = "https://api.nexusmods.com/v1";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NexusUser {
    pub name: String,
    pub is_premium: bool,
    pub profile_url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct ModInfo {
    pub mod_id: u64,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub picture_url: Option<String>,
    pub available: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct FileInfo {
    pub file_id: u64,
    pub name: Option<String>,
    pub version: Option<String>,
    pub file_name: Option<String>,
    pub category_name: Option<String>,
    pub size_kb: Option<u64>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct FileUpdate {
    pub old_file_id: u64,
    pub new_file_id: u64,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct FileList {
    pub files: Vec<FileInfo>,
    pub file_updates: Vec<FileUpdate>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NxmLink {
    pub game: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: Option<String>,
    pub expires: Option<String>,
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn parse_nxm(url: &str) -> Result<NxmLink> {
    let rest = url
        .trim()
        .strip_prefix("nxm://")
        .ok_or_else(|| anyhow!("Kein nxm://-Link"))?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    if parts.len() < 5 || !parts[1].eq_ignore_ascii_case("mods") || !parts[3].eq_ignore_ascii_case("files") {
        bail!("Ungültiger nxm-Link: {url}");
    }
    let mut link = NxmLink {
        game: parts[0].to_lowercase(),
        mod_id: parts[2].parse().context("Ungültige Mod-ID")?,
        file_id: parts[4].parse().context("Ungültige Datei-ID")?,
        key: None,
        expires: None,
    };
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "key" => link.key = Some(percent_decode(v)),
                "expires" => link.expires = Some(percent_decode(v)),
                _ => {}
            }
        }
    }
    Ok(link)
}

pub struct Client {
    http: reqwest::Client,
    key: String,
}

impl Client {
    pub fn new(key: &str) -> Result<Self> {
        if key.trim().is_empty() {
            bail!("Kein Nexus-API-Schlüssel hinterlegt (Einstellungen → Nexus Mods).");
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!("S2MM/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Client { http, key: key.trim().to_string() })
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let resp = self
            .http
            .get(format!("{API}{path}"))
            .header("apikey", &self.key)
            .header("Application-Name", "S2MM")
            .header("Application-Version", env!("CARGO_PKG_VERSION"))
            .send()
            .await
            .context("Nexus Mods nicht erreichbar")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["message"].as_str().map(|s| s.to_string()))
                .unwrap_or(body);
            match status.as_u16() {
                401 => bail!("Nexus-API-Schlüssel ungültig: {msg}"),
                403 => bail!("Nexus verweigert den Zugriff: {msg}"),
                404 => bail!("Nicht auf Nexus gefunden: {msg}"),
                429 => bail!("Nexus-Anfragelimit erreicht, bitte später erneut versuchen."),
                _ => bail!("Nexus-Fehler {status}: {msg}"),
            }
        }
        Ok(resp.json().await.context("Antwort von Nexus nicht lesbar")?)
    }

    pub async fn validate(&self) -> Result<NexusUser> {
        let v: serde_json::Value = self.get("/users/validate.json").await?;
        Ok(NexusUser {
            name: v["name"].as_str().unwrap_or("?").to_string(),
            is_premium: v["is_premium"].as_bool().unwrap_or(false),
            profile_url: v["profile_url"].as_str().map(|s| s.to_string()),
        })
    }

    pub async fn mod_info(&self, game: &str, mod_id: u64) -> Result<ModInfo> {
        self.get(&format!("/games/{game}/mods/{mod_id}.json")).await
    }

    pub async fn file_info(&self, game: &str, mod_id: u64, file_id: u64) -> Result<FileInfo> {
        self.get(&format!("/games/{game}/mods/{mod_id}/files/{file_id}.json")).await
    }

    pub async fn files(&self, game: &str, mod_id: u64) -> Result<FileList> {
        self.get(&format!("/games/{game}/mods/{mod_id}/files.json")).await
    }

    pub async fn download_url(&self, game: &str, mod_id: u64, file_id: u64, key: Option<&str>, expires: Option<&str>) -> Result<String> {
        let mut path = format!("/games/{game}/mods/{mod_id}/files/{file_id}/download_link.json");
        if let (Some(k), Some(e)) = (key, expires) {
            path.push_str(&format!("?key={k}&expires={e}"));
        }
        let links: Vec<serde_json::Value> = self.get(&path).await.map_err(|e| {
            if key.is_none() {
                anyhow!("{e:#}\nDirekte Downloads sind nur mit Nexus Premium möglich – nutze den Button „Mod Manager Download“ auf der Nexus-Seite.")
            } else {
                e
            }
        })?;
        links
            .iter()
            .find_map(|l| l["URI"].as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Nexus hat keinen Download-Link geliefert"))
    }

    /// Streams `url` into `dest`, reporting (received, total).
    pub async fn download(&self, url: &str, dest: &Path, mut progress: impl FnMut(u64, Option<u64>)) -> Result<()> {
        let resp = self.http.get(url).send().await?.error_for_status()?;
        let total = resp.content_length();
        let tmp = dest.with_extension("part");
        let mut file = tokio::fs::File::create(&tmp).await?;
        let mut received = 0u64;
        let mut stream = resp.bytes_stream();
        let mut last = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Download abgebrochen")?;
            file.write_all(&chunk).await?;
            received += chunk.len() as u64;
            if received - last > 512 * 1024 {
                last = received;
                progress(received, total);
            }
        }
        file.flush().await?;
        drop(file);
        tokio::fs::rename(&tmp, dest).await?;
        progress(received, total.or(Some(received)));
        Ok(())
    }
}

/// Follows the file_updates chain from `file_id` to the newest file.
pub fn newest_file(list: &FileList, file_id: u64) -> Option<u64> {
    let mut cur = file_id;
    let mut changed = false;
    for _ in 0..64 {
        match list.file_updates.iter().find(|u| u.old_file_id == cur) {
            Some(u) => {
                cur = u.new_file_id;
                changed = true;
            }
            None => break,
        }
    }
    changed.then_some(cur)
}

pub fn file_name_from_url(url: &str) -> Option<String> {
    let path = url.split('?').next()?;
    let name = percent_decode(path.rsplit('/').next()?);
    (!name.is_empty()).then_some(name)
}

#[cfg(windows)]
pub fn set_protocol_handler(enable: bool, previous: Option<&str>) -> Result<Option<String>> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let ours = format!("\"{exe}\" \"%1\"");
    let current: Option<String> = hkcu
        .open_subkey("Software\\Classes\\nxm\\shell\\open\\command")
        .ok()
        .and_then(|k| k.get_value::<String, _>("").ok());
    if enable {
        let (key, _) = hkcu.create_subkey("Software\\Classes\\nxm")?;
        key.set_value("", &"URL:NXM Protocol")?;
        key.set_value("URL Protocol", &"")?;
        let (icon, _) = key.create_subkey("DefaultIcon")?;
        icon.set_value("", &format!("\"{exe}\",0"))?;
        let (cmd, _) = key.create_subkey("shell\\open\\command")?;
        cmd.set_value("", &ours)?;
        Ok(current.filter(|c| c != &ours))
    } else {
        if current.as_deref() == Some(ours.as_str()) {
            match previous {
                Some(prev) => {
                    let (cmd, _) = hkcu.create_subkey("Software\\Classes\\nxm\\shell\\open\\command")?;
                    cmd.set_value("", &prev)?;
                }
                None => {
                    let _ = hkcu.delete_subkey_all("Software\\Classes\\nxm");
                }
            }
        }
        Ok(None)
    }
}

#[cfg(not(windows))]
pub fn set_protocol_handler(_enable: bool, _previous: Option<&str>) -> Result<Option<String>> {
    bail!("nxm-Links werden nur unter Windows unterstützt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nxm() {
        let l = parse_nxm("nxm://stalker2heartofchornobyl/mods/664/files/12345?key=ab%2Dc&expires=1700000000&user_id=1").unwrap();
        assert_eq!(l.mod_id, 664);
        assert_eq!(l.file_id, 12345);
        assert_eq!(l.key.as_deref(), Some("ab-c"));
        assert_eq!(l.expires.as_deref(), Some("1700000000"));
    }

    #[test]
    fn follows_update_chain() {
        let list = FileList {
            files: vec![],
            file_updates: vec![FileUpdate { old_file_id: 1, new_file_id: 2 }, FileUpdate { old_file_id: 2, new_file_id: 5 }],
        };
        assert_eq!(newest_file(&list, 1), Some(5));
        assert_eq!(newest_file(&list, 5), None);
    }
}
