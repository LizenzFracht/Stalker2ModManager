// Browser-only preview of the UI with fake backend data (`npm run dev` outside Tauri).
// Also used to render the screenshots in docs/ (see scripts/screenshots.mjs).
// URL parameters: ?lang=en|de  ?welcome=1 (show the first-start dialog)
import { mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { AppState, Binding, ConflictReport, ModDetails, ModView, SaveView, TweakState } from "./types";

const params = new URLSearchParams(location.search);
let protectEngineIni = true;
const ue4ssLog = params.get("ue4ss") === "failed"
  ? { state: "failed" as const, detail: "Fatal Error: AOB scans could not be completed\nWas unable to find AOB for 'GUObjectArray'", logPath: "Stalker2/Binaries/Win64/ue4ss/UE4SS.log", logTime: 1761400000 }
  : { state: "unknown" as const, detail: null, logPath: null, logTime: 0 };
const NEXUS = "stalker2heartofchornobyl";

const base = {
  summary: null,
  category: null,
  notes: "",
  installedAt: 1761400000,
  updatedAt: 1761400000,
  archiveName: null,
  nexus: null,
  pictureUrl: null,
  layout: { root: "", fomod: false, base: [], groups: [] },
  selection: {},
  needsSetup: false,
  update: null,
  enabled: true,
  plan: { kinds: ["pak"], files: 1, unmapped: [] },
  conflicts: { wins: 0, loses: 0 },
} satisfies Partial<ModView>;

// Fictional example mods.
let mods: ModView[] = [
  { ...base, id: "ue4ss", name: "UE4SS", version: "v3.0.1-1183", author: "UE4SS-RE", priority: 1, category: "Framework", summary: "Lua scripting framework for Unreal Engine – required by Lua mods and the live map.", plan: { kinds: ["binary"], files: 27, unmapped: [] } },
  { ...base, id: "mcm", name: "Mod Configuration Menu", version: "2.0", author: "MCM team", priority: 2, category: "Framework", plan: { kinds: ["zoneKit"], files: 6, unmapped: [] } },
  { ...base, id: "overhaul", name: "Zone Survival Overhaul", version: "3.2", author: "loner_dev", priority: 3, category: "Gameplay", archiveName: "Zone Survival Overhaul-410-3-2-1760000000.zip", nexus: { gameDomain: NEXUS, modId: 410, fileId: 1201 }, summary: "Rebalances hunger, stamina, weight and trader prices for a slower, grittier survival experience.", plan: { kinds: ["pak"], files: 3, unmapped: ["readme.txt"] }, conflicts: { wins: 0, loses: 2 } },
  { ...base, id: "ballistics", name: "Realistic Ballistics", version: "1.8", author: "gunsmith", priority: 4, category: "Weapons", archiveName: "Realistic Ballistics-512-1-8-1759000000.7z", nexus: { gameDomain: NEXUS, modId: 512, fileId: 1333 }, summary: "Bullet drop, penetration and damage values based on real ammunition data.", conflicts: { wins: 1, loses: 0 } },
  {
    ...base, id: "hud", name: "Clean PDA HUD", version: "2.0", author: "pda_dev", priority: 5, category: "UI", plan: { kinds: ["pak"], files: 3, unmapped: [] },
    layout: { root: "CleanHUD", fomod: true, base: [], groups: [
      { id: "s0g0", name: "HUD style", step: "Appearance", kind: "exactlyOne", options: [
        { id: "s0g0o0", name: "Minimal", description: "Only compass and ammo counter. Everything else fades in when needed.", image: null, files: [], defaultSelected: false },
        { id: "s0g0o1", name: "Classic", description: "All elements of the original HUD with a cleaner look.", image: null, files: [], defaultSelected: true },
        { id: "s0g0o2", name: "Immersive", description: "No HUD at all – check your PDA.", image: null, files: [], defaultSelected: false },
      ] },
      { id: "s0g1", name: "Extras", step: "Appearance", kind: "any", options: [
        { id: "s0g1o0", name: "Larger font", description: "Easier to read on 4K screens.", image: null, files: [], defaultSelected: false },
        { id: "s0g1o1", name: "Colour-blind palette", description: "", image: null, files: [], defaultSelected: false },
      ] },
    ] },
  },
  { ...base, id: "weight", name: "Carry Weight Tweaks", version: null, author: null, priority: 6, archiveName: "Carry Weight Tweaks.zip", needsSetup: true, conflicts: { wins: 2, loses: 0 },
    layout: { root: "Carry Weight Tweaks", fomod: false, base: [], groups: [{ id: "g0", name: "Variante", step: null, kind: "exactlyOne", options: [
      { id: "g0o0", name: "+50 kg", description: "", image: null, files: [], defaultSelected: true },
      { id: "g0o1", name: "+100 kg", description: "", image: null, files: [], defaultSelected: false },
      { id: "g0o2", name: "Unlimited", description: "", image: null, files: [], defaultSelected: false },
    ] }] } },
  { ...base, id: "lua", name: "Quick Actions", version: "0.4", author: "scriptkid", priority: 7, category: "Utility", plan: { kinds: ["ue4ss"], files: 3, unmapped: [] } },
  { ...base, id: "nightvision", name: "Better Night Vision", version: "1.1", author: "owl", priority: 8, enabled: false, update: { available: true, latestVersion: "1.2", latestFileId: 99, checkedAt: 0 } },
];

const state = (): AppState => ({
  recoveryWarnings: params.has("recovery") ? [params.get("lang") === "de" ? "Mod aus der Bibliothek wiederhergestellt (deaktiviert)." : "Mod recovered from the library (disabled)."] : [],
  settings: {
    gamePath: "C:\\Program Files (x86)\\Steam\\steamapps\\common\\S.T.A.L.K.E.R. 2 Heart of Chornobyl",
    platform: "steam",
    configDir: null,
    launchMethod: "auto",
    launchArgs: "",
    useHardlinks: true,
    addPSuffix: true,
    nexusApiKey: null,
    nxmHandler: false,
    nxmPrevious: null,
    autoInstallDownloads: true,
    checkUpdatesOnStart: true,
    ue4ssModId: "ue4ss",
    livemapModId: "livemap",
    mapPort: 27420,
    firstRunDone: params.get("welcome") !== "1",
    language: params.get("lang") ?? "auto",
    protectEngineIni,
  },
  game: {
    path: "C:\\Program Files (x86)\\Steam\\steamapps\\common\\S.T.A.L.K.E.R. 2 Heart of Chornobyl",
    platform: "steam",
    valid: true,
    binariesDir: "Stalker2/Binaries/Win64",
    modsDir: "Stalker2/Content/Paks/~mods",
    ue4ssModsDir: "Stalker2/Binaries/Win64/ue4ss/Mods",
    ue4ssInstalled: true,
    exe: "Stalker2.exe",
    configDir: "C:\\Users\\stalker\\AppData\\Local\\Stalker2\\Saved\\Config\\Windows",
    build: "26123434",
  },
  detected: [],
  mods,
  profiles: [{ id: "vanilla", name: "Vanilla" }, { id: "default", name: "Standard" }, { id: "p2", name: "Hardcore run" }],
  activeProfile: "default",
  pending: true,
  appliedAt: 0,
  deployedFiles: 42,
  unmanaged: [],
  gameUpdated: false,
  nexusUser: null,
  nexusEnabled: false,
  dataDir: "C:\\Users\\stalker\\AppData\\Roaming\\de.s2mm.stalker2modmanager",
  appVersion: "0.1.0",
  conflictErrors: 0,
  tools: {
    ue4ss: { modId: "ue4ss", enabled: true, version: "v3.0.1-1183" },
    ue4ssInGame: true,
    ue4ssLog,
    liveMapMod: { modId: "livemap", enabled: true, version: "0.1.0" },
    mcm: { modId: "mcm", enabled: true, version: "2.0" },
  },
});

const CFG = "Stalker2/Content/GameLite/GameData";
const conflicts: ConflictReport = {
  pairs: [
    { winner: "weight", loser: "overhaul", count: 2, items: [`${CFG}/ObjPrototypes/GeneralNPCObjPrototypes.cfg`, `${CFG}/ObjPrototypes.cfg`], kind: "asset" },
    { winner: "ballistics", loser: "overhaul", count: 1, items: [`${CFG}/ItemPrototypes/WeaponPrototypes.cfg`], kind: "asset" },
  ],
  errors: [],
  perMod: {},
};

const details: ModDetails = {
  files: [
    { logical: "ZoneSurvival_P.pak", target: "Stalker2/Content/Paks/~mods/003_zone-survival-overhaul_ZoneSurvival_P.pak", kind: "pak", size: 2_481_213 },
    { logical: "ZoneSurvival_P.utoc", target: "Stalker2/Content/Paks/~mods/003_zone-survival-overhaul_ZoneSurvival_P.utoc", kind: "pak", size: 18_211 },
    { logical: "ZoneSurvival_P.ucas", target: "Stalker2/Content/Paks/~mods/003_zone-survival-overhaul_ZoneSurvival_P.ucas", kind: "pak", size: 9_884_102 },
  ],
  unmapped: ["readme.txt"],
  paks: [{ file: "ZoneSurvival_P.pak", assets: [`${CFG}/ObjPrototypes.cfg`, `${CFG}/ObjPrototypes/GeneralNPCObjPrototypes.cfg`, `${CFG}/ItemPrototypes/WeaponPrototypes.cfg`, `${CFG}/Stamina.cfg`], error: null }],
  configFiles: ["QuickActions/Scripts/config.lua"],
  size: 12_383_526,
};

const tweaks: TweakState = {
  configDir: "C:\\Users\\stalker\\AppData\\Local\\Stalker2\\Saved\\Config\\Windows",
  engineIniExists: true,
  backupExists: true,
  readOnly: false,
  values: { "r.SceneColorFringeQuality": "0", "r.Tonemapper.Quality": "1", "r.Streaming.PoolSize": "6000", "r.Tonemapper.Sharpen": "0.6", "D3D12.PSO.DiskCache": "1" },
};

const keybinds: Binding[] = [
  { id: "1", source: "ue4ss", modId: "lua", modName: "Quick Actions", enabled: true, file: "QuickActions/Scripts/main.lua", line: 14, action: "toggleFlashlightMode", key: "L", modifiers: ["ALT"], style: "ue4ss", editable: true },
  { id: "2", source: "ue4ss", modId: "lua", modName: "Quick Actions", enabled: true, file: "QuickActions/Scripts/main.lua", line: 22, action: "quickHeal", key: "F5", modifiers: [], style: "ue4ss", editable: true },
  { id: "3", source: "config", modId: "hud", modName: "Clean PDA HUD", enabled: true, file: "CleanHUD/config.ini", line: 3, action: "ToggleHudKey", key: "Insert", modifiers: [], style: "plain", editable: true },
  { id: "4", source: "mcm", modId: null, modName: "Mod Configuration Menu", enabled: true, file: "x.sav", line: 0, action: "Open menu", key: "B", modifiers: [], style: "/p", editable: true },
  { id: "5", source: "mcm", modId: null, modName: "Zone Survival Overhaul", enabled: true, file: "x.sav", line: 0, action: "Show needs", key: "NumPadOne", modifiers: [], style: "/q", editable: true },
  { id: "g1", source: "game", modId: null, modName: "Stalker 2 (default)", enabled: true, file: null, line: 0, action: "Quick save", key: "F5", modifiers: [], style: "ue4ss", editable: false },
  { id: "g2", source: "game", modId: null, modName: "Stalker 2 (default)", enabled: true, file: null, line: 0, action: "Crouch", key: "C", modifiers: [], style: "ue4ss", editable: false },
];

const save: SaveView = {
  saveType: "/Game/MCM/SG_MCM_Settings.SG_MCM_Settings_C",
  leaves: [
    { pointer: "/a", path: ["Mods", "Zone Survival Overhaul"], label: "Hunger rate", kind: "float", value: 1.25 },
    { pointer: "/b", path: ["Mods", "Zone Survival Overhaul"], label: "Realistic sleep", kind: "bool", value: true },
    { pointer: "/c", path: ["Mods", "Zone Survival Overhaul"], label: "Show needs", kind: "key", value: "NumPadOne" },
    { pointer: "/d", path: ["Mods", "Clean PDA HUD"], label: "Show compass", kind: "bool", value: true },
    { pointer: "/e", path: ["Mods", "Clean PDA HUD"], label: "HUD opacity (%)", kind: "int", value: 80 },
    { pointer: "/f", path: ["Mods", "Clean PDA HUD"], label: "Fade out in combat", kind: "bool", value: false },
  ],
};

export function install() {
  mockWindows("main");
  mockConvertFileSrc("windows");
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    switch (cmd) {
      case "get_conflicts":
        return conflicts;
      case "mod_details":
        return details;
      case "get_tweaks":
      case "set_tweaks":
        return tweaks;
      case "set_engine_ini_protection":
        if (params.has("failProtection")) throw new Error("Could not change file permissions");
        protectEngineIni = a.protect as boolean;
        tweaks.readOnly = protectEngineIni;
        if (params.has("failProtectionSave")) throw new Error("Could not save state; recovery snapshot retained");
        return tweaks;
      case "get_downloads":
        return [];
      case "get_keybinds":
        return keybinds;
      case "list_saves":
        return { dir: "C:\\Users\\stalker\\AppData\\Local\\Stalker2\\Saved\\SaveGames", files: [{ name: "MCM_Settings.sav", path: "x.sav", size: 4096, modified: 1761400000, isMcm: true }] };
      case "read_save":
        return save;
      case "livemap_status":
      case "livemap_start":
        return { running: true, url: "http://127.0.0.1:27420/", ue4ssLog, position: { ok: ue4ssLog.state !== "failed", age: 1, x: 402400, y: 578997, z: 1200, yaw: 37 } };
      case "autosort_preview":
        return [
          { id: "ue4ss", from: 0, to: 0, reason: "Framework/base" },
          { id: "mcm", from: 1, to: 1, reason: "Framework/base" },
          { id: "lua", from: 6, to: 2, reason: "Script/DLL mod without pak conflicts" },
          { id: "overhaul", from: 2, to: 3, reason: "Pak mod · 412 assets" },
          { id: "hud", from: 4, to: 4, reason: "Pak mod · 38 assets" },
          { id: "ballistics", from: 3, to: 5, reason: "Pak mod · 12 assets" },
          { id: "nightvision", from: 7, to: 6, reason: "Pak mod · 4 assets" },
          { id: "weight", from: 5, to: 7, reason: "Patch/compatibility – has to win" },
        ];
      case "restore_vanilla":
        return {
          state: { ...state(), activeProfile: "vanilla", settings: { ...state().settings, firstRunDone: true } },
          report: { removedDeployed: 42, moved: ["Stalker2/Content/Paks/~mods/OldMod_P.pak"], engineIniReset: ["Engine.ini"], quarantine: "C:/backup" },
        };
      case "detect_games":
        return [];
      case "read_mod_file":
        return "-- Quick Actions configuration\nreturn {\n  healKey = Key.F5,\n  flashlightKey = Key.L,\n  healAmount = 25,\n}\n";
      case "third_party_notices":
        return "# Third-party notices\n\n(generated at build time)";
      case "set_enabled":
        mods = mods.map((m) => ((a.ids as string[]).includes(m.id) ? { ...m, enabled: a.enabled as boolean } : m));
        return state();
      case "reorder_mods":
        mods = (a.ids as string[]).map((id, i) => ({ ...mods.find((m) => m.id === id)!, priority: i + 1 }));
        return state();
      case "apply":
        return { state: { ...state(), pending: false }, placed: 42, removed: 3, kept: 0 };
      default:
        if (cmd.startsWith("plugin:")) return 1;
        return state();
    }
  });
}
