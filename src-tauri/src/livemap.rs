//! Local web server for the live map: proxies (and caches) joric's Stalker 2 map,
//! injects a small overlay script and serves the player position written by the
//! S2MM Live-Map UE4SS mod.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const UPSTREAM: &str = "https://joric.github.io/stalker";
pub const LUA_MOD: &str = include_str!("../livemap/main.lua");
const OVERLAY: &str = include_str!("../livemap/overlay.js");

pub struct Server {
    pub port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
}

impl Server {
    pub fn stop(self) {
        let _ = self.stop.send(());
    }
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LivePosition {
    pub ok: bool,
    /// Seconds since the game last wrote the position.
    pub age: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
}

/// `%LOCALAPPDATA%\S2MM` — shared with the Lua mod.
pub fn live_dir() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into())).join("S2MM")
}

pub fn read_position() -> LivePosition {
    let file = live_dir().join("live.json");
    let Ok(text) = std::fs::read_to_string(&file) else { return LivePosition::default() };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return LivePosition::default() };
    let (_, mtime) = crate::util::file_meta(&file);
    LivePosition {
        ok: true,
        age: crate::util::now().saturating_sub(mtime),
        x: v["x"].as_f64().unwrap_or(0.0),
        y: v["y"].as_f64().unwrap_or(0.0),
        z: v["z"].as_f64().unwrap_or(0.0),
        yaw: v["yaw"].as_f64().unwrap_or(0.0),
    }
}

fn content_type(path: &str) -> &'static str {
    match crate::util::ext_lower(path).as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

/// Makes the module-scoped `map` reachable for the overlay and loads the overlay.
fn inject(html: &str) -> String {
    let mut out = html.replacen(
        "window.saveLocalData = saveLocalData;",
        "window.saveLocalData = saveLocalData;\nwindow.__s2mmMap = () => map;\nwindow.__s2mmGroup = () => groupLayer;",
        1,
    );
    if let (Some(a), Some(b)) = (out.find("<title>"), out.find("</title>")) {
        if a < b {
            out.replace_range(a + 7..b, "S2MM Live-Karte – Stalker 2");
        }
    }
    let tag = "<script src=\"/s2mm/overlay.js\"></script>";
    match out.rfind("</body>") {
        Some(i) => out.insert_str(i, tag),
        None => out.push_str(tag),
    }
    out
}

struct Ctx {
    cache: PathBuf,
    http: reqwest::Client,
}

async fn fetch_cached(ctx: &Ctx, path: &str) -> Result<Vec<u8>> {
    let rel = path.trim_start_matches('/');
    if rel.split('/').any(|s| s == ".." || s.contains(':')) {
        bail!("ungültiger Pfad");
    }
    let file = crate::util::join_rel(&ctx.cache, rel);
    let fresh = std::fs::metadata(&file)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .map(|age| !rel.ends_with(".html") || age.as_secs() < 86_400)
        .unwrap_or(false);
    if fresh {
        return Ok(tokio::fs::read(&file).await?);
    }
    let resp = ctx.http.get(format!("{UPSTREAM}/{rel}")).send().await?;
    if !resp.status().is_success() {
        // Serve a stale copy rather than nothing when offline.
        if file.exists() {
            return Ok(tokio::fs::read(&file).await?);
        }
        bail!("{}", resp.status());
    }
    let body = resp.bytes().await?.to_vec();
    if let Some(dir) = file.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    tokio::fs::write(&file, &body).await?;
    Ok(body)
}

async fn respond(stream: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) -> Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

async fn handle(mut stream: TcpStream, ctx: Arc<Ctx>) -> Result<()> {
    let mut buf = vec![0u8; 8192];
    let mut len = 0;
    loop {
        let n = stream.read(&mut buf[len..]).await?;
        if n == 0 {
            return Ok(());
        }
        len += n;
        if buf[..len].windows(4).any(|w| w == b"\r\n\r\n") || len == buf.len() {
            break;
        }
    }
    let req = String::from_utf8_lossy(&buf[..len]);
    let mut parts = req.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/"));
    if method != "GET" {
        return respond(&mut stream, "405 Method Not Allowed", "text/plain", b"").await;
    }
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let path = if path == "/" { "/index.html" } else { path };
    match path {
        "/s2mm/live" => {
            let body = serde_json::to_vec(&read_position())?;
            respond(&mut stream, "200 OK", "application/json", &body).await
        }
        "/s2mm/overlay.js" => respond(&mut stream, "200 OK", "text/javascript; charset=utf-8", OVERLAY.as_bytes()).await,
        "/favicon.ico" => respond(&mut stream, "404 Not Found", "text/plain", b"").await,
        _ => match fetch_cached(&ctx, path).await {
            Ok(body) if path == "/index.html" => {
                let html = inject(&String::from_utf8_lossy(&body));
                respond(&mut stream, "200 OK", content_type(path), html.as_bytes()).await
            }
            Ok(body) => respond(&mut stream, "200 OK", content_type(path), &body).await,
            Err(e) => {
                let msg = format!("Karte nicht erreichbar ({e:#}). Beim ersten Start wird eine Internetverbindung benötigt.");
                respond(&mut stream, "502 Bad Gateway", "text/plain; charset=utf-8", msg.as_bytes()).await
            }
        },
    }
}

pub async fn start(port: u16, cache: &Path) -> Result<Server> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("Port {port} ist belegt – in den Einstellungen einen anderen wählen"))?;
    std::fs::create_dir_all(cache)?;
    std::fs::create_dir_all(live_dir())?;
    let ctx = Arc::new(Ctx {
        cache: cache.to_path_buf(),
        http: reqwest::Client::builder().user_agent(concat!("S2MM/", env!("CARGO_PKG_VERSION"))).build()?,
    });
    let (stop, mut stopped) = tokio::sync::oneshot::channel::<()>();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut stopped => break,
                accepted = listener.accept() => {
                    if let Ok((stream, _)) = accepted {
                        let ctx = ctx.clone();
                        tauri::async_runtime::spawn(async move { let _ = handle(stream, ctx).await; });
                    }
                }
            }
        }
    });
    Ok(Server { port, stop })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_overlay_and_map_hook() {
        let html = "<title>Joric's Stalker</title><script type=\"module\">let map = null;\nwindow.saveLocalData = saveLocalData;\n</script></body></html>";
        let out = inject(html);
        assert!(out.contains("window.__s2mmMap = () => map;"));
        assert!(out.contains("<script src=\"/s2mm/overlay.js\"></script></body>"));
        assert!(out.contains("<title>S2MM Live-Karte – Stalker 2</title>"));
    }
}
