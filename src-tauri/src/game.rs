//! Game install detection, layout and launching.

use crate::tr;

use crate::model::{LaunchMethod, Platform, Settings};
use crate::util::hidden_command;
use anyhow::{bail, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const STEAM_APP_ID: &str = "1643320";
pub const STEAM_DIR: &str = "S.T.A.L.K.E.R. 2 Heart of Chornobyl";
pub const NEXUS_DOMAIN: &str = "stalker2heartofchornobyl";
pub const SHIPPING_EXES: [&str; 2] = ["Stalker2-Win64-Shipping.exe", "Stalker2-WinGDK-Shipping.exe"];

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInstall {
    pub path: String,
    pub platform: Platform,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub path: String,
    pub platform: Platform,
    pub valid: bool,
    /// Game-relative folders (forward slashes).
    pub binaries_dir: String,
    pub mods_dir: String,
    pub ue4ss_mods_dir: String,
    pub ue4ss_installed: bool,
    pub exe: Option<String>,
    pub config_dir: Option<String>,
    pub build: Option<String>,
}

impl GameInfo {
    pub fn root(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }
    pub fn abs(&self, rel: &str) -> PathBuf {
        crate::util::join_rel(&self.root(), rel)
    }
}

/// Accepts the game root or its `Stalker2` subfolder and returns the root.
pub fn normalize_root(path: &Path) -> Option<PathBuf> {
    if path.join("Stalker2").join("Content").is_dir() {
        return Some(path.to_path_buf());
    }
    if path.join("Content").join("Paks").is_dir()
        && path.file_name().map(|n| n.eq_ignore_ascii_case("Stalker2")).unwrap_or(false)
    {
        return path.parent().map(|p| p.to_path_buf());
    }
    // Game Pass: <XboxGames>\S.T.A.L.K.E.R. 2- ...\Content\Stalker2
    if path.join("Content").join("Stalker2").join("Content").is_dir() {
        return Some(path.join("Content"));
    }
    None
}

pub fn guess_platform(root: &Path) -> Platform {
    let s = root.to_string_lossy().to_lowercase();
    if s.contains("steamapps") {
        Platform::Steam
    } else if s.contains("xboxgames") || root.join("gamelaunchhelper.exe").exists() || root.join("MicrosoftGame.config").exists() {
        Platform::GamePass
    } else if s.contains("gog") || root.join("goggame-1529799785.info").exists() {
        Platform::Gog
    } else if s.contains("epic") {
        Platform::Epic
    } else {
        Platform::Unknown
    }
}

pub fn inspect(root: &Path, settings: &Settings) -> GameInfo {
    if let Ok(mut g) = GAME_ROOT.write() {
        *g = Some(root.to_path_buf());
    }
    let valid = root.join("Stalker2").join("Content").join("Paks").is_dir();
    let platform = match settings.platform {
        Platform::Unknown => guess_platform(root),
        p => p,
    };
    let bin_name = if root.join("Stalker2/Binaries/WinGDK").is_dir() && !root.join("Stalker2/Binaries/Win64").is_dir() {
        "WinGDK"
    } else {
        "Win64"
    };
    let binaries_dir = format!("Stalker2/Binaries/{bin_name}");
    let ue4ss_installed = root.join(&binaries_dir).join("ue4ss").is_dir();
    let ue4ss_mods_dir = if ue4ss_installed || !root.join(&binaries_dir).join("Mods").is_dir() {
        format!("{binaries_dir}/ue4ss/Mods")
    } else {
        format!("{binaries_dir}/Mods")
    };
    let exe = ["Stalker2.exe", "gamelaunchhelper.exe"]
        .iter()
        .map(|e| root.join(e))
        .chain(SHIPPING_EXES.iter().map(|e| root.join(&binaries_dir).join(e)))
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned());

    GameInfo {
        path: root.to_string_lossy().into_owned(),
        platform,
        valid,
        binaries_dir,
        mods_dir: "Stalker2/Content/Paks/~mods".into(),
        ue4ss_mods_dir,
        ue4ss_installed,
        exe,
        config_dir: config_dir(platform, settings).map(|p| p.to_string_lossy().into_owned()),
        build: steam_build(root),
    }
}

/// Game root of the last inspected install; on Linux it locates the Proton prefix.
static GAME_ROOT: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

/// The game's `%LOCALAPPDATA%` – on Windows the real one, on Linux the one inside the
/// Proton prefix (`steamapps/compatdata/1643320/pfx/drive_c/users/steamuser/AppData/Local`).
pub fn local_appdata() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        let root = GAME_ROOT.read().ok()?.clone()?;
        let steamapps = root.parent()?.parent()?;
        let p = steamapps
            .join("compatdata")
            .join(STEAM_APP_ID)
            .join("pfx/drive_c/users/steamuser/AppData/Local");
        p.is_dir().then_some(p)
    }
}

/// `%LOCALAPPDATA%\Stalker2\Saved\Config\Windows` (Steam/GOG) or `...\WinGDK` (Game Pass).
pub fn config_dir(platform: Platform, settings: &Settings) -> Option<PathBuf> {
    if let Some(dir) = settings.config_dir.as_ref().filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    // Strictly by platform: an old Game Pass "WinGDK" folder next to a Steam install must
    // not catch the tweaks (the Steam build only reads "Windows").
    let base = local_appdata()?.join("Stalker2/Saved/Config");
    Some(base.join(if platform == Platform::GamePass { "WinGDK" } else { "Windows" }))
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn config_location_follows_platform_even_before_folder_exists() {
        let mut settings = Settings::default();
        if local_appdata().is_some() {
            assert!(config_dir(Platform::Steam, &settings).unwrap().ends_with("Stalker2/Saved/Config/Windows"));
            assert!(config_dir(Platform::GamePass, &settings).unwrap().ends_with("Stalker2/Saved/Config/WinGDK"));
        }
        let custom = std::env::temp_dir().join("s2mm-custom-config");
        settings.config_dir = Some(custom.to_string_lossy().into_owned());
        assert_eq!(config_dir(Platform::Steam, &settings).unwrap(), custom);
        assert_eq!(config_dir(Platform::GamePass, &settings).unwrap(), custom);
    }
}

/// Steam build id from the app manifest; used to notice game patches.
fn steam_build(root: &Path) -> Option<String> {
    let steamapps = root.parent()?.parent()?;
    let manifest = std::fs::read_to_string(steamapps.join(format!("appmanifest_{STEAM_APP_ID}.acf"))).ok()?;
    vdf_values(&manifest, "buildid").into_iter().next()
}

/// Extracts every `"key" "value"` pair for a given key from a VDF/ACF text.
fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{}\"", key.to_lowercase());
    text.lines()
        .filter_map(|line| {
            let t = line.trim();
            if !t.to_lowercase().starts_with(&needle) {
                return None;
            }
            let rest = t[needle.len()..].trim();
            let v = rest.trim_matches('"');
            Some(v.replace("\\\\", "\\"))
        })
        .collect()
}

pub fn detect_installs() -> Vec<DetectedInstall> {
    let mut found: Vec<DetectedInstall> = Vec::new();
    let mut push = |p: PathBuf, platform: Platform| {
        let p = if cfg!(windows) { PathBuf::from(p.to_string_lossy().replace('/', "\\")) } else { p };
        if let Some(root) = normalize_root(&p) {
            let s = root.to_string_lossy().into_owned();
            if !found.iter().any(|f| if cfg!(windows) { f.path.eq_ignore_ascii_case(&s) } else { f.path == s }) {
                found.push(DetectedInstall { path: s, platform });
            }
        }
    };

    // Steam
    for lib in steam_libraries() {
        push(lib.join("steamapps/common").join(STEAM_DIR), Platform::Steam);
    }

    // Game Pass: <drive>:\XboxGames\S.T.A.L.K.E.R. 2...\Content
    #[cfg(windows)]
    for letter in b'C'..=b'Z' {
        let xbox = PathBuf::from(format!("{}:\\XboxGames", letter as char));
        if let Ok(rd) = std::fs::read_dir(&xbox) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if name.contains("s.t.a.l.k.e.r. 2") || name.contains("stalker 2") {
                    push(e.path().join("Content"), Platform::GamePass);
                }
            }
        }
    }

    // GOG
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(games) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\GOG.com\\Games") {
            for name in games.enum_keys().flatten() {
                if let Ok(k) = games.open_subkey(&name) {
                    let title: String = k.get_value("gameName").unwrap_or_default();
                    if title.to_lowercase().contains("s.t.a.l.k.e.r. 2") {
                        let path: String = k.get_value("path").unwrap_or_default();
                        push(PathBuf::from(path), Platform::Gog);
                    }
                }
            }
        }
    }

    // Epic
    #[cfg(windows)]
    if let Ok(rd) = std::fs::read_dir("C:\\ProgramData\\Epic\\EpicGamesLauncher\\Data\\Manifests") {
        for e in rd.flatten() {
            if let Ok(text) = std::fs::read_to_string(e.path()) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    let title = v["DisplayName"].as_str().unwrap_or("").to_lowercase();
                    if title.contains("s.t.a.l.k.e.r. 2") {
                        if let Some(loc) = v["InstallLocation"].as_str() {
                            push(PathBuf::from(loc), Platform::Epic);
                        }
                    }
                }
            }
        }
    }

    found
}

fn steam_libraries() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
            if let Ok(p) = k.get_value::<String, _>("SteamPath") {
                roots.push(PathBuf::from(p.replace('/', "\\")));
            }
        }
    }
    #[cfg(windows)]
    roots.push(PathBuf::from("C:\\Program Files (x86)\\Steam"));
    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for rel in [".steam/steam", ".local/share/Steam", ".var/app/com.valvesoftware.Steam/data/Steam", ".steam/root"] {
            roots.push(home.join(rel));
        }
    }
    let mut libs = Vec::new();
    for root in roots {
        if let Ok(text) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            libs.extend(vdf_values(&text, "path").into_iter().map(PathBuf::from));
        }
        libs.push(root);
    }
    libs.sort();
    libs.dedup();
    libs
}

#[cfg(windows)]
pub fn is_running() -> bool {
    let Ok(out) = hidden_command("tasklist").args(["/FO", "CSV", "/NH"]).output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    SHIPPING_EXES.iter().any(|e| text.contains(&e.to_lowercase()))
}

/// Under Proton the game shows up as a process whose command line names the exe.
#[cfg(not(windows))]
pub fn is_running() -> bool {
    let Ok(rd) = std::fs::read_dir("/proc") else { return false };
    rd.flatten().any(|e| {
        std::fs::read(e.path().join("cmdline"))
            .map(|c| {
                let c = String::from_utf8_lossy(&c).to_lowercase();
                SHIPPING_EXES.iter().any(|x| c.contains(&x.to_lowercase()))
            })
            .unwrap_or(false)
    })
}

/// Opens a URL with the system handler (used for steam:// links).
fn open_url(url: &str) -> Result<()> {
    #[cfg(windows)]
    hidden_command("cmd").args(["/C", "start", "", url]).spawn()?;
    #[cfg(not(windows))]
    std::process::Command::new("xdg-open").arg(url).spawn()?;
    Ok(())
}

pub fn launch(game: &GameInfo, settings: &Settings) -> Result<()> {
    let use_steam = match settings.launch_method {
        LaunchMethod::Steam => true,
        LaunchMethod::Direct => false,
        LaunchMethod::Auto => game.platform == Platform::Steam || !cfg!(windows),
    };
    if use_steam {
        let mut url = format!("steam://rungameid/{STEAM_APP_ID}");
        if !settings.launch_args.trim().is_empty() {
            url = format!("steam://run/{STEAM_APP_ID}//{}/", settings.launch_args.trim());
        }
        return open_url(&url);
    }
    let Some(exe) = &game.exe else {
        bail!(tr!("Keine Spiel-EXE gefunden. Bitte Startmethode „Steam“ wählen oder den Spielordner prüfen.", "No game executable found. Choose the “Steam” launch method or check the game folder."));
    };
    let mut cmd = std::process::Command::new(exe);
    cmd.current_dir(Path::new(exe).parent().unwrap_or(Path::new(".")));
    cmd.args(settings.launch_args.split_whitespace());
    cmd.spawn()?;
    Ok(())
}
