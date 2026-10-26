mod archive;
mod deploy;
#[cfg(test)]
mod e2e_tests;
mod fomod;
mod game;
mod gvas;
mod install;
mod keybinds;
mod layout;
mod livemap;
mod model;
mod nexus;
mod pak;
mod plan;
mod tweaks;
mod ue4ss;
mod util;

use anyhow::{anyhow, bail, Context, Result};
use game::{DetectedInstall, GameInfo};
use install::InstallMeta;
use model::{Db, Deployment, ModEntry, NexusRef, Platform, Profile, ProfileMod, Settings, UpdateInfo};
use nexus::NexusUser;
use plan::{ConflictReport, ModConflicts, ModPlanInfo, PakCache};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use util::{now, read_json, write_json};

type Res<T> = std::result::Result<T, String>;

pub struct Paths {
    pub data: PathBuf,
    pub mods: PathBuf,
    pub downloads: PathBuf,
    pub backups: PathBuf,
    pub state: PathBuf,
    pub deployment: PathBuf,
}

impl Paths {
    fn new(data: PathBuf) -> Self {
        Paths {
            mods: data.join("mods"),
            downloads: data.join("downloads"),
            backups: data.join("backups"),
            state: data.join("state.json"),
            deployment: data.join("deployment.json"),
            data,
        }
    }
}

pub struct Core {
    paths: Arc<Paths>,
    db: Db,
    deployment: Deployment,
    cache: PakCache,
    nexus_user: Option<NexusUser>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    id: String,
    file_name: String,
    mod_id: u64,
    file_id: u64,
    mod_name: Option<String>,
    version: Option<String>,
    received: u64,
    total: Option<u64>,
    /// running | done | failed | installed
    state: String,
    error: Option<String>,
    path: String,
    started_at: u64,
    #[serde(skip)]
    meta: Option<InstallMetaSer>,
    #[serde(skip)]
    replace: Option<String>,
}

#[derive(Clone, Debug)]
struct InstallMetaSer {
    name: Option<String>,
    version: Option<String>,
    author: Option<String>,
    summary: Option<String>,
    picture_url: Option<String>,
    nexus: Option<NexusRef>,
}

impl From<InstallMetaSer> for InstallMeta {
    fn from(m: InstallMetaSer) -> Self {
        InstallMeta { name: m.name, version: m.version, author: m.author, summary: m.summary, picture_url: m.picture_url, nexus: m.nexus }
    }
}

#[derive(Clone)]
pub struct AppCore {
    core: Arc<Mutex<Core>>,
    paths: Arc<Paths>,
    downloads: Arc<Mutex<Vec<Download>>>,
    map_server: Arc<Mutex<Option<livemap::Server>>>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ToolMod {
    mod_id: Option<String>,
    enabled: bool,
    version: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ToolsView {
    ue4ss: ToolMod,
    /// UE4SS files are present in the game folder (installed by us or manually).
    ue4ss_in_game: bool,
    live_map_mod: ToolMod,
    mcm: ToolMod,
}

impl AppCore {
    fn lock(&self) -> MutexGuard<'_, Core> {
        self.core.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl Core {
    fn save(&self) -> Result<()> {
        write_json(&self.paths.state, &self.db)
    }

    fn game(&self) -> Option<GameInfo> {
        let path = self.db.settings.game_path.as_ref()?;
        Some(game::inspect(Path::new(path), &self.db.settings))
    }

    fn require_game(&self) -> Result<GameInfo> {
        let g = self.game().ok_or_else(|| anyhow!("Kein Spielordner festgelegt"))?;
        if !g.valid {
            bail!("Der Spielordner {} enthält keine Stalker-2-Installation", g.path);
        }
        Ok(g)
    }

    fn state_view(&mut self) -> StateView {
        let game = self.game();
        let mut per_mod: BTreeMap<String, ModPlanInfo> = BTreeMap::new();
        let mut conflicts = ConflictReport::default();
        let mut pending = false;
        let mut unmanaged = Vec::new();
        if let Some(g) = game.as_ref().filter(|g| g.valid) {
            let plan = plan::build(&self.db, g, &self.paths.mods);
            pending = plan.fingerprint != self.deployment.fingerprint
                && !(plan.files.is_empty() && self.deployment.files.is_empty());
            conflicts = plan::conflicts(&plan, &mut self.cache);
            per_mod = plan.per_mod.into_iter().collect();
            unmanaged = deploy::unmanaged(g, &self.deployment);
        }
        let profile = self.db.profile().clone();
        let mods = profile
            .mods
            .iter()
            .enumerate()
            .filter_map(|(i, pm)| {
                let entry = self.db.find_mod(&pm.id)?.clone();
                Some(ModView {
                    plan: per_mod.get(&pm.id).cloned().unwrap_or_default(),
                    conflicts: conflicts.per_mod.get(&pm.id).cloned().unwrap_or_default(),
                    enabled: pm.enabled,
                    priority: i + 1,
                    entry,
                })
            })
            .collect();
        let game_updated = match (&game, &self.deployment.game_build) {
            (Some(g), Some(b)) => g.build.as_ref().map(|x| x != b).unwrap_or(false) && !self.deployment.files.is_empty(),
            _ => false,
        };
        StateView {
            detected: if game.is_none() { game::detect_installs() } else { Vec::new() },
            settings: self.db.settings.clone(),
            game,
            mods,
            profiles: self.db.profiles.iter().map(|p| ProfileInfo { id: p.id.clone(), name: p.name.clone() }).collect(),
            active_profile: self.db.active_profile.clone(),
            pending,
            applied_at: self.deployment.applied_at,
            deployed_files: self.deployment.files.len(),
            unmanaged,
            game_updated,
            nexus_user: self.nexus_user.clone(),
            data_dir: self.paths.data.to_string_lossy().into_owned(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            conflict_errors: conflicts.errors.len(),
            tools: self.tools(),
        }
    }

    fn tool_mod(&self, id: Option<&String>, name_hint: &str) -> ToolMod {
        let entry = id
            .and_then(|id| self.db.find_mod(id))
            .or_else(|| self.db.mods.iter().find(|m| m.name.to_lowercase().contains(name_hint)));
        match entry {
            Some(e) => ToolMod {
                mod_id: Some(e.id.clone()),
                enabled: self.db.profile().mods.iter().any(|p| p.id == e.id && p.enabled),
                version: e.version.clone(),
            },
            None => ToolMod { mod_id: None, enabled: false, version: None },
        }
    }

    fn tools(&self) -> ToolsView {
        let s = &self.db.settings;
        ToolsView {
            ue4ss: self.tool_mod(s.ue4ss_mod_id.as_ref(), "ue4ss"),
            ue4ss_in_game: self.game().map(|g| g.ue4ss_installed).unwrap_or(false),
            live_map_mod: self.tool_mod(s.livemap_mod_id.as_ref(), "s2mm live-map"),
            mcm: self.tool_mod(None, "mod configuration menu"),
        }
    }

    fn insert_mod(&mut self, entry: ModEntry, enable: bool) {
        let id = entry.id.clone();
        match self.db.find_mod_mut(&id) {
            Some(existing) => *existing = entry,
            None => {
                self.db.mods.push(entry);
                let active = self.db.active_profile.clone();
                for p in &mut self.db.profiles {
                    p.mods.push(ProfileMod { id: id.clone(), enabled: enable && p.id == active });
                }
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModView {
    #[serde(flatten)]
    entry: ModEntry,
    enabled: bool,
    priority: usize,
    plan: ModPlanInfo,
    conflicts: ModConflicts,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileInfo {
    id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StateView {
    settings: Settings,
    game: Option<GameInfo>,
    detected: Vec<DetectedInstall>,
    mods: Vec<ModView>,
    profiles: Vec<ProfileInfo>,
    active_profile: String,
    pending: bool,
    applied_at: u64,
    deployed_files: usize,
    unmanaged: Vec<String>,
    game_updated: bool,
    nexus_user: Option<NexusUser>,
    data_dir: String,
    app_version: String,
    conflict_errors: usize,
    tools: ToolsView,
}

async fn with_core<T: Send + 'static>(app: &AppCore, f: impl FnOnce(&mut Core) -> Result<T> + Send + 'static) -> Res<T> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut core = app.lock();
        f(&mut core).map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn notice(app: &AppHandle, kind: &str, message: impl Into<String>) {
    let _ = app.emit("notice", serde_json::json!({ "kind": kind, "message": message.into() }));
}

fn emit_downloads(app: &AppHandle, core: &AppCore) {
    let list = core.downloads.lock().unwrap_or_else(|p| p.into_inner()).clone();
    let _ = app.emit("downloads-changed", list);
}

// ---------------------------------------------------------------- state & game

#[tauri::command]
async fn get_state(core: State<'_, AppCore>) -> Res<StateView> {
    with_core(&core, |c| Ok(c.state_view())).await
}

#[tauri::command]
async fn detect_games() -> Res<Vec<DetectedInstall>> {
    tauri::async_runtime::spawn_blocking(game::detect_installs).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_game_path(core: State<'_, AppCore>, path: String, platform: Option<Platform>) -> Res<StateView> {
    with_core(&core, move |c| {
        let root = game::normalize_root(Path::new(&path))
            .ok_or_else(|| anyhow!("In {path} wurde kein Stalker 2 gefunden (erwartet: Ordner mit „Stalker2\\Content\\Paks“)."))?;
        c.db.settings.game_path = Some(root.to_string_lossy().into_owned());
        c.db.settings.platform = platform.unwrap_or_else(|| game::guess_platform(&root));
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn save_settings(app: AppHandle, core: State<'_, AppCore>, settings: Settings) -> Res<StateView> {
    let _ = app;
    with_core(&core, move |c| {
        let mut settings = settings;
        if settings.nxm_handler != c.db.settings.nxm_handler {
            let prev = nexus::set_protocol_handler(settings.nxm_handler, c.db.settings.nxm_previous.as_deref())?;
            settings.nxm_previous = if settings.nxm_handler { prev.or(c.db.settings.nxm_previous.clone()) } else { None };
        } else {
            settings.nxm_previous = c.db.settings.nxm_previous.clone();
        }
        if settings.nexus_api_key != c.db.settings.nexus_api_key {
            c.nexus_user = None;
        }
        c.db.settings = settings;
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn launch_game(core: State<'_, AppCore>) -> Res<()> {
    with_core(&core, |c| {
        let g = c.require_game()?;
        game::launch(&g, &c.db.settings)
    })
    .await
}

#[tauri::command]
async fn open_location(app: AppHandle, core: State<'_, AppCore>, what: String) -> Res<()> {
    let path = with_core(&core, move |c| {
        let p: PathBuf = match what.as_str() {
            "game" => c.require_game()?.root(),
            "mods" => {
                let g = c.require_game()?;
                let p = g.abs(&g.mods_dir);
                std::fs::create_dir_all(&p)?;
                p
            }
            "config" => PathBuf::from(c.game().and_then(|g| g.config_dir).ok_or_else(|| anyhow!("Konfigurationsordner unbekannt"))?),
            "data" => c.paths.data.clone(),
            "downloads" => c.paths.downloads.clone(),
            other => match other.strip_prefix("mod:") {
                Some(id) => {
                    let e = c.db.find_mod(id).ok_or_else(|| anyhow!("Mod nicht gefunden"))?;
                    util::join_rel(&plan::mod_files_dir(&c.paths.mods, id), &e.layout.root)
                }
                None => bail!("Unbekannter Ort"),
            },
        };
        std::fs::create_dir_all(&p).ok();
        Ok(p)
    })
    .await?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_url(app: AppHandle, url: String) -> Res<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Ungültige URL".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- mods

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddResult {
    state: StateView,
    added: Vec<String>,
    errors: Vec<String>,
}

fn install_one(core: &AppCore, src: &Path, meta: InstallMeta, replace: Option<String>, enable: bool) -> Result<ModEntry> {
    let existing = replace.and_then(|id| core.lock().db.find_mod(&id).cloned());
    std::fs::create_dir_all(&core.paths.mods)?;
    let entry = install::stage(&core.paths.mods, src, meta, existing.as_ref())?;
    let mut c = core.lock();
    c.insert_mod(entry.clone(), enable);
    c.save()?;
    Ok(entry)
}

#[tauri::command]
async fn add_mods(core: State<'_, AppCore>, paths: Vec<String>) -> Res<AddResult> {
    let app = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut added = Vec::new();
        let mut errors = Vec::new();
        for p in paths {
            let path = PathBuf::from(&p);
            match install_one(&app, &path, InstallMeta::default(), None, true) {
                Ok(e) => added.push(e.id),
                Err(e) => errors.push(format!("{}: {e:#}", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(p))),
            }
        }
        let state = app.lock().state_view();
        AddResult { state, added, errors }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reinstall_mod(core: State<'_, AppCore>, id: String, path: String) -> Res<StateView> {
    let app = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        install_one(&app, Path::new(&path), InstallMeta::default(), Some(id), true).map_err(|e| format!("{e:#}"))?;
        Ok(app.lock().state_view())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remove_mod(core: State<'_, AppCore>, id: String) -> Res<StateView> {
    with_core(&core, move |c| {
        c.db.mods.retain(|m| m.id != id);
        c.db.normalize();
        c.save()?;
        let dir = c.paths.mods.join(&id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).with_context(|| format!("{} löschen", dir.display()))?;
        }
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn set_enabled(core: State<'_, AppCore>, ids: Vec<String>, enabled: bool) -> Res<StateView> {
    with_core(&core, move |c| {
        for pm in c.db.profile_mut().mods.iter_mut().filter(|pm| ids.contains(&pm.id)) {
            pm.enabled = enabled;
        }
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn reorder_mods(core: State<'_, AppCore>, ids: Vec<String>) -> Res<StateView> {
    with_core(&core, move |c| {
        let profile = c.db.profile_mut();
        let mut next: Vec<ProfileMod> = ids
            .iter()
            .filter_map(|id| profile.mods.iter().find(|m| &m.id == id).cloned())
            .collect();
        for m in &profile.mods {
            if !next.iter().any(|n| n.id == m.id) {
                next.push(m.clone());
            }
        }
        profile.mods = next;
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn set_selection(core: State<'_, AppCore>, id: String, selection: BTreeMap<String, Vec<String>>) -> Res<StateView> {
    with_core(&core, move |c| {
        let m = c.db.find_mod_mut(&id).ok_or_else(|| anyhow!("Mod nicht gefunden"))?;
        m.selection = selection;
        m.needs_setup = false;
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModPatch {
    name: Option<String>,
    version: Option<String>,
    notes: Option<String>,
    category: Option<String>,
    nexus_mod_id: Option<u64>,
}

#[tauri::command]
async fn update_mod(core: State<'_, AppCore>, id: String, patch: ModPatch) -> Res<StateView> {
    with_core(&core, move |c| {
        let m = c.db.find_mod_mut(&id).ok_or_else(|| anyhow!("Mod nicht gefunden"))?;
        if let Some(n) = patch.name.filter(|n| !n.trim().is_empty()) {
            m.name = n.trim().to_string();
        }
        if let Some(v) = patch.version {
            m.version = Some(v).filter(|v| !v.trim().is_empty());
        }
        if let Some(n) = patch.notes {
            m.notes = n;
        }
        if let Some(cat) = patch.category {
            m.category = Some(cat).filter(|c| !c.trim().is_empty());
        }
        if let Some(nid) = patch.nexus_mod_id {
            m.nexus = (nid > 0).then(|| NexusRef { game_domain: game::NEXUS_DOMAIN.into(), mod_id: nid, file_id: None });
            m.update = None;
        }
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyOutcome {
    state: StateView,
    placed: usize,
    removed: usize,
    kept: usize,
}

#[tauri::command]
async fn apply(core: State<'_, AppCore>) -> Res<ApplyOutcome> {
    with_core(&core, |c| {
        let g = c.require_game()?;
        let plan = plan::build(&c.db, &g, &c.paths.mods);
        let result = deploy::apply(&g, &plan, &c.deployment, &c.paths.backups, c.db.settings.use_hardlinks);
        let r = result?;
        c.deployment = r.deployment;
        write_json(&c.paths.deployment, &c.deployment)?;
        Ok(ApplyOutcome { state: c.state_view(), placed: r.placed, removed: r.removed, kept: r.kept })
    })
    .await
}

#[tauri::command]
async fn purge(core: State<'_, AppCore>) -> Res<StateView> {
    with_core(&core, |c| {
        let g = c.require_game()?;
        deploy::purge(&g, &c.deployment)?;
        c.deployment = Deployment::default();
        write_json(&c.paths.deployment, &c.deployment)?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn get_conflicts(core: State<'_, AppCore>) -> Res<ConflictReport> {
    with_core(&core, |c| {
        let g = c.require_game()?;
        let plan = plan::build(&c.db, &g, &c.paths.mods);
        Ok(plan::conflicts(&plan, &mut c.cache))
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileRow {
    logical: String,
    target: String,
    kind: layout::TargetKind,
    size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PakRow {
    file: String,
    assets: Vec<String>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModDetails {
    files: Vec<FileRow>,
    unmapped: Vec<String>,
    paks: Vec<PakRow>,
    config_files: Vec<String>,
    size: u64,
}

#[tauri::command]
async fn mod_details(core: State<'_, AppCore>, id: String) -> Res<ModDetails> {
    with_core(&core, move |c| {
        let g = c.require_game()?;
        let prio = c.db.profile().mods.iter().position(|m| m.id == id).map(|i| i + 1).unwrap_or(1);
        let (files, info) = plan::plan_mod(&c.db, &g, &c.paths.mods, &id, prio);
        let files_dir = plan::mod_files_dir(&c.paths.mods, &id);
        let mut paks = Vec::new();
        let mut config_files = Vec::new();
        for f in &files {
            let ext = util::ext_lower(&f.logical);
            if ext == "pak" || ext == "utoc" {
                match c.cache.assets(&f.src) {
                    Ok(a) if !a.is_empty() => paks.push(PakRow { file: f.logical.clone(), assets: (*a).clone(), error: None }),
                    Ok(_) => {}
                    Err(e) => paks.push(PakRow { file: f.logical.clone(), assets: Vec::new(), error: Some(e) }),
                }
            }
            if util::is_text_ext(&ext) && f.size < 2_000_000 {
                config_files.push(util::rel_path(&files_dir, &f.src));
            }
        }
        Ok(ModDetails {
            files: files.into_iter().map(|f| FileRow { logical: f.logical, target: f.target, kind: f.kind, size: f.size }).collect(),
            unmapped: info.unmapped,
            paks,
            config_files,
            size: util::dir_size(&files_dir),
        })
    })
    .await
}

fn safe_mod_path(c: &Core, id: &str, rel: &str) -> Result<PathBuf> {
    let base = plan::mod_files_dir(&c.paths.mods, id).canonicalize()?;
    let p = util::join_rel(&base, rel).canonicalize().context("Datei nicht gefunden")?;
    if !p.starts_with(&base) {
        bail!("Ungültiger Pfad");
    }
    Ok(p)
}

#[tauri::command]
async fn read_mod_file(core: State<'_, AppCore>, id: String, path: String) -> Res<String> {
    with_core(&core, move |c| {
        let p = safe_mod_path(c, &id, &path)?;
        let bytes = std::fs::read(&p)?;
        Ok(String::from_utf8_lossy(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes)).into_owned())
    })
    .await
}

#[tauri::command]
async fn write_mod_file(core: State<'_, AppCore>, id: String, path: String, content: String) -> Res<StateView> {
    with_core(&core, move |c| {
        let p = safe_mod_path(c, &id, &path)?;
        std::fs::write(&p, content)?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn import_unmanaged(core: State<'_, AppCore>, files: Vec<String>) -> Res<AddResult> {
    let app = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let game = app.lock().require_game().map_err(|e| format!("{e:#}"))?;
        // Group IoStore siblings by stem.
        let mut groups: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for f in &files {
            if !f.replace('\\', "/").starts_with(&game.mods_dir) {
                continue;
            }
            let p = game.abs(f);
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            groups.entry(stem).or_default().push(p);
        }
        let mut added = Vec::new();
        let mut errors = Vec::new();
        for (stem, paths) in groups {
            let tmp = app.paths.data.join("import-tmp").join(&stem);
            let _ = std::fs::remove_dir_all(&tmp);
            let r: Result<ModEntry> = (|| {
                std::fs::create_dir_all(&tmp)?;
                for p in &paths {
                    std::fs::copy(p, tmp.join(p.file_name().unwrap()))?;
                }
                let entry = install_one(&app, &tmp, InstallMeta { name: Some(paths[0].file_stem().unwrap().to_string_lossy().into_owned()), ..Default::default() }, None, true)?;
                for p in &paths {
                    std::fs::remove_file(p)?;
                }
                Ok(entry)
            })();
            let _ = std::fs::remove_dir_all(&tmp);
            match r {
                Ok(e) => added.push(e.id),
                Err(e) => errors.push(format!("{stem}: {e:#}")),
            }
        }
        let state = app.lock().state_view();
        Ok(AddResult { state, added, errors })
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------- profiles

#[tauri::command]
async fn create_profile(core: State<'_, AppCore>, name: String, copy_current: bool) -> Res<StateView> {
    with_core(&core, move |c| {
        let mods = if copy_current {
            c.db.profile().mods.clone()
        } else {
            c.db.profile().mods.iter().map(|m| ProfileMod { id: m.id.clone(), enabled: false }).collect()
        };
        let id = uuid::Uuid::new_v4().simple().to_string();
        c.db.profiles.push(Profile { id: id.clone(), name: name.trim().to_string(), mods });
        c.db.active_profile = id;
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn rename_profile(core: State<'_, AppCore>, id: String, name: String) -> Res<StateView> {
    with_core(&core, move |c| {
        if let Some(p) = c.db.profiles.iter_mut().find(|p| p.id == id) {
            p.name = name.trim().to_string();
        }
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn delete_profile(core: State<'_, AppCore>, id: String) -> Res<StateView> {
    with_core(&core, move |c| {
        if c.db.profiles.len() <= 1 {
            bail!("Das letzte Profil kann nicht gelöscht werden");
        }
        c.db.profiles.retain(|p| p.id != id);
        c.db.normalize();
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn switch_profile(core: State<'_, AppCore>, id: String) -> Res<StateView> {
    with_core(&core, move |c| {
        if c.db.profiles.iter().any(|p| p.id == id) {
            c.db.active_profile = id;
            c.save()?;
        }
        Ok(c.state_view())
    })
    .await
}

// ---------------------------------------------------------------- tweaks

fn config_dir(c: &Core) -> Option<PathBuf> {
    let platform = c.game().map(|g| g.platform).unwrap_or(c.db.settings.platform);
    game::config_dir(platform, &c.db.settings)
}

#[tauri::command]
async fn get_tweaks(core: State<'_, AppCore>) -> Res<tweaks::TweakState> {
    with_core(&core, |c| Ok(tweaks::read(config_dir(c).as_deref()))).await
}

#[tauri::command]
async fn set_tweaks(core: State<'_, AppCore>, changes: BTreeMap<String, Option<String>>) -> Res<tweaks::TweakState> {
    with_core(&core, move |c| {
        let dir = config_dir(c).ok_or_else(|| anyhow!("Konfigurationsordner unbekannt"))?;
        tweaks::write(&dir, &changes)?;
        Ok(tweaks::read(Some(&dir)))
    })
    .await
}

#[tauri::command]
async fn restore_tweaks(core: State<'_, AppCore>) -> Res<tweaks::TweakState> {
    with_core(&core, |c| {
        let dir = config_dir(c).ok_or_else(|| anyhow!("Konfigurationsordner unbekannt"))?;
        tweaks::restore(&dir)?;
        Ok(tweaks::read(Some(&dir)))
    })
    .await
}

#[tauri::command]
async fn export_engine_ini(core: State<'_, AppCore>, path: String) -> Res<()> {
    with_core(&core, move |c| {
        let dir = config_dir(c).ok_or_else(|| anyhow!("Konfigurationsordner unbekannt"))?;
        tweaks::export(&dir, Path::new(&path))
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportOutcome {
    entries: usize,
    state: tweaks::TweakState,
}

#[tauri::command]
async fn import_engine_ini(core: State<'_, AppCore>, path: String, replace: bool) -> Res<ImportOutcome> {
    with_core(&core, move |c| {
        let dir = config_dir(c).ok_or_else(|| anyhow!("Konfigurationsordner unbekannt"))?;
        let entries = tweaks::import(&dir, Path::new(&path), replace)?;
        Ok(ImportOutcome { entries, state: tweaks::read(Some(&dir)) })
    })
    .await
}

// ---------------------------------------------------------------- nexus

fn api_key(core: &AppCore) -> Result<String> {
    core.lock().db.settings.nexus_api_key.clone().filter(|k| !k.trim().is_empty()).ok_or_else(|| {
        anyhow!("Kein Nexus-API-Schlüssel hinterlegt (Einstellungen → Nexus Mods).")
    })
}

#[tauri::command]
async fn nexus_login(core: State<'_, AppCore>, key: String) -> Res<StateView> {
    let client = nexus::Client::new(&key).map_err(|e| format!("{e:#}"))?;
    let user = client.validate().await.map_err(|e| format!("{e:#}"))?;
    with_core(&core, move |c| {
        c.db.settings.nexus_api_key = Some(key.trim().to_string());
        c.nexus_user = Some(user);
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn nexus_logout(core: State<'_, AppCore>) -> Res<StateView> {
    with_core(&core, |c| {
        c.db.settings.nexus_api_key = None;
        c.nexus_user = None;
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

fn normalize_version(v: &str) -> String {
    v.trim().trim_start_matches(['v', 'V']).trim_end_matches(".0").to_string()
}

async fn check_updates_inner(core: &AppCore) -> Result<usize> {
    let key = api_key(core)?;
    let client = nexus::Client::new(&key)?;
    if core.lock().nexus_user.is_none() {
        let user = client.validate().await?;
        core.lock().nexus_user = Some(user);
    }
    let targets: Vec<(String, NexusRef, Option<String>)> = core
        .lock()
        .db
        .mods
        .iter()
        .filter_map(|m| m.nexus.clone().map(|n| (m.id.clone(), n, m.version.clone())))
        .collect();
    let mut found = 0;
    for (id, nref, version) in targets {
        let info = match client.mod_info(&nref.game_domain, nref.mod_id).await {
            Ok(i) => i,
            Err(_) => continue,
        };
        let files = client.files(&nref.game_domain, nref.mod_id).await.ok();
        let mut update = UpdateInfo { checked_at: now(), latest_version: info.version.clone(), ..Default::default() };
        if let (Some(fid), Some(list)) = (nref.file_id, files.as_ref()) {
            if let Some(newest) = nexus::newest_file(list, fid) {
                update.available = true;
                update.latest_file_id = Some(newest);
                update.latest_version = list.files.iter().find(|f| f.file_id == newest).and_then(|f| f.version.clone()).or(update.latest_version);
            }
        } else if let (Some(latest), Some(current)) = (&info.version, &version) {
            update.available = normalize_version(latest) != normalize_version(current);
        }
        if update.available {
            found += 1;
        }
        let mut c = core.lock();
        if let Some(m) = c.db.find_mod_mut(&id) {
            m.update = Some(update);
            if m.picture_url.is_none() {
                m.picture_url = info.picture_url.clone();
            }
            if m.summary.is_none() {
                m.summary = info.summary.clone();
            }
            if m.author.is_none() {
                m.author = info.author.clone();
            }
        }
    }
    core.lock().save()?;
    Ok(found)
}

#[tauri::command]
async fn check_updates(core: State<'_, AppCore>) -> Res<StateView> {
    let app = core.inner().clone();
    check_updates_inner(&app).await.map_err(|e| format!("{e:#}"))?;
    with_core(&core, |c| Ok(c.state_view())).await
}

#[tauri::command]
async fn nexus_update_mod(app: AppHandle, core: State<'_, AppCore>, id: String) -> Res<()> {
    let entry = core.lock().db.find_mod(&id).cloned().ok_or("Mod nicht gefunden")?;
    let nref = entry.nexus.clone().ok_or("Mod ist nicht mit Nexus verknüpft")?;
    let premium = core.lock().nexus_user.as_ref().map(|u| u.is_premium).unwrap_or(false);
    let file_id = entry.update.as_ref().and_then(|u| u.latest_file_id);
    let url = format!("https://www.nexusmods.com/{}/mods/{}?tab=files", nref.game_domain, nref.mod_id);
    match (premium, file_id) {
        (true, Some(fid)) => {
            let link = nexus::NxmLink { game: nref.game_domain, mod_id: nref.mod_id, file_id: fid, key: None, expires: None };
            start_download(app, core.inner().clone(), link, Some(id));
            Ok(())
        }
        _ => {
            // Free accounts must use "Mod Manager Download" on the website.
            app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
async fn handle_nxm(app: AppHandle, core: State<'_, AppCore>, url: String) -> Res<()> {
    let link = nexus::parse_nxm(&url).map_err(|e| format!("{e:#}"))?;
    if link.game != game::NEXUS_DOMAIN {
        return Err(format!("Dieser Link gehört zu einem anderen Spiel ({}).", link.game));
    }
    start_download(app, core.inner().clone(), link, None);
    Ok(())
}

fn start_download(app: AppHandle, core: AppCore, link: nexus::NxmLink, replace: Option<String>) {
    tauri::async_runtime::spawn(async move {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let result = download_task(&app, &core, &id, link, replace).await;
        if let Err(e) = result {
            let msg = format!("{e:#}");
            {
                let mut list = core.downloads.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(d) = list.iter_mut().find(|d| d.id == id) {
                    d.state = "failed".into();
                    d.error = Some(msg.clone());
                }
            }
            emit_downloads(&app, &core);
            notice(&app, "error", format!("Download fehlgeschlagen: {msg}"));
        }
    });
}

async fn download_task(app: &AppHandle, core: &AppCore, id: &str, link: nexus::NxmLink, replace: Option<String>) -> Result<()> {
    let key = api_key(core)?;
    let client = nexus::Client::new(&key)?;
    let game = link.game.clone();
    let info = client.mod_info(&game, link.mod_id).await.ok();
    let file = client.file_info(&game, link.mod_id, link.file_id).await.ok();
    let url = client.download_url(&game, link.mod_id, link.file_id, link.key.as_deref(), link.expires.as_deref()).await?;
    let file_name = file
        .as_ref()
        .and_then(|f| f.file_name.clone())
        .or_else(|| nexus::file_name_from_url(&url))
        .unwrap_or_else(|| format!("{}-{}.zip", link.mod_id, link.file_id));
    let file_name: String = file_name.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) { '_' } else { c }).collect();
    std::fs::create_dir_all(&core.paths.downloads)?;
    let dest = core.paths.downloads.join(&file_name);

    // An existing mod from the same Nexus page whose update points at this file gets replaced.
    let replace = replace.or_else(|| {
        let c = core.lock();
        c.db.mods
            .iter()
            .find(|m| {
                m.nexus.as_ref().map(|n| n.mod_id == link.mod_id).unwrap_or(false)
                    && (m.update.as_ref().and_then(|u| u.latest_file_id) == Some(link.file_id)
                        || m.nexus.as_ref().and_then(|n| n.file_id) == Some(link.file_id))
            })
            .map(|m| m.id.clone())
    });
    let meta = InstallMetaSer {
        name: info.as_ref().and_then(|i| i.name.clone()),
        version: file.as_ref().and_then(|f| f.version.clone()).or(info.as_ref().and_then(|i| i.version.clone())),
        author: info.as_ref().and_then(|i| i.author.clone()),
        summary: info.as_ref().and_then(|i| i.summary.clone()),
        picture_url: info.as_ref().and_then(|i| i.picture_url.clone()),
        nexus: Some(NexusRef { game_domain: game.clone(), mod_id: link.mod_id, file_id: Some(link.file_id) }),
    };
    {
        let mut list = core.downloads.lock().unwrap_or_else(|p| p.into_inner());
        list.retain(|d| !(d.file_id == link.file_id && d.state != "running"));
        list.insert(0, Download {
            id: id.into(),
            file_name: file_name.clone(),
            mod_id: link.mod_id,
            file_id: link.file_id,
            mod_name: meta.name.clone(),
            version: meta.version.clone(),
            received: 0,
            total: file.as_ref().and_then(|f| f.size_kb).map(|k| k * 1024),
            state: "running".into(),
            error: None,
            path: dest.to_string_lossy().into_owned(),
            started_at: now(),
            meta: Some(meta.clone()),
            replace: replace.clone(),
        });
    }
    emit_downloads(app, core);
    notice(app, "info", format!("Download gestartet: {}", meta.name.clone().unwrap_or(file_name.clone())));

    client
        .download(&url, &dest, |received, total| {
            {
                let mut list = core.downloads.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(d) = list.iter_mut().find(|d| d.id == id) {
                    d.received = received;
                    if total.is_some() {
                        d.total = total;
                    }
                }
            }
            emit_downloads(app, core);
        })
        .await?;
    set_download_state(core, id, "done", None);
    emit_downloads(app, core);

    if core.lock().db.settings.auto_install_downloads {
        install_download_inner(app, core, id).await?;
    }
    Ok(())
}

fn set_download_state(core: &AppCore, id: &str, state: &str, error: Option<String>) {
    let mut list = core.downloads.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(d) = list.iter_mut().find(|d| d.id == id) {
        d.state = state.into();
        d.error = error;
    }
}

async fn install_download_inner(app: &AppHandle, core: &AppCore, id: &str) -> Result<()> {
    let d = core
        .downloads
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .find(|d| d.id == id)
        .cloned()
        .ok_or_else(|| anyhow!("Download nicht gefunden"))?;
    let c2 = core.clone();
    let meta: InstallMeta = d.meta.clone().map(Into::into).unwrap_or_default();
    let path = PathBuf::from(&d.path);
    let replace = d.replace.clone();
    let entry = tauri::async_runtime::spawn_blocking(move || install_one(&c2, &path, meta, replace, true))
        .await
        .map_err(|e| anyhow!("{e}"))??;
    if let Some(m) = core.lock().db.find_mod_mut(&entry.id) {
        m.update = None;
    }
    core.lock().save()?;
    set_download_state(core, id, "installed", None);
    emit_downloads(app, core);
    let _ = app.emit("state-changed", ());
    let verb = if d.replace.is_some() { "aktualisiert" } else { "installiert" };
    notice(app, "success", format!("{} {verb}", entry.name));
    Ok(())
}

#[tauri::command]
async fn get_downloads(core: State<'_, AppCore>) -> Res<Vec<Download>> {
    Ok(core.downloads.lock().unwrap_or_else(|p| p.into_inner()).clone())
}

#[tauri::command]
async fn install_download(app: AppHandle, core: State<'_, AppCore>, id: String) -> Res<()> {
    install_download_inner(&app, core.inner(), &id).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn remove_download(app: AppHandle, core: State<'_, AppCore>, id: String, delete_file: bool) -> Res<()> {
    let removed = {
        let mut list = core.downloads.lock().unwrap_or_else(|p| p.into_inner());
        let pos = list.iter().position(|d| d.id == id && d.state != "running");
        pos.map(|i| list.remove(i))
    };
    if let (Some(d), true) = (removed, delete_file) {
        let _ = std::fs::remove_file(&d.path);
    }
    emit_downloads(&app, core.inner());
    Ok(())
}

// ---------------------------------------------------------------- tools: UE4SS, live map

#[tauri::command]
async fn ue4ss_install(core: State<'_, AppCore>) -> Res<StateView> {
    let app = core.inner().clone();
    let (platform, existing) = {
        let c = app.lock();
        let platform = c.game().map(|g| g.platform).unwrap_or(c.db.settings.platform);
        (platform, c.tool_mod(c.db.settings.ue4ss_mod_id.as_ref(), "ue4ss").mod_id)
    };
    let work = app.paths.data.join("ue4ss-download");
    let prepared = ue4ss::prepare(&work, platform).await.map_err(|e| format!("{e:#}"))?;
    let app2 = app.clone();
    let entry = tauri::async_runtime::spawn_blocking(move || {
        let meta = InstallMeta {
            name: Some("UE4SS".into()),
            version: Some(prepared.version.clone()),
            author: Some("UE4SS-RE".into()),
            summary: Some("Lua-Scripting-Framework für Unreal Engine – Voraussetzung für Lua-Mods und die Live-Karte.".into()),
            ..Default::default()
        };
        install_one(&app2, &prepared.folder, meta, existing, true)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))?;
    let _ = std::fs::remove_dir_all(&work);
    with_core(&core, move |c| {
        c.db.settings.ue4ss_mod_id = Some(entry.id.clone());
        // Make sure UE4SS is enabled in the active profile.
        if let Some(pm) = c.db.profile_mut().mods.iter_mut().find(|m| m.id == entry.id) {
            pm.enabled = true;
        }
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn set_tool_enabled(core: State<'_, AppCore>, tool: String, enabled: bool) -> Res<StateView> {
    with_core(&core, move |c| {
        let tools = c.tools();
        let id = match tool.as_str() {
            "ue4ss" => tools.ue4ss.mod_id,
            "livemap" => tools.live_map_mod.mod_id,
            "mcm" => tools.mcm.mod_id,
            _ => None,
        }
        .ok_or_else(|| anyhow!("Nicht installiert"))?;
        if let Some(pm) = c.db.profile_mut().mods.iter_mut().find(|m| m.id == id) {
            pm.enabled = enabled;
        }
        c.save()?;
        Ok(c.state_view())
    })
    .await
}

#[tauri::command]
async fn livemap_install_mod(core: State<'_, AppCore>) -> Res<StateView> {
    let app = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<StateView> {
        let existing = {
            let c = app.lock();
            c.tool_mod(c.db.settings.livemap_mod_id.as_ref(), "s2mm live-map").mod_id
        };
        let work = app.paths.data.join("livemap-build").join("S2MM_LiveMap");
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(work.join("Scripts"))?;
        std::fs::write(work.join("Scripts/main.lua"), livemap::LUA_MOD)?;
        std::fs::write(work.join("enabled.txt"), b"")?;
        std::fs::create_dir_all(livemap::live_dir())?;
        let meta = InstallMeta {
            name: Some("S2MM Live-Map".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            author: Some("S2MM".into()),
            summary: Some("Schreibt die Spielerposition für die Live-Karte (benötigt UE4SS).".into()),
            ..Default::default()
        };
        let entry = install_one(&app, &work, meta, existing, true)?;
        let _ = std::fs::remove_dir_all(work.parent().unwrap());
        let mut c = app.lock();
        c.db.settings.livemap_mod_id = Some(entry.id.clone());
        if let Some(pm) = c.db.profile_mut().mods.iter_mut().find(|m| m.id == entry.id) {
            pm.enabled = true;
        }
        c.save()?;
        Ok(c.state_view())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveStatus {
    running: bool,
    url: Option<String>,
    position: livemap::LivePosition,
}

fn live_status_of(core: &AppCore) -> LiveStatus {
    let port = core.map_server.lock().unwrap_or_else(|p| p.into_inner()).as_ref().map(|s| s.port);
    LiveStatus {
        running: port.is_some(),
        url: port.map(|p| format!("http://127.0.0.1:{p}/")),
        position: livemap::read_position(),
    }
}

#[tauri::command]
async fn livemap_start(app: AppHandle, core: State<'_, AppCore>, open_browser: bool) -> Res<LiveStatus> {
    let running = core.map_server.lock().unwrap_or_else(|p| p.into_inner()).is_some();
    if !running {
        let port = core.lock().db.settings.map_port;
        let server = livemap::start(port, &core.paths.data.join("mapcache")).await.map_err(|e| format!("{e:#}"))?;
        *core.map_server.lock().unwrap_or_else(|p| p.into_inner()) = Some(server);
    }
    let status = live_status_of(core.inner());
    if open_browser {
        if let Some(url) = &status.url {
            app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())?;
        }
    }
    Ok(status)
}

#[tauri::command]
async fn livemap_stop(core: State<'_, AppCore>) -> Res<LiveStatus> {
    if let Some(s) = core.map_server.lock().unwrap_or_else(|p| p.into_inner()).take() {
        s.stop();
    }
    Ok(live_status_of(core.inner()))
}

#[tauri::command]
async fn livemap_status(core: State<'_, AppCore>) -> Res<LiveStatus> {
    Ok(live_status_of(core.inner()))
}

// ---------------------------------------------------------------- MCM / SaveGame settings

fn checked_save_path(path: &str) -> Result<PathBuf> {
    let dir = gvas::save_dir().ok_or_else(|| anyhow!("SaveGames-Ordner unbekannt"))?.canonicalize()?;
    let p = PathBuf::from(path).canonicalize().context("Datei nicht gefunden")?;
    if !p.starts_with(&dir) || util::ext_lower(path) != "sav" {
        bail!("Nur .sav-Dateien im SaveGames-Ordner können bearbeitet werden");
    }
    Ok(p)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveList {
    dir: Option<String>,
    files: Vec<gvas::SaveFileInfo>,
}

#[tauri::command]
async fn list_saves() -> Res<SaveList> {
    Ok(SaveList { dir: gvas::save_dir().map(|d| d.to_string_lossy().into_owned()), files: gvas::list() })
}

#[tauri::command]
async fn read_save(path: String) -> Res<gvas::SaveView> {
    tauri::async_runtime::spawn_blocking(move || -> Result<gvas::SaveView> { Ok(gvas::view(&gvas::load(&checked_save_path(&path)?)?)) })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn write_save(path: String, edits: Vec<gvas::Edit>) -> Res<gvas::SaveView> {
    tauri::async_runtime::spawn_blocking(move || -> Result<gvas::SaveView> {
        if game::is_running() {
            bail!("Bitte das Spiel schließen – es überschreibt die Einstellungen sonst beim Beenden.");
        }
        let p = checked_save_path(&path)?;
        gvas::apply_edits(&p, &edits)?;
        Ok(gvas::view(&gvas::load(&p)?))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn restore_save(path: String) -> Res<gvas::SaveView> {
    tauri::async_runtime::spawn_blocking(move || -> Result<gvas::SaveView> {
        let p = checked_save_path(&path)?;
        gvas::restore_backup(&p)?;
        Ok(gvas::view(&gvas::load(&p)?))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

// ---------------------------------------------------------------- key bindings

fn collect_keybinds(c: &Core) -> Result<Vec<keybinds::Binding>> {
    let mut out = keybinds::game_defaults();
    let profile = c.db.profile().clone();
    for pm in &profile.mods {
        let Some(entry) = c.db.find_mod(&pm.id) else { continue };
        let files_dir = plan::mod_files_dir(&c.paths.mods, &pm.id);
        for f in layout::resolve(&files_dir, entry) {
            let ext = util::ext_lower(&f.logical);
            if !matches!(ext.as_str(), "lua" | "ini" | "cfg" | "txt" | "json" | "toml") || util::file_meta(&f.src).0 > 1_000_000 {
                continue;
            }
            // UE4SS's own bundled mods are not interesting here.
            if entry.name.eq_ignore_ascii_case("ue4ss") && !f.logical.to_lowercase().contains("keybinds") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&f.src) {
                let rel = util::rel_path(&files_dir, &f.src);
                keybinds::scan_file(&text, &rel, &pm.id, &entry.name, pm.enabled, &mut out);
            }
        }
    }
    // Keys stored in MCM's settings save.
    for save in gvas::list().into_iter().filter(|s| s.is_mcm) {
        if let Ok(json) = gvas::load(Path::new(&save.path)) {
            for leaf in gvas::view(&json).leaves.into_iter().filter(|l| l.kind == "key") {
                out.push(keybinds::Binding {
                    id: format!("mcm:{}", leaf.pointer),
                    source: "mcm".into(),
                    mod_id: None,
                    mod_name: leaf.path.iter().find(|p| !p.starts_with('#') && p.len() > 2).cloned().unwrap_or_else(|| "MCM".into()),
                    enabled: true,
                    file: Some(save.path.clone()),
                    line: 0,
                    action: leaf.label.clone(),
                    key: leaf.value.as_str().unwrap_or("").to_string(),
                    modifiers: Vec::new(),
                    style: leaf.pointer.clone(),
                    editable: true,
                });
            }
        }
    }
    Ok(out)
}

#[tauri::command]
async fn get_keybinds(core: State<'_, AppCore>) -> Res<Vec<keybinds::Binding>> {
    with_core(&core, |c| collect_keybinds(c)).await
}

#[tauri::command]
async fn rebind_key(core: State<'_, AppCore>, binding: keybinds::Binding, key: String, modifiers: Vec<String>) -> Res<Vec<keybinds::Binding>> {
    with_core(&core, move |c| {
        if binding.source == "mcm" {
            if game::is_running() {
                bail!("Bitte das Spiel schließen, bevor MCM-Tasten geändert werden.");
            }
            let p = checked_save_path(binding.file.as_deref().unwrap_or(""))?;
            gvas::apply_edits(&p, &[gvas::Edit { pointer: binding.style.clone(), value: serde_json::Value::String(key) }])?;
        } else {
            let id = binding.mod_id.as_deref().ok_or_else(|| anyhow!("Nicht änderbar"))?;
            let rel = binding.file.as_deref().ok_or_else(|| anyhow!("Nicht änderbar"))?;
            let p = safe_mod_path(c, id, rel)?;
            keybinds::rebind(&p, &binding, &key, &modifiers)?;
        }
        collect_keybinds(c)
    })
    .await
}

// ---------------------------------------------------------------- setup

fn data_dir(app: &AppHandle) -> PathBuf {
    // Portable mode: a `portable.txt` next to the exe keeps all data beside it.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("portable.txt").exists() {
                return dir.join("S2MM-Daten");
            }
        }
    }
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("S2MM-Daten"))
}

fn forward_nxm(app: &AppHandle, args: &[String]) {
    if let Some(url) = args.iter().find(|a| a.to_lowercase().starts_with("nxm://")) {
        let core = app.state::<AppCore>().inner().clone();
        match nexus::parse_nxm(url) {
            Ok(link) if link.game == game::NEXUS_DOMAIN => start_download(app.clone(), core, link, None),
            Ok(link) => notice(app, "error", format!("Dieser Link gehört zu einem anderen Spiel ({}).", link.game)),
            Err(e) => notice(app, "error", format!("{e:#}")),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            forward_nxm(app, &argv);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = Arc::new(Paths::new(data_dir(app.handle())));
            std::fs::create_dir_all(&paths.data)?;
            let mut db: Db = read_json(&paths.state).unwrap_or_else(|e| {
                // Keep the damaged file for inspection instead of overwriting it silently.
                let _ = std::fs::copy(&paths.state, paths.data.join("state.damaged.json"));
                eprintln!("{e:#}");
                Db::default()
            });
            db.normalize();
            if db.settings.game_path.is_none() {
                if let Some(first) = game::detect_installs().into_iter().next() {
                    db.settings.game_path = Some(first.path);
                    db.settings.platform = first.platform;
                }
            }
            let deployment: Deployment = read_json(&paths.deployment).unwrap_or_default();
            let check_updates = db.settings.check_updates_on_start && db.settings.nexus_api_key.is_some();
            let core = Core { paths: paths.clone(), db, deployment, cache: PakCache::default(), nexus_user: None };
            core.save()?;
            let app_core = AppCore {
                core: Arc::new(Mutex::new(core)),
                paths,
                downloads: Arc::new(Mutex::new(Vec::new())),
                map_server: Arc::new(Mutex::new(None)),
            };
            app.manage(app_core.clone());

            let handle = app.handle().clone();
            let args: Vec<String> = std::env::args().collect();
            tauri::async_runtime::spawn(async move {
                forward_nxm(&handle, &args);
                if check_updates {
                    match check_updates_inner(&app_core).await {
                        Ok(n) if n > 0 => notice(&handle, "info", format!("{n} Mod-Update(s) auf Nexus verfügbar")),
                        Ok(_) => {}
                        Err(e) => notice(&handle, "error", format!("Update-Prüfung: {e:#}")),
                    }
                    let _ = handle.emit("state-changed", ());
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            detect_games,
            set_game_path,
            save_settings,
            launch_game,
            open_location,
            open_url,
            add_mods,
            reinstall_mod,
            remove_mod,
            set_enabled,
            reorder_mods,
            set_selection,
            update_mod,
            apply,
            purge,
            get_conflicts,
            mod_details,
            read_mod_file,
            write_mod_file,
            import_unmanaged,
            create_profile,
            rename_profile,
            delete_profile,
            switch_profile,
            get_tweaks,
            set_tweaks,
            restore_tweaks,
            export_engine_ini,
            import_engine_ini,
            ue4ss_install,
            set_tool_enabled,
            livemap_install_mod,
            livemap_start,
            livemap_stop,
            livemap_status,
            list_saves,
            read_save,
            write_save,
            restore_save,
            get_keybinds,
            rebind_key,
            nexus_login,
            nexus_logout,
            check_updates,
            nexus_update_mod,
            handle_nxm,
            get_downloads,
            install_download,
            remove_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Development helper: runs only the live-map server (see examples/mapserver.rs).
#[doc(hidden)]
pub fn dev_run_map_server(port: u16, cache: &Path) {
    let server = tauri::async_runtime::block_on(livemap::start(port, cache)).expect("server");
    println!("Live-Karte: http://127.0.0.1:{}/", server.port);
    std::thread::park();
}
