//! Persisted data model (state.json / deployment.json).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Steam,
    GamePass,
    Gog,
    Epic,
    #[default]
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LaunchMethod {
    #[default]
    Auto,
    Steam,
    Direct,
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub game_path: Option<String>,
    pub platform: Platform,
    /// Overrides the auto-detected `%LOCALAPPDATA%\Stalker2\Saved\Config\<Platform>` folder.
    pub config_dir: Option<String>,
    pub launch_method: LaunchMethod,
    pub launch_args: String,
    #[serde(default = "yes")]
    pub use_hardlinks: bool,
    /// Append `_P` to deployed pak names so they get patch priority over the base game.
    #[serde(default = "yes")]
    pub add_p_suffix: bool,
    pub nexus_api_key: Option<String>,
    pub nxm_handler: bool,
    /// The nxm:// command that was registered before we took over, restored on disable.
    pub nxm_previous: Option<String>,
    #[serde(default = "yes")]
    pub auto_install_downloads: bool,
    #[serde(default = "yes")]
    pub check_updates_on_start: bool,
    /// Library mod that holds UE4SS when installed through the manager.
    pub ue4ss_mod_id: Option<String>,
    /// Library mod with the live-map position writer (UE4SS Lua).
    pub livemap_mod_id: Option<String>,
    #[serde(default = "default_map_port")]
    pub map_port: u16,
    /// The welcome dialog (offering a vanilla reset) was shown.
    pub first_run_done: bool,
}

fn default_map_port() -> u16 {
    27420
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            game_path: None,
            platform: Platform::Unknown,
            config_dir: None,
            launch_method: LaunchMethod::Auto,
            launch_args: String::new(),
            use_hardlinks: true,
            add_p_suffix: true,
            nexus_api_key: None,
            nxm_handler: false,
            nxm_previous: None,
            auto_install_downloads: true,
            check_updates_on_start: true,
            ue4ss_mod_id: None,
            livemap_mod_id: None,
            map_port: default_map_port(),
            first_run_done: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NexusRef {
    pub game_domain: String,
    pub mod_id: u64,
    pub file_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub latest_version: Option<String>,
    pub latest_file_id: Option<u64>,
    pub checked_at: u64,
}

/// Maps a file or folder inside the staged mod (`src`, relative to the content root)
/// to a logical destination (`dest`) that is later classified into a game path.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileMap {
    pub src: String,
    pub dest: String,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GroupKind {
    ExactlyOne,
    AtMostOne,
    AtLeastOne,
    Any,
    All,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModOption {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Image path relative to the content root.
    pub image: Option<String>,
    #[serde(default)]
    pub files: Vec<FileMap>,
    #[serde(default)]
    pub default_selected: bool,
    /// FOMOD condition flags set when this option is chosen.
    #[serde(default)]
    pub flags: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OptionGroup {
    pub id: String,
    pub name: String,
    pub step: Option<String>,
    pub kind: GroupKind,
    pub options: Vec<ModOption>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Conditional {
    /// true = all dependencies must match, false = any.
    pub all: bool,
    pub flags: Vec<(String, String)>,
    pub files: Vec<FileMap>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModLayout {
    /// Content root relative to the mod's `files` folder ("" = the folder itself).
    pub root: String,
    pub fomod: bool,
    pub base: Vec<FileMap>,
    pub groups: Vec<OptionGroup>,
    #[serde(default)]
    pub conditionals: Vec<Conditional>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub notes: String,
    pub installed_at: u64,
    pub updated_at: u64,
    pub archive_name: Option<String>,
    pub nexus: Option<NexusRef>,
    pub picture_url: Option<String>,
    pub layout: ModLayout,
    /// group id -> selected option ids
    #[serde(default)]
    pub selection: BTreeMap<String, Vec<String>>,
    /// The mod has options the user has not confirmed yet.
    #[serde(default)]
    pub needs_setup: bool,
    #[serde(default)]
    pub update: Option<UpdateInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMod {
    pub id: String,
    pub enabled: bool,
}

/// A profile is an ordered list of mods; later entries win conflicts.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub mods: Vec<ProfileMod>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Db {
    pub version: u32,
    pub settings: Settings,
    pub mods: Vec<ModEntry>,
    pub profiles: Vec<Profile>,
    pub active_profile: String,
}

/// The built-in, read-only profile without any mods.
pub const VANILLA_ID: &str = "vanilla";

impl Default for Db {
    fn default() -> Self {
        Db {
            version: 1,
            settings: Settings::default(),
            mods: Vec::new(),
            profiles: vec![
                Profile { id: VANILLA_ID.into(), name: "Vanilla".into(), mods: Vec::new() },
                Profile { id: "default".into(), name: "Standard".into(), mods: Vec::new() },
            ],
            active_profile: "default".into(),
        }
    }
}

impl Db {
    pub fn profile(&self) -> &Profile {
        self.profiles
            .iter()
            .find(|p| p.id == self.active_profile)
            .unwrap_or(&self.profiles[0])
    }

    pub fn profile_mut(&mut self) -> &mut Profile {
        let idx = self
            .profiles
            .iter()
            .position(|p| p.id == self.active_profile)
            .unwrap_or(0);
        &mut self.profiles[idx]
    }

    pub fn find_mod(&self, id: &str) -> Option<&ModEntry> {
        self.mods.iter().find(|m| m.id == id)
    }

    pub fn find_mod_mut(&mut self, id: &str) -> Option<&mut ModEntry> {
        self.mods.iter_mut().find(|m| m.id == id)
    }

    pub fn vanilla_active(&self) -> bool {
        self.active_profile == VANILLA_ID
    }

    /// Switches away from the Vanilla profile (creating "Standard" if needed).
    pub fn leave_vanilla(&mut self) {
        if !self.vanilla_active() {
            return;
        }
        if !self.profiles.iter().any(|p| p.id != VANILLA_ID) {
            self.profiles.push(Profile { id: "default".into(), name: "Standard".into(), mods: Vec::new() });
            self.normalize();
        }
        self.active_profile = self.profiles.iter().find(|p| p.id != VANILLA_ID).unwrap().id.clone();
    }

    /// Makes sure every profile references every mod exactly once and the read-only
    /// Vanilla profile exists (first, with everything disabled).
    pub fn normalize(&mut self) {
        if self.profiles.is_empty() {
            *self = Db {
                mods: std::mem::take(&mut self.mods),
                settings: self.settings.clone(),
                ..Db::default()
            };
        }
        match self.profiles.iter().position(|p| p.id == VANILLA_ID) {
            Some(0) => {}
            Some(i) => {
                let v = self.profiles.remove(i);
                self.profiles.insert(0, v);
            }
            None => self.profiles.insert(0, Profile { id: VANILLA_ID.into(), name: "Vanilla".into(), mods: Vec::new() }),
        }
        self.profiles[0].name = "Vanilla".into();
        if !self.profiles.iter().any(|p| p.id == self.active_profile) {
            self.active_profile = self.profiles[0].id.clone();
        }
        let ids: Vec<String> = self.mods.iter().map(|m| m.id.clone()).collect();
        for p in &mut self.profiles {
            let mut seen = std::collections::HashSet::new();
            p.mods.retain(|e| ids.contains(&e.id) && seen.insert(e.id.clone()));
            for id in &ids {
                if !p.mods.iter().any(|e| &e.id == id) {
                    p.mods.push(ProfileMod { id: id.clone(), enabled: false });
                }
            }
        }
        for m in &mut self.profiles[0].mods {
            m.enabled = false;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeployedFile {
    /// Game-relative path with forward slashes.
    pub target: String,
    pub source: String,
    pub mod_id: String,
    pub size: u64,
    pub mtime: u64,
    /// Where the original game file was moved before we overwrote it.
    pub backup: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Deployment {
    pub game_path: String,
    pub fingerprint: String,
    pub applied_at: u64,
    pub game_build: Option<String>,
    pub files: Vec<DeployedFile>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_old_state_and_guards_vanilla() {
        let mut db: Db = serde_json::from_str(
            r#"{"profiles":[{"id":"default","name":"Standard","mods":[{"id":"a","enabled":true}]}],"activeProfile":"default","mods":[]}"#,
        )
        .unwrap();
        db.mods.push(serde_json::from_value(serde_json::json!({
            "id":"a","name":"A","version":null,"author":null,"summary":null,"installedAt":0,"updatedAt":0,
            "archiveName":null,"nexus":null,"pictureUrl":null,"layout":{"root":"","fomod":false,"base":[],"groups":[]}
        })).unwrap());
        db.normalize();
        assert_eq!(db.profiles[0].id, VANILLA_ID);
        assert!(db.profiles[0].mods.iter().all(|m| !m.enabled));
        assert!(db.profiles[1].mods[0].enabled, "existing profile keeps its choices");
        assert!(!db.settings.first_run_done);

        db.active_profile = VANILLA_ID.into();
        db.leave_vanilla();
        assert_eq!(db.active_profile, "default");
    }
}
