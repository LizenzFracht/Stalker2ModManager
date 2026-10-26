//! End-to-end test against a fake game folder: stage archives, plan, detect conflicts,
//! deploy, change options, undeploy.

use crate::install::{stage, InstallMeta};
use crate::model::{Db, Deployment, GroupKind, Platform, ProfileMod, Settings};
use crate::plan::{self, PakCache};
use crate::{deploy, game};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Writes a minimal pak v11 whose full directory index lists `assets`.
fn fake_pak(assets: &[&str]) -> Vec<u8> {
    fn fstring(out: &mut Vec<u8>, s: &str) {
        out.extend(((s.len() + 1) as i32).to_le_bytes());
        out.extend(s.as_bytes());
        out.push(0);
    }
    let mut by_dir: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for a in assets {
        let (d, f) = a.rsplit_once('/').unwrap();
        by_dir.entry(format!("{d}/")).or_default().push(f.to_string());
    }
    let mut fdi = Vec::new();
    fdi.extend((by_dir.len() as u32).to_le_bytes());
    for (d, files) in &by_dir {
        fstring(&mut fdi, d);
        fdi.extend((files.len() as u32).to_le_bytes());
        for f in files {
            fstring(&mut fdi, f);
            fdi.extend(0u32.to_le_bytes());
        }
    }
    let mut out = vec![0u8; 16]; // pretend data
    let fdi_offset = out.len() as u64;
    out.extend(&fdi);
    let index_offset = out.len() as u64;
    let mut index = Vec::new();
    fstring(&mut index, "../../../");
    index.extend((assets.len() as u32).to_le_bytes());
    index.extend(0u64.to_le_bytes());
    index.extend(0u32.to_le_bytes()); // no path hash index
    index.extend(1u32.to_le_bytes()); // full directory index
    index.extend(fdi_offset.to_le_bytes());
    index.extend((fdi.len() as u64).to_le_bytes());
    index.extend([0u8; 20]);
    index.extend(0u32.to_le_bytes());
    out.extend(&index);
    out.extend([0u8; 16]);
    out.push(0);
    out.extend(0x5A6F_12E1u32.to_le_bytes());
    out.extend(11u32.to_le_bytes());
    out.extend(index_offset.to_le_bytes());
    out.extend((index.len() as u64).to_le_bytes());
    out.extend([0u8; 20]);
    out.extend([0u8; 160]);
    out
}

fn make_zip(path: &Path, files: &[(&str, Vec<u8>)]) {
    let f = std::fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    for (name, data) in files {
        z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap();
}

const CFG: &str = "Stalker2/Content/GameLite/GameData";

#[test]
fn full_cycle() {
    let tmp: PathBuf = std::env::temp_dir().join(format!("s2mm-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let game_root = tmp.join("game");
    std::fs::create_dir_all(game_root.join("Stalker2/Content/Paks")).unwrap();
    std::fs::create_dir_all(game_root.join("Stalker2/Binaries/Win64/ue4ss/Mods")).unwrap();
    std::fs::write(game_root.join("Stalker2/Binaries/Win64/dwmapi.dll"), b"ORIGINAL").unwrap();
    let mods_root = tmp.join("data/mods");
    let backups = tmp.join("data/backups");
    std::fs::create_dir_all(&mods_root).unwrap();
    let src = tmp.join("src");
    std::fs::create_dir_all(&src).unwrap();

    // A: plain pak with Nexus-style file name
    let a = src.join("Better Stamina-123-1-0-1732400000.zip");
    make_zip(&a, &[
        ("BetterStamina_P.pak", fake_pak(&[&format!("{CFG}/Stamina.cfg"), &format!("{CFG}/ObjPrototypes.cfg")])),
        ("readme.txt", b"hi".to_vec()),
    ]);
    // B: variants x2/x5 inside a wrapper folder
    let b = src.join("Weight Mod.zip");
    let wpak = fake_pak(&[&format!("{CFG}/ObjPrototypes.cfg"), &format!("{CFG}/Weight.cfg")]);
    make_zip(&b, &[
        ("Weight Mod/x2/WeightMod.pak", wpak.clone()),
        ("Weight Mod/x5/WeightMod.pak", wpak),
        ("Weight Mod/README.md", b"choose one".to_vec()),
    ]);
    // C: FOMOD with two groups and a flag-based conditional
    let c = src.join("Fomod Mod.zip");
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<config><moduleName>Fancy Fomod</moduleName>
<requiredInstallFiles><file source="core\Core_P.pak" destination="" /></requiredInstallFiles>
<installSteps order="Explicit"><installStep name="Main">
<optionalFileGroups order="Explicit">
 <group name="HUD" type="SelectExactlyOne"><plugins order="Explicit">
  <plugin name="Minimal"><description>Less HUD</description><image path="img\min.png"/><conditionFlags><flag name="hud">min</flag></conditionFlags>
   <files><folder source="hud_min" destination="" /></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
  <plugin name="Full"><description>All HUD</description><files><folder source="hud_full" destination="" /></files><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
 </plugins></group>
</optionalFileGroups></installStep></installSteps>
<conditionalFileInstalls><patterns><pattern><dependencies operator="And"><flagDependency flag="hud" value="min"/></dependencies>
 <files><file source="extra\Extra_P.pak" destination="" /></files></pattern></patterns></conditionalFileInstalls>
</config>"#;
    make_zip(&c, &[
        ("FancyFomod/fomod/ModuleConfig.xml", xml.as_bytes().to_vec()),
        ("FancyFomod/core/Core_P.pak", fake_pak(&["Stalker2/Content/UI/core.uasset"])),
        ("FancyFomod/hud_min/Hud_P.pak", fake_pak(&["Stalker2/Content/UI/hud.uasset"])),
        ("FancyFomod/hud_full/Hud_P.pak", fake_pak(&["Stalker2/Content/UI/hud.uasset"])),
        ("FancyFomod/extra/Extra_P.pak", fake_pak(&["Stalker2/Content/UI/extra.uasset"])),
        ("FancyFomod/img/min.png", vec![0u8; 4]),
    ]);
    // D: UE4SS Lua mod + a proxy dll that replaces a game file
    let d = src.join("Lua Tweaks.zip");
    make_zip(&d, &[
        ("LuaTweaks/Scripts/main.lua", b"print('hi')".to_vec()),
        ("LuaTweaks/config.lua", b"return { speed = 2 }".to_vec()),
        ("dwmapi.dll", b"PROXY".to_vec()),
    ]);

    let mut db = Db { settings: Settings { game_path: Some(game_root.to_string_lossy().into()), platform: Platform::Steam, ..Settings::default() }, ..Db::default() };
    for p in [&a, &b, &c, &d] {
        let e = stage(&mods_root, p, InstallMeta::default(), None).unwrap();
        db.profile_mut().mods.push(ProfileMod { id: e.id.clone(), enabled: true });
        db.mods.push(e);
    }
    let (ma, mb, mc) = (db.mods[0].clone(), db.mods[1].clone(), db.mods[2].clone());

    // Analysis
    assert_eq!(ma.name, "Better Stamina");
    assert_eq!(ma.version.as_deref(), Some("1.0"));
    assert_eq!(ma.nexus.as_ref().unwrap().mod_id, 123);
    assert_eq!(mb.layout.groups.len(), 1);
    assert_eq!(mb.layout.groups[0].kind, GroupKind::ExactlyOne);
    assert!(mb.needs_setup);
    assert_eq!(mc.name, "Fancy Fomod");
    assert!(mc.layout.fomod);
    assert_eq!(mc.layout.groups[0].options.len(), 2);
    assert_eq!(mc.layout.groups[0].options[0].image.as_deref(), Some("img/min.png"));

    let g = game::inspect(&game_root, &db.settings);
    assert!(g.valid);
    let p = plan::build(&db, &g, &mods_root);
    let targets: Vec<&str> = p.files.iter().map(|f| f.target.as_str()).collect();
    assert!(targets.contains(&"Stalker2/Content/Paks/~mods/001_better-stamina_BetterStamina_P.pak"), "{targets:?}");
    assert!(targets.contains(&"Stalker2/Content/Paks/~mods/002_weight-mod_WeightMod_P.pak"), "{targets:?}");
    // FOMOD default = "Full" (Recommended) → no conditional Extra pak.
    assert!(targets.iter().any(|t| t.ends_with("003_fancy-fomod_Hud_P.pak")));
    assert!(!targets.iter().any(|t| t.contains("Extra_P")));
    assert!(targets.contains(&"Stalker2/Binaries/Win64/ue4ss/Mods/LuaTweaks/Scripts/main.lua"), "{targets:?}");
    assert!(targets.contains(&"Stalker2/Binaries/Win64/ue4ss/Mods/LuaTweaks/enabled.txt"), "{targets:?}");
    assert!(targets.contains(&"Stalker2/Binaries/Win64/dwmapi.dll"));

    // Conflicts: ObjPrototypes.cfg in A and B → B (later) wins.
    let report = plan::conflicts(&p, &mut PakCache::default());
    assert_eq!(report.pairs.len(), 1, "{:?}", report.pairs);
    assert_eq!(report.pairs[0].winner, mb.id);
    assert_eq!(report.pairs[0].loser, ma.id);
    assert_eq!(report.pairs[0].items, vec![format!("{CFG}/ObjPrototypes.cfg")]);

    // Deploy
    let r = deploy::apply(&g, &p, &Deployment::default(), &backups, true).unwrap();
    let dep = r.deployment;
    assert_eq!(r.placed, p.files.len());
    assert_eq!(std::fs::read(game_root.join("Stalker2/Binaries/Win64/dwmapi.dll")).unwrap(), b"PROXY");
    assert!(dep.files.iter().any(|f| f.backup.is_some()));
    assert!(game_root.join("Stalker2/Content/Paks/~mods/002_weight-mod_WeightMod_P.pak").exists());
    assert!(deploy::unmanaged(&g, &dep).is_empty());

    // Re-apply unchanged → everything kept
    let r2 = deploy::apply(&g, &p, &dep, &backups, true).unwrap();
    assert_eq!(r2.kept, p.files.len());
    let dep = r2.deployment;

    // Switch FOMOD to "Minimal" → conditional Extra pak appears; disable D → original dll back.
    let gid = mc.layout.groups[0].id.clone();
    let min = mc.layout.groups[0].options[0].id.clone();
    db.find_mod_mut(&mc.id).unwrap().selection.insert(gid, vec![min]);
    db.profile_mut().mods[3].enabled = false;
    let p2 = plan::build(&db, &g, &mods_root);
    assert!(p2.files.iter().any(|f| f.target.contains("Extra_P")));
    let r3 = deploy::apply(&g, &p2, &dep, &backups, true).unwrap();
    assert_eq!(std::fs::read(game_root.join("Stalker2/Binaries/Win64/dwmapi.dll")).unwrap(), b"ORIGINAL");
    assert!(!game_root.join("Stalker2/Binaries/Win64/ue4ss/Mods/LuaTweaks").exists());
    assert!(game_root.join("Stalker2/Binaries/Win64/ue4ss/Mods").exists());

    // A foreign pak dropped in ~mods is reported as unmanaged and survives purge.
    std::fs::write(game_root.join("Stalker2/Content/Paks/~mods/Manual_P.pak"), fake_pak(&["x/y.cfg"])).unwrap();
    assert_eq!(deploy::unmanaged(&g, &r3.deployment), vec!["Stalker2/Content/Paks/~mods/Manual_P.pak".to_string()]);
    deploy::purge(&g, &r3.deployment).unwrap();
    let left: Vec<_> = std::fs::read_dir(game_root.join("Stalker2/Content/Paks/~mods")).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(left, vec![std::ffi::OsString::from("Manual_P.pak")]);
    assert_eq!(std::fs::read(game_root.join("Stalker2/Binaries/Win64/dwmapi.dll")).unwrap(), b"ORIGINAL");

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn stages_real_mcm_archive() {
    // Point S2MM_TEST_MCM_ARCHIVE at a downloaded "Mod Configuration Menu" archive to run this.
    let Some(archive) = std::env::var_os("S2MM_TEST_MCM_ARCHIVE").map(PathBuf::from) else {
        return;
    };
    let archive = archive.as_path();
    let tmp = std::env::temp_dir().join(format!("s2mm-mcm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("game/Stalker2/Content/Paks")).unwrap();
    let mods_root = tmp.join("mods");
    std::fs::create_dir_all(&mods_root).unwrap();
    let e = stage(&mods_root, archive, InstallMeta::default(), None).unwrap();
    assert!(e.layout.groups.is_empty(), "{:?}", e.layout.groups);
    let mut db = Db::default();
    db.profile_mut().mods.push(ProfileMod { id: e.id.clone(), enabled: true });
    db.mods.push(e);
    let g = game::inspect(&tmp.join("game"), &Settings::default());
    let p = plan::build(&db, &g, &mods_root);
    let targets: Vec<&str> = p.files.iter().map(|f| f.target.as_str()).collect();
    assert_eq!(p.files.len(), 6, "{targets:?}");
    assert!(targets.iter().all(|t| t.starts_with("Stalker2/Mods/ModConfigurationMenu/Content/Paks/Windows/")), "{targets:?}");
    let report = plan::conflicts(&p, &mut PakCache::default());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
#[ignore = "network"]
fn ue4ss_download_and_plan() {
    let tmp = std::env::temp_dir().join(format!("s2mm-ue4ss-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("game/Stalker2/Content/Paks")).unwrap();
    std::fs::create_dir_all(tmp.join("game/Stalker2/Binaries/Win64")).unwrap();
    let prepared = tauri::async_runtime::block_on(crate::ue4ss::prepare(&tmp.join("work"), Platform::Steam)).unwrap();
    let mods_root = tmp.join("mods");
    let e = stage(&mods_root, &prepared.folder, InstallMeta::default(), None).unwrap();
    let mut db = Db::default();
    db.profile_mut().mods.push(ProfileMod { id: e.id.clone(), enabled: true });
    db.mods.push(e);
    let g = game::inspect(&tmp.join("game"), &Settings::default());
    let p = plan::build(&db, &g, &mods_root);
    let t: Vec<&str> = p.files.iter().map(|f| f.target.as_str()).collect();
    assert!(t.contains(&"Stalker2/Binaries/Win64/dwmapi.dll"), "{t:?}");
    assert!(t.contains(&"Stalker2/Binaries/Win64/ue4ss/UE4SS.dll"), "{t:?}");
    assert!(t.contains(&"Stalker2/Binaries/Win64/ue4ss/VTableLayout.ini"), "Stalker2 config missing: {t:?}");
    assert!(t.iter().any(|x| x.contains("ue4ss/UE4SS_Signatures/")), "{t:?}");
    assert!(!t.iter().any(|x| x.ends_with("/enabled.txt")), "bundled mods must stay controlled by mods.txt: {t:?}");
    println!("UE4SS {} → {} Dateien", prepared.version, t.len());
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
#[ignore = "network"]
fn livemap_server_proxies_and_injects() {
    use std::io::{Read, Write as _};
    let tmp = std::env::temp_dir().join(format!("s2mm-map-{}", std::process::id()));
    let server = tauri::async_runtime::block_on(crate::livemap::start(27999, &tmp)).unwrap();
    let get = |path: &str| {
        let mut s = std::net::TcpStream::connect("127.0.0.1:27999").unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).into_owned()
    };
    let live = get("/s2mm/live");
    assert!(live.starts_with("HTTP/1.1 200") && live.contains("\"ok\""), "{live}");
    let index = get("/");
    assert!(index.starts_with("HTTP/1.1 200"), "{}", &index[..200.min(index.len())]);
    assert!(index.contains("window.__s2mmMap = () => map;"), "map hook not injected");
    assert!(index.contains("/s2mm/overlay.js"));
    let js = get("/js/lib/maptalks-gl/maptalks-gl.js");
    assert!(js.starts_with("HTTP/1.1 200") && js.contains("text/javascript"));
    server.stop();
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn db_with_staged_mods_serializes() {
    let tmp = std::env::temp_dir().join(format!("s2mm-ser-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let a = tmp.join("Some Mod-123-1-0-1732400000.zip");
    make_zip(&a, &[("x/A_P.pak", fake_pak(&["a/b.cfg"])), ("y/A_P.pak", fake_pak(&["a/b.cfg"]))]);
    let mut db = Db::default();
    let e = stage(&tmp.join("mods"), &a, InstallMeta::default(), None).unwrap();
    let metadata = tmp.join("mods").join(&e.id).join("mod.json");
    assert!(metadata.is_file(), "staged mods need metadata even if state saving fails");
    db.profile_mut().mods.push(ProfileMod { id: e.id.clone(), enabled: true });
    db.mods.push(e);
    db.normalize();
    let path = tmp.join("state.json");
    crate::util::write_json(&path, &db).expect("state.json must be writable");
    let back: Db = crate::util::read_json(&path).unwrap();
    assert_eq!(back.mods.len(), 1);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// Re-stages every mod of a real library (S2MM_TEST_LIBRARY=<data>/mods) into a temp folder
/// and saves the resulting database – catches data that cannot be serialized.
#[test]
#[ignore = "needs a real mod library"]
fn real_library_serializes() {
    let Some(lib) = std::env::var_os("S2MM_TEST_LIBRARY").map(PathBuf::from) else { return };
    let tmp = std::env::temp_dir().join(format!("s2mm-lib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let mut db = Db::default();
    for e in std::fs::read_dir(&lib).unwrap().flatten() {
        let files = e.path().join("files");
        if !files.is_dir() {
            continue;
        }
        for top in std::fs::read_dir(&files).unwrap().flatten() {
            match stage(&tmp.join("mods"), &top.path(), InstallMeta::default(), None) {
                Ok(m) => {
                    println!("staged {} ({} groups)", m.name, m.layout.groups.len());
                    if let Err(err) = serde_json::to_vec(&m) {
                        println!("  !! cannot serialize {}: {err}", m.name);
                    }
                    db.mods.push(m);
                }
                Err(err) => println!("stage {:?}: {err:#}", top.path()),
            }
        }
    }
    db.normalize();
    crate::util::write_json(&tmp.join("state.json"), &db).expect("save failed");
    println!("saved {} mods", db.mods.len());
    let _ = std::fs::remove_dir_all(&tmp);
}
