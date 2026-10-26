// Browser-only preview of the UI with fake backend data (`npm run dev` outside Tauri).
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { AppState, ConflictReport, ModDetails, ModView, TweakState } from "./types";

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

let mods: ModView[] = [
  { ...base, id: "a", name: "Better Stamina", version: "1.4", author: "zoneguy", priority: 1, archiveName: "Better Stamina-123-1-4-1732400000.zip", nexus: { gameDomain: "stalker2heartofchornobyl", modId: 123, fileId: 456 }, summary: "Mehr Ausdauer, langsamere Erschöpfung beim Sprinten.", conflicts: { wins: 0, loses: 1 }, update: { available: true, latestVersion: "1.5", latestFileId: 789, checkedAt: 0 } },
  { ...base, id: "b", name: "Weight Mod", version: null, author: null, priority: 2, archiveName: "Weight Mod.zip", needsSetup: true, conflicts: { wins: 1, loses: 0 }, layout: { root: "Weight Mod", fomod: false, base: [], groups: [{ id: "g0", name: "Variante", step: null, kind: "exactlyOne", options: [{ id: "g0o0", name: "x2", description: "", image: null, files: [], defaultSelected: true }, { id: "g0o1", name: "x5", description: "", image: null, files: [], defaultSelected: false }] }] } },
  { ...base, id: "c", name: "Fancy HUD", version: "2.0", author: "pda_dev", priority: 3, category: "UI", plan: { kinds: ["pak"], files: 3, unmapped: ["readme.txt"] }, layout: { root: "FancyHud", fomod: true, base: [], groups: [{ id: "s0g0", name: "HUD-Stil", step: "Darstellung", kind: "exactlyOne", options: [{ id: "s0g0o0", name: "Minimal", description: "Nur Kompass und Munition.", image: "img/min.png", files: [], defaultSelected: false }, { id: "s0g0o1", name: "Vollständig", description: "Alle Elemente des Original-HUDs.", image: null, files: [], defaultSelected: true }] }, { id: "s0g1", name: "Extras", step: "Darstellung", kind: "any", options: [{ id: "s0g1o0", name: "Größere Schrift", description: "", image: null, files: [], defaultSelected: false }] }] } },
  { ...base, id: "d", name: "Lua Tweaks", version: "0.3", author: null, priority: 4, enabled: false, plan: { kinds: ["ue4ss", "binary"], files: 4, unmapped: [] } },
  { ...base, id: "e", name: "Leere Mod", version: null, author: null, priority: 5, enabled: false, plan: { kinds: [], files: 0, unmapped: ["notes.txt", "Stamina.cfg"] } },
];

const state = (): AppState => ({
  settings: {
    gamePath: "C:\\Program Files (x86)\\Steam\\steamapps\\common\\S.T.A.L.K.E.R. 2 Heart of Chornobyl",
    platform: "steam",
    configDir: null,
    launchMethod: "auto",
    launchArgs: "",
    useHardlinks: true,
    addPSuffix: true,
    nexusApiKey: "x",
    nxmHandler: true,
    nxmPrevious: null,
    autoInstallDownloads: true,
    checkUpdatesOnStart: true,
    ue4ssModId: "d",
    livemapModId: null,
    mapPort: 27420,
    firstRunDone: false,
  },
  game: {
    path: "C:\\Program Files (x86)\\Steam\\steamapps\\common\\S.T.A.L.K.E.R. 2 Heart of Chornobyl",
    platform: "steam",
    valid: true,
    binariesDir: "Stalker2/Binaries/Win64",
    modsDir: "Stalker2/Content/Paks/~mods",
    ue4ssModsDir: "Stalker2/Binaries/Win64/ue4ss/Mods",
    ue4ssInstalled: false,
    exe: "Stalker2.exe",
    configDir: "C:\\Users\\stalker\\AppData\\Local\\Stalker2\\Saved\\Config\\Windows",
    build: "26123434",
  },
  detected: [],
  mods,
  profiles: [{ id: "vanilla", name: "Vanilla" }, { id: "default", name: "Standard" }, { id: "p2", name: "Hardcore" }],
  activeProfile: "default",
  pending: true,
  appliedAt: 0,
  deployedFiles: 6,
  unmanaged: ["Stalker2/Content/Paks/~mods/Manual_P.pak"],
  gameUpdated: false,
  nexusUser: { name: "Strelok", isPremium: false, profileUrl: null },
  dataDir: "C:\\Users\\stalker\\AppData\\Roaming\\de.s2mm.stalker2modmanager",
  appVersion: "0.1.0",
  conflictErrors: 0,
  tools: {
    ue4ss: { modId: "d", enabled: false, version: "v3.0.1-1183" },
    ue4ssInGame: false,
    liveMapMod: { modId: null, enabled: false, version: null },
    mcm: { modId: "c", enabled: true, version: "2.0" },
  },
});

const conflicts: ConflictReport = {
  pairs: [{ winner: "b", loser: "a", count: 1, items: ["Stalker2/Content/GameLite/GameData/ObjPrototypes.cfg"], kind: "asset" }],
  errors: [],
  perMod: {},
};

const details: ModDetails = {
  files: [{ logical: "BetterStamina_P.pak", target: "Stalker2/Content/Paks/~mods/001_better-stamina_BetterStamina_P.pak", kind: "pak", size: 48213 }],
  unmapped: ["readme.txt"],
  paks: [{ file: "BetterStamina_P.pak", assets: ["Stalker2/Content/GameLite/GameData/ObjPrototypes.cfg", "Stalker2/Content/GameLite/GameData/Stamina.cfg"], error: null }],
  configFiles: ["Lua/config.lua"],
  size: 48213,
};

const tweaks: TweakState = {
  configDir: "C:\\Users\\stalker\\AppData\\Local\\Stalker2\\Saved\\Config\\Windows",
  engineIniExists: true,
  backupExists: true,
  readOnly: false,
  values: { "r.SceneColorFringeQuality": "0", "r.Streaming.PoolSize": "6000", "r.Lumen.Reflections.Allow": "0" },
};

export function install() {
  mockWindows("main");
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
      case "get_downloads":
        return [
          { id: "1", fileName: "Fancy HUD-77-2-0-1732.zip", modId: 77, fileId: 5, modName: "Fancy HUD", version: "2.0", received: 3_400_000, total: 9_100_000, state: "running", error: null, path: "", startedAt: 0 },
          { id: "2", fileName: "Better Stamina-123-1-4.zip", modId: 123, fileId: 4, modName: "Better Stamina", version: "1.4", received: 50_000, total: 50_000, state: "installed", error: null, path: "", startedAt: 0 },
        ];
      case "get_keybinds":
        return [
          { id: "1", source: "ue4ss", modId: "d", modName: "Lua Tweaks", enabled: true, file: "LuaTweaks/Scripts/main.lua", line: 12, action: "toggleNoclip", key: "C", modifiers: ["ALT"], style: "ue4ss", editable: true },
          { id: "2", source: "ue4ss", modId: "d", modName: "Lua Tweaks", enabled: true, file: "LuaTweaks/Scripts/main.lua", line: 20, action: "Schnellspeichern+", key: "F5", modifiers: [], style: "ue4ss", editable: true },
          { id: "3", source: "config", modId: "c", modName: "Fancy HUD", enabled: true, file: "config.ini", line: 3, action: "ToggleHudKey", key: "Insert", modifiers: [], style: "plain", editable: true },
          { id: "4", source: "mcm", modId: null, modName: "Better Stamina", enabled: true, file: "x.sav", line: 0, action: "Ausdauer-Anzeige", key: "NumPadOne", modifiers: [], style: "/p", editable: true },
          { id: "g1", source: "game", modId: null, modName: "Stalker 2 (Standard)", enabled: true, file: null, line: 0, action: "Schnellspeichern", key: "F5", modifiers: [], style: "ue4ss", editable: false },
          { id: "g2", source: "game", modId: null, modName: "Stalker 2 (Standard)", enabled: true, file: null, line: 0, action: "Ducken", key: "C", modifiers: [], style: "ue4ss", editable: false },
        ];
      case "list_saves":
        return { dir: "C:\Users\stalker\AppData\Local\Stalker2\Saved\SaveGames", files: [{ name: "MCM_Settings.sav", path: "x.sav", size: 4096, modified: 1761400000, isMcm: true }] };
      case "read_save":
        return {
          saveType: "/Game/MCM/SG_MCM_Settings.SG_MCM_Settings_C",
          leaves: [
            { pointer: "/a", path: ["Mods", "Better Stamina"], label: "Ausdauer-Multiplikator", kind: "float", value: 1.5 },
            { pointer: "/b", path: ["Mods", "Better Stamina"], label: "Erschöpfung aktiv", kind: "bool", value: true },
            { pointer: "/c", path: ["Mods", "Better Stamina"], label: "Ausdauer-Anzeige", kind: "key", value: "NumPadOne" },
            { pointer: "/d", path: ["Mods", "Fancy HUD"], label: "Kompass zeigen", kind: "bool", value: false },
            { pointer: "/e", path: ["Mods", "Fancy HUD"], label: "Transparenz", kind: "int", value: 80 },
          ],
        };
      case "livemap_status":
      case "livemap_start":
        return { running: cmd === "livemap_start", url: cmd === "livemap_start" ? "http://127.0.0.1:27420/" : null, position: { ok: true, age: 1, x: 402400, y: 578997, z: 1200, yaw: 37 } };
      case "autosort_preview":
        return [
          { id: "d", from: 3, to: 0, reason: "Skript-/DLL-Mod ohne Pak-Konflikte" },
          { id: "c", from: 2, to: 1, reason: "Pak-Mod · 412 Assets" },
          { id: "a", from: 0, to: 2, reason: "Pak-Mod · 3 Assets" },
          { id: "b", from: 1, to: 3, reason: "Patch für „Better Stamina“ – direkt danach" },
          { id: "e", from: 4, to: 4, reason: "Pak-Mod" },
        ];
      case "restore_vanilla":
        return { state: { ...state(), activeProfile: "vanilla", settings: { ...state().settings, firstRunDone: true } }, report: { removedDeployed: 6, moved: ["Stalker2/Binaries/Win64/ue4ss", "Stalker2/Binaries/Win64/dwmapi.dll"], engineIniReset: ["C:/x/Engine.ini"], quarantine: "C:/q" } };
      case "detect_games":
        return [];
      case "read_mod_file":
        return "return {\n  speed = 2,\n}\n";
      case "set_enabled":
        mods = mods.map((m) => ((a.ids as string[]).includes(m.id) ? { ...m, enabled: a.enabled as boolean } : m));
        return state();
      case "reorder_mods":
        mods = (a.ids as string[]).map((id, i) => ({ ...mods.find((m) => m.id === id)!, priority: i + 1 }));
        return state();
      case "apply":
        return { state: { ...state(), pending: false }, placed: 5, removed: 1, kept: 0 };
      default:
        if (cmd.startsWith("plugin:")) return 1;
        return state();
    }
  });
}
