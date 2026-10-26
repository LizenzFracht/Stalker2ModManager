//! Understanding a staged mod: finding its content root and options (FOMOD or folder
//! heuristics), resolving the selected files and mapping them into the game folder.

use crate::fomod;
use crate::game::GameInfo;
use crate::model::{FileMap, GroupKind, ModEntry, ModLayout, ModOption, OptionGroup};
use crate::util::{ext_lower, join_logical, rel_path, resolve_ci};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

const STRUCTURAL: [&str; 11] = [
    "stalker2", "content", "paks", "~mods", "binaries", "win64", "wingdk", "ue4ss", "logicmods", "plugins", "windows",
];
const PAYLOAD_EXTS: [&str; 7] = ["pak", "utoc", "ucas", "dll", "asi", "lua", "bk2"];
const DOC_EXTS: [&str; 14] = [
    "txt", "md", "pdf", "jpg", "jpeg", "png", "gif", "webp", "url", "html", "htm", "rtf", "doc", "docx",
];
const JUNK: [&str; 3] = ["__macosx", ".ds_store", "thumbs.db"];

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    Pak,
    /// Official Zone Kit layout: Stalker2/Mods/<Name>/Content/Paks/Windows/*.pak
    ZoneKit,
    Ue4ss,
    Binary,
    GameFile,
}

pub struct Analysis {
    pub layout: ModLayout,
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
}

fn is_junk(name: &str) -> bool {
    JUNK.contains(&name.to_lowercase().as_str())
}

fn is_ue4ss_mod(dir: &Path) -> bool {
    resolve_ci(dir, "Scripts/main.lua").map(|p| p.is_file()).unwrap_or(false)
}

fn is_payload_file(name: &str) -> bool {
    PAYLOAD_EXTS.contains(&ext_lower(name).as_str())
}

fn payload_names(dir: &Path) -> Vec<String> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.file_name().to_string_lossy().to_lowercase())
        .filter(|n| is_payload_file(n))
        .collect()
}

fn has_payload(dir: &Path) -> bool {
    walkdir::WalkDir::new(dir).into_iter().flatten().any(|e| {
        let n = e.file_name().to_string_lossy().to_lowercase();
        (e.file_type().is_file() && is_payload_file(&n)) || (e.file_type().is_dir() && STRUCTURAL.contains(&n.as_str()))
    })
}

fn entries(dir: &Path) -> Vec<(String, PathBuf, bool)> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path(), e.path().is_dir()))
                .filter(|(n, _, _)| !is_junk(n))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    v
}

fn find_fomod(files: &Path) -> Option<PathBuf> {
    walkdir::WalkDir::new(files)
        .max_depth(4)
        .into_iter()
        .flatten()
        .find(|e| {
            e.file_type().is_file()
                && e.file_name().to_string_lossy().eq_ignore_ascii_case("ModuleConfig.xml")
                && e.path()
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().eq_ignore_ascii_case("fomod"))
                    .unwrap_or(false)
        })
        .map(|e| e.into_path())
}

/// Builds options from sub folders: every folder with payload becomes a choice.
fn option_group(id: &str, name: &str, root: &Path, dirs: &[(String, PathBuf)], base_has_payload: bool) -> OptionGroup {
    let sets: Vec<HashSet<String>> = dirs.iter().map(|(_, p)| payload_names(p).into_iter().collect()).collect();
    let overlapping = sets.iter().enumerate().any(|(i, a)| sets.iter().skip(i + 1).any(|b| !a.is_disjoint(b)));
    let kind = if overlapping { GroupKind::ExactlyOne } else { GroupKind::Any };
    let options = dirs
        .iter()
        .enumerate()
        .map(|(i, (n, p))| ModOption {
            id: format!("{id}o{i}"),
            name: n.clone(),
            description: String::new(),
            image: None,
            files: vec![FileMap { src: rel_path(root, p), dest: String::new(), priority: 0 }],
            default_selected: i == 0 && (overlapping || !base_has_payload),
            flags: Vec::new(),
        })
        .collect();
    OptionGroup { id: id.into(), name: name.into(), step: None, kind, options }
}

pub fn analyze(files: &Path) -> Analysis {
    if let Some(xml) = find_fomod(files) {
        if let Ok(f) = fomod::parse(&xml) {
            let mut layout = f.layout;
            let content_root = xml.parent().and_then(|p| p.parent()).unwrap_or(files);
            layout.root = rel_path(files, content_root);
            return Analysis { layout, name: f.name, author: f.author, version: f.version };
        }
    }

    // Unwrap single wrapper folders ("ModName/ModName v1.2/...").
    let mut root = files.to_path_buf();
    loop {
        let list = entries(&root);
        let non_doc: Vec<_> = list
            .iter()
            .filter(|(n, _, dir)| *dir || !DOC_EXTS.contains(&ext_lower(n).as_str()))
            .collect();
        match non_doc.as_slice() {
            [(name, path, true)] if !STRUCTURAL.contains(&name.to_lowercase().as_str()) && !is_ue4ss_mod(path) => {
                root = path.clone();
            }
            _ => break,
        }
    }

    let mut base = Vec::new();
    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    for (name, path, is_dir) in entries(&root) {
        let lower = name.to_lowercase();
        if !is_dir {
            if is_payload_file(&name) {
                base.push(FileMap { src: name.clone(), dest: name, priority: 0 });
            }
        } else if STRUCTURAL.contains(&lower.as_str()) || is_ue4ss_mod(&path) || lower == "mods" {
            base.push(FileMap { src: name.clone(), dest: name, priority: 0 });
        } else if has_payload(&path) {
            candidates.push((name, path));
        }
    }

    let mut layout = ModLayout { root: rel_path(files, &root), ..Default::default() };
    if candidates.is_empty() {
        layout.base = vec![FileMap { src: String::new(), dest: String::new(), priority: 0 }];
        return Analysis { layout, name: None, author: None, version: None };
    }
    if candidates.len() == 1 && base.is_empty() {
        let (_, p) = &candidates[0];
        base.push(FileMap { src: rel_path(&root, p), dest: String::new(), priority: 0 });
        layout.base = base;
        return Analysis { layout, name: None, author: None, version: None };
    }

    let base_has_payload = !base.is_empty();
    let mut rest = Vec::new();
    for (name, path) in candidates {
        // "Health/x2, Health/x5" → its own group.
        let direct_payload = entries(&path).iter().any(|(n, _, d)| !d && is_payload_file(n));
        let subs: Vec<(String, PathBuf)> = entries(&path)
            .into_iter()
            .filter(|(n, p, d)| *d && has_payload(p) && !STRUCTURAL.contains(&n.to_lowercase().as_str()) && !is_ue4ss_mod(p))
            .map(|(n, p, _)| (n, p))
            .collect();
        if !direct_payload && subs.len() >= 2 {
            let id = format!("g{}", layout.groups.len());
            layout.groups.push(option_group(&id, &name, &root, &subs, false));
        } else {
            rest.push((name, path));
        }
    }
    if !rest.is_empty() {
        let id = format!("g{}", layout.groups.len());
        let title = if base_has_payload { "Optionale Dateien" } else { "Variante" };
        layout.groups.insert(0, option_group(&id, title, &root, &rest, base_has_payload));
    }
    layout.base = base;
    Analysis { layout, name: None, author: None, version: None }
}

/// The effective selection: the user's choice or the defaults of each group.
pub fn effective_selection(entry: &ModEntry) -> BTreeMap<String, Vec<String>> {
    let mut sel = BTreeMap::new();
    for g in &entry.layout.groups {
        let chosen: Vec<String> = match entry.selection.get(&g.id) {
            Some(s) => s.iter().filter(|id| g.options.iter().any(|o| &o.id == *id)).cloned().collect(),
            None => {
                let mut d: Vec<String> = g.options.iter().filter(|o| o.default_selected).map(|o| o.id.clone()).collect();
                if d.is_empty() && matches!(g.kind, GroupKind::ExactlyOne | GroupKind::AtLeastOne) {
                    d.push(g.options[0].id.clone());
                }
                d
            }
        };
        let chosen = if g.kind == GroupKind::All { g.options.iter().map(|o| o.id.clone()).collect() } else { chosen };
        sel.insert(g.id.clone(), chosen);
    }
    sel
}

#[derive(Clone, Debug)]
pub struct ResolvedFile {
    pub src: PathBuf,
    pub logical: String,
}

/// All files the mod installs with its current option selection.
pub fn resolve(files_dir: &Path, entry: &ModEntry) -> Vec<ResolvedFile> {
    let root = crate::util::join_rel(files_dir, &entry.layout.root);
    let sel = effective_selection(entry);
    let mut maps: Vec<FileMap> = entry.layout.base.clone();
    let mut flags: HashMap<String, String> = HashMap::new();
    for g in &entry.layout.groups {
        let chosen = sel.get(&g.id).cloned().unwrap_or_default();
        for o in g.options.iter().filter(|o| chosen.contains(&o.id)) {
            maps.extend(o.files.iter().cloned());
            for (k, v) in &o.flags {
                flags.insert(k.clone(), v.clone());
            }
        }
    }
    for c in &entry.layout.conditionals {
        let check = |(k, v): &(String, String)| flags.get(k).map(|x| x == v).unwrap_or(v.is_empty());
        let ok = if c.all { c.flags.iter().all(check) } else { c.flags.iter().any(check) };
        if ok {
            maps.extend(c.files.iter().cloned());
        }
    }
    maps.sort_by_key(|m| m.priority);

    let mut out: Vec<ResolvedFile> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut push = |src: PathBuf, logical: String| {
        let key = logical.to_lowercase();
        match index.get(&key) {
            Some(&i) => out[i] = ResolvedFile { src, logical },
            None => {
                index.insert(key, out.len());
                out.push(ResolvedFile { src, logical });
            }
        }
    };
    for m in maps {
        let Some(p) = resolve_ci(&root, &m.src) else { continue };
        if p.is_file() {
            let dest = if m.dest.is_empty() {
                p.file_name().unwrap().to_string_lossy().into_owned()
            } else {
                m.dest.clone()
            };
            push(p, dest);
        } else if p.is_dir() {
            for e in walkdir::WalkDir::new(&p).into_iter().flatten() {
                if !e.file_type().is_file() {
                    continue;
                }
                let rel = rel_path(&p, e.path());
                let lower = rel.to_lowercase();
                if lower.split('/').any(|s| s == "fomod" || is_junk(s)) {
                    continue;
                }
                push(e.path().to_path_buf(), join_logical(&m.dest, &rel));
            }
        }
    }
    out
}

pub struct Classifier<'a> {
    game: &'a GameInfo,
    /// (lowercase logical prefix, folder name) of UE4SS mods inside this mod.
    ue4ss_roots: Vec<(String, String)>,
}

impl<'a> Classifier<'a> {
    pub fn new(game: &'a GameInfo, files: &[ResolvedFile], mod_name: &str) -> Self {
        let mut ue4ss_roots = Vec::new();
        for f in files {
            let lower = f.logical.to_lowercase();
            if let Some(prefix) = lower.strip_suffix("scripts/main.lua") {
                let prefix = prefix.trim_end_matches('/').to_string();
                let original = &f.logical[..prefix.len()];
                let name = original
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| mod_name.chars().filter(|c| c.is_ascii_alphanumeric()).collect());
                ue4ss_roots.push((prefix, name));
            }
        }
        Classifier { game, ue4ss_roots }
    }

    fn binaries(&self, rest: &[&str]) -> String {
        join_logical(&self.game.binaries_dir, &rest.join("/"))
    }

    /// Maps a logical path to a game-relative target. Pak files return just their file
    /// name; the planner renames them into the ~mods folder.
    pub fn classify(&self, logical: &str) -> Option<(String, TargetKind)> {
        let segs: Vec<&str> = logical.split('/').filter(|s| !s.is_empty()).collect();
        let lower: Vec<String> = segs.iter().map(|s| s.to_lowercase()).collect();
        let file = *segs.last()?;
        let ext = ext_lower(file);
        let pakish = crate::archive::PAK_EXTS.contains(&ext.as_str());

        // Zone Kit mods keep their own folder below Stalker2/Mods.
        if let Some(i) = (0..lower.len().saturating_sub(2)).find(|&i| lower[i] == "stalker2" && lower[i + 1] == "mods") {
            return Some((format!("Stalker2/Mods/{}", segs[i + 2..].join("/")), TargetKind::ZoneKit));
        }
        if let Some(i) = (1..lower.len().saturating_sub(3)).find(|&i| lower[i] == "content" && lower[i + 1] == "paks" && lower[i + 2] == "windows") {
            if pakish && lower[i - 1] != "stalker2" {
                return Some((format!("Stalker2/Mods/{}/Content/Paks/Windows/{}", segs[i - 1], file), TargetKind::ZoneKit));
            }
        }
        if let Some(i) = lower.iter().position(|s| s == "stalker2") {
            if let Some(next) = lower.get(i + 1) {
                if pakish {
                    return Some((file.to_string(), TargetKind::Pak));
                }
                if next == "binaries" && segs.len() > i + 3 {
                    return Some((self.binaries(&segs[i + 3..]), TargetKind::Binary));
                }
                if next == "content" || next == "plugins" {
                    return Some((format!("Stalker2/{}", segs[i + 1..].join("/")), TargetKind::GameFile));
                }
            }
        }
        if pakish {
            return Some((file.to_string(), TargetKind::Pak));
        }
        if let Some(i) = lower[..lower.len() - 1].iter().rposition(|s| s == "ue4ss") {
            return Some((self.binaries(&segs[i..]), TargetKind::Binary));
        }
        for (prefix, name) in &self.ue4ss_roots {
            let n = prefix.split('/').filter(|s| !s.is_empty()).count();
            if segs.len() > n && lower[..n].join("/") == *prefix {
                let target = join_logical(&join_logical(&self.game.ue4ss_mods_dir, name), &segs[n..].join("/"));
                return Some((target, TargetKind::Ue4ss));
            }
        }
        if DOC_EXTS.contains(&ext.as_str()) {
            return None;
        }
        if let Some(i) = lower.iter().position(|s| s == "win64" || s == "wingdk") {
            return Some((self.binaries(&segs[i + 1..]), TargetKind::Binary));
        }
        if ext == "dll" || ext == "asi" {
            return Some((self.binaries(&[file]), TargetKind::Binary));
        }
        if let Some(i) = lower.iter().position(|s| s == "content") {
            if segs.len() > i + 1 {
                return Some((format!("Stalker2/{}", segs[i..].join("/")), TargetKind::GameFile));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Platform;

    fn game() -> GameInfo {
        GameInfo {
            path: "C:/Game".into(),
            platform: Platform::Steam,
            valid: true,
            binaries_dir: "Stalker2/Binaries/Win64".into(),
            mods_dir: "Stalker2/Content/Paks/~mods".into(),
            ue4ss_mods_dir: "Stalker2/Binaries/Win64/ue4ss/Mods".into(),
            ue4ss_installed: true,
            exe: None,
            config_dir: None,
            build: None,
        }
    }

    fn rf(l: &str) -> ResolvedFile {
        ResolvedFile { src: PathBuf::new(), logical: l.into() }
    }

    #[test]
    fn classifies_common_layouts() {
        let g = game();
        let files = vec![rf("MyLuaMod/Scripts/main.lua"), rf("MyLuaMod/enabled.txt")];
        let c = Classifier::new(&g, &files, "x");
        assert_eq!(c.classify("Stalker2/Content/Paks/~mods/Foo_P.pak").unwrap(), ("Foo_P.pak".into(), TargetKind::Pak));
        assert_eq!(c.classify("Foo.utoc").unwrap().1, TargetKind::Pak);
        assert_eq!(c.classify("readme.txt"), None);
        assert_eq!(
            c.classify("MyLuaMod/enabled.txt").unwrap().0,
            "Stalker2/Binaries/Win64/ue4ss/Mods/MyLuaMod/enabled.txt"
        );
        assert_eq!(
            c.classify("Stalker2/Binaries/WinGDK/dwmapi.dll").unwrap().0,
            "Stalker2/Binaries/Win64/dwmapi.dll"
        );
        assert_eq!(
            c.classify("ue4ss/UE4SS-settings.ini").unwrap().0,
            "Stalker2/Binaries/Win64/ue4ss/UE4SS-settings.ini"
        );
        assert_eq!(c.classify("plugin.asi").unwrap().0, "Stalker2/Binaries/Win64/plugin.asi");
        assert_eq!(
            c.classify("ModConfigurationMenu/Windows/NewContent/Windows/Stalker2/Mods/ModConfigurationMenu/Content/Paks/Windows/MCM-NewContent.pak").unwrap(),
            ("Stalker2/Mods/ModConfigurationMenu/Content/Paks/Windows/MCM-NewContent.pak".into(), TargetKind::ZoneKit)
        );
        assert_eq!(
            c.classify("MyKitMod/Content/Paks/Windows/MyKitMod.utoc").unwrap().0,
            "Stalker2/Mods/MyKitMod/Content/Paks/Windows/MyKitMod.utoc"
        );
    }

    #[test]
    fn detects_variant_folders() {
        let tmp = std::env::temp_dir().join(format!("s2mm-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        for v in ["x2", "x5"] {
            let d = tmp.join("Wrapper").join(v);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("Stamina_P.pak"), b"x").unwrap();
        }
        std::fs::write(tmp.join("Wrapper/readme.txt"), b"x").unwrap();
        let a = analyze(&tmp);
        assert_eq!(a.layout.root, "Wrapper");
        assert_eq!(a.layout.groups.len(), 1);
        assert_eq!(a.layout.groups[0].kind, GroupKind::ExactlyOne);
        assert_eq!(a.layout.groups[0].options.len(), 2);
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
