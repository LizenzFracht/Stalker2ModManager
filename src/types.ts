export type Platform = "steam" | "gamePass" | "gog" | "epic" | "unknown";
export type LaunchMethod = "auto" | "steam" | "direct";
export type TargetKind = "pak" | "zoneKit" | "ue4ss" | "binary" | "gameFile";
export type GroupKind = "exactlyOne" | "atMostOne" | "atLeastOne" | "any" | "all";

export interface Settings {
  gamePath: string | null;
  platform: Platform;
  configDir: string | null;
  launchMethod: LaunchMethod;
  launchArgs: string;
  useHardlinks: boolean;
  addPSuffix: boolean;
  nexusApiKey: string | null;
  nxmHandler: boolean;
  nxmPrevious: string | null;
  autoInstallDownloads: boolean;
  checkUpdatesOnStart: boolean;
  ue4ssModId: string | null;
  livemapModId: string | null;
  mapPort: number;
  firstRunDone: boolean;
}

export interface VanillaReport {
  removedDeployed: number;
  moved: string[];
  engineIniReset: string[];
  quarantine: string | null;
}

export interface SortEntry {
  id: string;
  from: number;
  to: number;
  reason: string;
}

export interface ToolMod {
  modId: string | null;
  enabled: boolean;
  version: string | null;
}

export interface ToolsView {
  ue4ss: ToolMod;
  ue4ssInGame: boolean;
  liveMapMod: ToolMod;
  mcm: ToolMod;
}

export interface Binding {
  id: string;
  source: "ue4ss" | "config" | "mcm" | "game";
  modId: string | null;
  modName: string;
  enabled: boolean;
  file: string | null;
  line: number;
  action: string;
  key: string;
  modifiers: string[];
  style: string;
  editable: boolean;
}

export interface SaveFileInfo {
  name: string;
  path: string;
  size: number;
  modified: number;
  isMcm: boolean;
}

export interface SaveLeaf {
  pointer: string;
  path: string[];
  label: string;
  kind: "bool" | "int" | "float" | "string" | "key";
  value: boolean | number | string;
}

export interface SaveView {
  saveType: string;
  leaves: SaveLeaf[];
}

export interface LiveStatus {
  running: boolean;
  url: string | null;
  position: { ok: boolean; age: number; x: number; y: number; z: number; yaw: number };
}

export interface GameInfo {
  path: string;
  platform: Platform;
  valid: boolean;
  binariesDir: string;
  modsDir: string;
  ue4ssModsDir: string;
  ue4ssInstalled: boolean;
  exe: string | null;
  configDir: string | null;
  build: string | null;
}

export interface DetectedInstall {
  path: string;
  platform: Platform;
}

export interface FileMap {
  src: string;
  dest: string;
  priority: number;
}

export interface ModOption {
  id: string;
  name: string;
  description: string;
  image: string | null;
  files: FileMap[];
  defaultSelected: boolean;
}

export interface OptionGroup {
  id: string;
  name: string;
  step: string | null;
  kind: GroupKind;
  options: ModOption[];
}

export interface ModLayout {
  root: string;
  fomod: boolean;
  base: FileMap[];
  groups: OptionGroup[];
}

export interface NexusRef {
  gameDomain: string;
  modId: number;
  fileId: number | null;
}

export interface UpdateInfo {
  available: boolean;
  latestVersion: string | null;
  latestFileId: number | null;
  checkedAt: number;
}

export interface ModView {
  id: string;
  name: string;
  version: string | null;
  author: string | null;
  summary: string | null;
  category: string | null;
  notes: string;
  installedAt: number;
  updatedAt: number;
  archiveName: string | null;
  nexus: NexusRef | null;
  pictureUrl: string | null;
  layout: ModLayout;
  selection: Record<string, string[]>;
  needsSetup: boolean;
  update: UpdateInfo | null;
  enabled: boolean;
  priority: number;
  plan: { kinds: TargetKind[]; files: number; unmapped: string[] };
  conflicts: { wins: number; loses: number };
}

export interface NexusUser {
  name: string;
  isPremium: boolean;
  profileUrl: string | null;
}

export interface AppState {
  settings: Settings;
  game: GameInfo | null;
  detected: DetectedInstall[];
  mods: ModView[];
  profiles: { id: string; name: string }[];
  activeProfile: string;
  pending: boolean;
  appliedAt: number;
  deployedFiles: number;
  unmanaged: string[];
  gameUpdated: boolean;
  nexusUser: NexusUser | null;
  dataDir: string;
  appVersion: string;
  conflictErrors: number;
  tools: ToolsView;
}

export interface ConflictPair {
  winner: string;
  loser: string;
  count: number;
  items: string[];
  kind: "asset" | "file";
}

export interface ConflictReport {
  pairs: ConflictPair[];
  errors: { modId: string; file: string; error: string }[];
  perMod: Record<string, { wins: number; loses: number }>;
}

export interface ModDetails {
  files: { logical: string; target: string; kind: TargetKind; size: number }[];
  unmapped: string[];
  paks: { file: string; assets: string[]; error: string | null }[];
  configFiles: string[];
  size: number;
}

export interface TweakState {
  configDir: string | null;
  engineIniExists: boolean;
  backupExists: boolean;
  readOnly: boolean;
  values: Record<string, string>;
}

export interface Download {
  id: string;
  fileName: string;
  modId: number;
  fileId: number;
  modName: string | null;
  version: string | null;
  received: number;
  total: number | null;
  state: "running" | "done" | "failed" | "installed";
  error: string | null;
  path: string;
  startedAt: number;
}
