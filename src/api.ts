import { invoke } from "@tauri-apps/api/core";
import type {
  AppState,
  Binding,
  LiveStatus,
  SaveFileInfo,
  SaveView,
  SortEntry,
  VanillaReport,
  ConflictReport,
  DetectedInstall,
  Download,
  ModDetails,
  Platform,
  Settings,
  TweakState,
} from "./types";

export interface AddResult {
  state: AppState;
  added: string[];
  errors: string[];
}

export interface ApplyOutcome {
  state: AppState;
  placed: number;
  removed: number;
  kept: number;
}

export const api = {
  getState: () => invoke<AppState>("get_state"),
  detectGames: () => invoke<DetectedInstall[]>("detect_games"),
  setGamePath: (path: string, platform?: Platform) => invoke<AppState>("set_game_path", { path, platform: platform ?? null }),
  saveSettings: (settings: Settings) => invoke<AppState>("save_settings", { settings }),
  launchGame: () => invoke<void>("launch_game"),
  openLocation: (what: string) => invoke<void>("open_location", { what }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),

  addMods: (paths: string[]) => invoke<AddResult>("add_mods", { paths }),
  reinstallMod: (id: string, path: string) => invoke<AppState>("reinstall_mod", { id, path }),
  removeMod: (id: string) => invoke<AppState>("remove_mod", { id }),
  setEnabled: (ids: string[], enabled: boolean) => invoke<AppState>("set_enabled", { ids, enabled }),
  reorderMods: (ids: string[]) => invoke<AppState>("reorder_mods", { ids }),
  setSelection: (id: string, selection: Record<string, string[]>) => invoke<AppState>("set_selection", { id, selection }),
  updateMod: (
    id: string,
    patch: { name?: string; version?: string; notes?: string; category?: string; nexusModId?: number },
  ) => invoke<AppState>("update_mod", { id, patch }),
  apply: () => invoke<ApplyOutcome>("apply"),
  restoreVanilla: (options: { ue4ss: boolean; engineIni: boolean; foreignMods: boolean }) =>
    invoke<{ state: AppState; report: VanillaReport }>("restore_vanilla", { options }),
  finishFirstRun: () => invoke<AppState>("finish_first_run"),
  openFolder: (path: string) => invoke<void>("open_folder", { path }),
  autosortPreview: () => invoke<SortEntry[]>("autosort_preview"),
  purge: () => invoke<AppState>("purge"),
  getConflicts: () => invoke<ConflictReport>("get_conflicts"),
  modDetails: (id: string) => invoke<ModDetails>("mod_details", { id }),
  readModFile: (id: string, path: string) => invoke<string>("read_mod_file", { id, path }),
  writeModFile: (id: string, path: string, content: string) => invoke<AppState>("write_mod_file", { id, path, content }),
  importUnmanaged: (files: string[]) => invoke<AddResult>("import_unmanaged", { files }),

  createProfile: (name: string, copyCurrent: boolean) => invoke<AppState>("create_profile", { name, copyCurrent }),
  renameProfile: (id: string, name: string) => invoke<AppState>("rename_profile", { id, name }),
  deleteProfile: (id: string) => invoke<AppState>("delete_profile", { id }),
  switchProfile: (id: string) => invoke<AppState>("switch_profile", { id }),

  getTweaks: () => invoke<TweakState>("get_tweaks"),
  setEngineIniProtection: (protect: boolean) => invoke<TweakState>("set_engine_ini_protection", { protect }),
  setTweaks: (changes: Record<string, string | null>) => invoke<TweakState>("set_tweaks", { changes }),
  restoreTweaks: () => invoke<TweakState>("restore_tweaks"),
  exportEngineIni: (path: string) => invoke<void>("export_engine_ini", { path }),
  importEngineIni: (path: string, replace: boolean) =>
    invoke<{ entries: number; state: TweakState }>("import_engine_ini", { path, replace }),

  ue4ssInstall: () => invoke<AppState>("ue4ss_install"),
  setToolEnabled: (tool: "ue4ss" | "livemap" | "mcm", enabled: boolean) => invoke<AppState>("set_tool_enabled", { tool, enabled }),
  livemapInstallMod: () => invoke<AppState>("livemap_install_mod"),
  livemapStart: (openBrowser: boolean) => invoke<LiveStatus>("livemap_start", { openBrowser }),
  livemapStop: () => invoke<LiveStatus>("livemap_stop"),
  livemapStatus: () => invoke<LiveStatus>("livemap_status"),
  listSaves: () => invoke<{ dir: string | null; files: SaveFileInfo[] }>("list_saves"),
  readSave: (path: string) => invoke<SaveView>("read_save", { path }),
  writeSave: (path: string, edits: { pointer: string; value: unknown }[]) => invoke<SaveView>("write_save", { path, edits }),
  restoreSave: (path: string) => invoke<SaveView>("restore_save", { path }),
  getKeybinds: () => invoke<Binding[]>("get_keybinds"),
  rebindKey: (binding: Binding, key: string, modifiers: string[]) => invoke<Binding[]>("rebind_key", { binding, key, modifiers }),

  nexusLogin: (key: string) => invoke<AppState>("nexus_login", { key }),
  nexusLogout: () => invoke<AppState>("nexus_logout"),
  checkUpdates: () => invoke<AppState>("check_updates"),
  nexusUpdateMod: (id: string) => invoke<void>("nexus_update_mod", { id }),
  handleNxm: (url: string) => invoke<void>("handle_nxm", { url }),
  getDownloads: () => invoke<Download[]>("get_downloads"),
  installDownload: (id: string) => invoke<void>("install_download", { id }),
  removeDownload: (id: string, deleteFile: boolean) => invoke<void>("remove_download", { id, deleteFile }),
};

export const NEXUS_GAME_URL = "https://www.nexusmods.com/stalker2heartofchornobyl";

export function nexusModUrl(modId: number) {
  return `${NEXUS_GAME_URL}/mods/${modId}`;
}

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
}
