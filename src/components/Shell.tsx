import { useEffect, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  Download,
  FolderOpen,
  Info,
  Keyboard,
  Lock,
  Layers,
  ListChecks,
  Map as MapIcon,
  Loader2,
  Play,
  Plus,
  Settings,
  SlidersHorizontal,
  Swords,
  Pencil,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import { api } from "../api";
import { useStore, type Page } from "../store";
import { Button, confirmDialog, Input, Modal, platformLabel } from "./ui";
import { t } from "../i18n";

/** Built-in profiles are shown in the UI language. */
function profileName(id?: string, name?: string) {
  if (id === "vanilla") return "Vanilla";
  if (id === "default" && name === "Standard") return t("Standard");
  return name ?? "";
}

export function Logo() {
  return (
    <svg viewBox="0 0 1024 1024" className="h-7 w-7" aria-hidden>
      <circle cx="512" cy="512" r="330" fill="none" stroke="currentColor" strokeWidth="64" />
      <circle cx="512" cy="512" r="74" fill="currentColor" />
      <path d="M512 512 L382 287 A260 260 0 0 1 642 287 Z" fill="currentColor" />
      <path d="M512 512 L772 512 A260 260 0 0 1 642 737 Z" fill="currentColor" />
      <path d="M512 512 L382 737 A260 260 0 0 1 252 512 Z" fill="currentColor" />
    </svg>
  );
}

export function Sidebar({ conflictCount }: { conflictCount: number }) {
  const { state, page, setPage, downloads } = useStore();
  const running = downloads.filter((d) => d.state === "running").length;
  type Item = { id: Page; label: string; icon: typeof Layers; count?: number; tone?: string };
  const groups: { title?: string; items: Item[] }[] = [
    {
      items: [
        { id: "mods", label: t("Mods"), icon: Layers, count: state?.mods.length },
        { id: "conflicts", label: t("Konflikte"), icon: Swords, count: conflictCount || undefined, tone: "text-accent" },
        ...(state?.nexusEnabled ? [{ id: "downloads" as Page, label: t("Downloads"), icon: Download, count: running || undefined, tone: "text-accent" }] : []),
      ],
    },
    {
      title: t("Werkzeuge"),
      items: [
        { id: "mcm", label: t("Mod-Einstellungen"), icon: ListChecks },
        { id: "keybinds", label: t("Tastenbelegung"), icon: Keyboard },
        { id: "livemap", label: t("Live-Karte"), icon: MapIcon },
        { id: "tweaks", label: t("Spiel-Tweaks"), icon: SlidersHorizontal },
      ],
    },
    { items: [{ id: "settings", label: t("Einstellungen"), icon: Settings }] },
  ];
  return (
    <nav className="pda-texture flex w-56 shrink-0 flex-col border-r border-line bg-panel">
      <div className="flex items-center gap-2.5 px-5 pb-6 pt-5 text-accent">
        <Logo />
        <div className="leading-none">
          <div className="font-display text-2xl font-bold tracking-wider text-text">S2MM</div>
          <div className="label-caps mt-1 text-[10.5px] text-dim">{t("Stalker 2 Mod Manager")}</div>
        </div>
      </div>
      {groups.map((g, gi) => (
      <ul key={gi} className="mb-3 flex flex-col gap-0.5 px-3">
        {g.title && <li className="label-caps px-3 pb-1 pt-1 text-[11px] text-dim">{g.title}</li>}
        {g.items.map((it) => {
          const active = page === it.id;
          const Icon = it.icon;
          return (
            <li key={it.id}>
              <button
                onClick={() => setPage(it.id)}
                aria-current={active ? "page" : undefined}
                className={`relative flex h-10 w-full items-center gap-3 rounded-md px-3 text-left text-[14px] transition-colors ${
                  active ? "bg-raised text-text" : "text-muted hover:bg-hover/60 hover:text-text"
                }`}
              >
                {active && <span className="absolute -left-3 top-2 bottom-2 w-[3px] rounded-r bg-accent" />}
                <Icon size={17} className={active ? "text-accent" : ""} />
                <span className="flex-1">{it.label}</span>
                {it.count !== undefined && (
                  <span className={`font-mono text-xs ${it.tone ?? "text-dim"}`}>{it.count}</span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
      ))}
      <div className="mt-auto border-t border-line px-5 py-4 text-xs text-dim">
        {!state?.nexusEnabled ? null : state?.nexusUser ? (
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-ok" />
            <span className="truncate text-muted">
              Nexus: {state.nexusUser.name}
              {state.nexusUser.isPremium && <span className="ml-1 text-accent">Premium</span>}
            </span>
          </div>
        ) : (
          <button className="flex items-center gap-2 hover:text-muted" onClick={() => setPage("settings")}>
            <span className="h-2 w-2 rounded-full bg-dim" />
            {t("Nexus nicht verbunden")}
          </button>
        )}
        <div className="mt-2 font-mono text-[11px]">v{state?.appVersion}</div>
      </div>
    </nav>
  );
}

function ProfileMenu() {
  const { state, run, setState } = useStore();
  const [open, setOpen] = useState(false);
  const [dialog, setDialog] = useState<null | "new" | "rename">(null);
  const [name, setName] = useState("");
  const [copy, setCopy] = useState(true);
  if (!state) return null;
  const active = state.profiles.find((p) => p.id === state.activeProfile);

  const submit = async () => {
    if (!name.trim()) return;
    const s =
      dialog === "new"
        ? await run(() => api.createProfile(name, copy))
        : await run(() => api.renameProfile(state.activeProfile, name));
    if (s) setState(s);
    setDialog(null);
  };

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        className="flex h-9 items-center gap-2 rounded-md border border-line-strong bg-raised px-3 text-sm hover:bg-hover"
        aria-haspopup="menu"
        aria-expanded={open}
      >
        <span className="label-caps text-[11px] text-dim">{t("Profil")}</span>
        <span className="max-w-40 truncate">{profileName(active?.id, active?.name)}</span>
        <ChevronDown size={15} className="text-dim" />
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-20" onClick={() => setOpen(false)} />
          <div role="menu" className="absolute left-0 top-11 z-30 w-64 rounded-lg border border-line-strong bg-raised p-1.5 shadow-xl">
            {state.profiles.map((p) => (
              <button
                key={p.id}
                role="menuitemradio"
                aria-checked={p.id === state.activeProfile}
                onClick={async () => {
                  setOpen(false);
                  const s = await run(() => api.switchProfile(p.id));
                  if (s) setState(s);
                }}
                className="flex h-9 w-full items-center gap-2 rounded-md px-2.5 text-left text-sm hover:bg-hover"
              >
                <span className={`h-1.5 w-1.5 rounded-full ${p.id === state.activeProfile ? "bg-accent" : "bg-transparent"}`} />
                <span className="truncate">{profileName(p.id, p.name)}</span>
                {p.id === "vanilla" && <Lock size={13} className="ml-auto text-dim" aria-label={t("schreibgeschützt")} />}
              </button>
            ))}
            <div className="my-1.5 h-px bg-line" />
            <MenuItem icon={<Plus size={15} />} onClick={() => { setOpen(false); setName(""); setDialog("new"); }}>
              {t("Neues Profil")}
            </MenuItem>
            <MenuItem icon={<Pencil size={15} />} disabled={state.activeProfile === "vanilla"} onClick={() => { setOpen(false); setName(active?.name ?? ""); setDialog("rename"); }}>
              {t("Umbenennen")}
            </MenuItem>
            <MenuItem
              icon={<Trash2 size={15} />}
              disabled={state.activeProfile === "vanilla" || state.profiles.length < 3}
              onClick={async () => {
                setOpen(false);
                if (!(await confirmDialog(t("Profil „{name}“ löschen? Die Mods selbst bleiben erhalten.", { name: active?.name ?? "" }), t("Löschen")))) return;
                const s = await run(() => api.deleteProfile(state.activeProfile));
                if (s) setState(s);
              }}
            >
              {t("Profil löschen")}
            </MenuItem>
          </div>
        </>
      )}
      {dialog && (
        <Modal
          title={dialog === "new" ? t("Neues Profil") : t("Profil umbenennen")}
          onClose={() => setDialog(null)}
          width="max-w-md"
          footer={
            <>
              <Button variant="ghost" onClick={() => setDialog(null)}>{t("Abbrechen")}</Button>
              <Button variant="primary" onClick={submit} disabled={!name.trim()}>
                {dialog === "new" ? "Anlegen" : "Speichern"}
              </Button>
            </>
          }
        >
          <Input autoFocus value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && submit()} placeholder={t("z. B. Hardcore-Durchgang")} />
          {dialog === "new" && (
            <label className="mt-3 flex items-center gap-2 text-sm text-muted">
              <input type="checkbox" checked={copy} onChange={(e) => setCopy(e.target.checked)} className="accent-[var(--color-accent)]" />
              {t("Aktive Mods und Reihenfolge vom aktuellen Profil übernehmen")}
            </label>
          )}
        </Modal>
      )}
    </div>
  );
}

function MenuItem({ icon, children, onClick, disabled }: { icon: React.ReactNode; children: React.ReactNode; onClick: () => void; disabled?: boolean }) {
  return (
    <button
      role="menuitem"
      disabled={disabled}
      onClick={onClick}
      className="flex h-9 w-full items-center gap-2.5 rounded-md px-2.5 text-left text-sm text-muted hover:bg-hover hover:text-text disabled:opacity-40"
    >
      {icon}
      {children}
    </button>
  );
}

export function TopBar() {
  const { state, run, setState, toast } = useStore();
  if (!state) return null;
  const game = state.game;

  const apply = async () => {
    const r = await run(() => api.apply(), { busy: t("Mods werden angewendet …") });
    if (r) {
      setState(r.state);
      toast("success", t("Angewendet: {placed} Dateien platziert, {removed} entfernt, {kept} unverändert.", { placed: r.placed, removed: r.removed, kept: r.kept }));
    }
    return r;
  };

  const launch = async () => {
    if (state.pending) {
      const r = await apply();
      if (!r) return;
    }
    await run(() => api.launchGame(), { success: t("Stalker 2 wird gestartet …") });
  };

  return (
    <header className="flex h-16 shrink-0 items-center gap-3 border-b border-line bg-panel/60 px-5">
      <ProfileMenu />
      {game && (
        <button
          onClick={() => run(() => api.openLocation("game"))}
          className="flex min-w-0 items-center gap-2 rounded-md px-2 py-1 text-left text-xs text-muted hover:bg-hover"
          title={t("Spielordner öffnen")}
        >
          <FolderOpen size={15} className="shrink-0 text-dim" />
          <span className="min-w-0 truncate font-mono">{game.path}</span>
          <span className="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10.5px] text-dim">{t(platformLabel[game.platform])}</span>
        </button>
      )}
      <div className="ml-auto flex shrink-0 items-center gap-2">
        {state.pending && (
          <span className="mr-1 flex items-center gap-2 whitespace-nowrap text-xs text-accent">
            <span className="pulse-dot h-2 w-2 rounded-full bg-accent" />
            {t("Nicht angewendet")}
          </span>
        )}
        <Button variant={state.pending ? "primary" : "secondary"} icon={<Upload size={16} />} onClick={apply} disabled={!game?.valid}>
          {t("Anwenden")}
        </Button>
        <Button variant={state.pending ? "secondary" : "primary"} icon={<Play size={16} />} onClick={launch} disabled={!game?.valid}>
          {t("Spiel starten")}
        </Button>
      </div>
    </header>
  );
}

export function Toasts() {
  const { toasts, dismissToast } = useStore();
  const icon = { info: <Info size={17} className="text-info" />, success: <CheckCircle2 size={17} className="text-ok" />, error: <AlertTriangle size={17} className="text-danger" /> };
  return (
    <div className="pointer-events-none fixed bottom-5 right-5 z-50 flex w-96 flex-col gap-2" aria-live="polite">
      {toasts.map((item) => (
        <div key={item.id} className="toast-in pointer-events-auto flex items-start gap-3 rounded-lg border border-line-strong bg-raised px-4 py-3 shadow-xl">
          <span className="mt-0.5">{icon[item.kind]}</span>
          <p className="flex-1 whitespace-pre-line text-sm leading-snug">{item.message}</p>
          <button aria-label={t("Schließen")} onClick={() => dismissToast(item.id)} className="text-dim hover:text-text">
            <X size={15} />
          </button>
        </div>
      ))}
    </div>
  );
}

export function BusyOverlay() {
  const { busy } = useStore();
  const [show, setShow] = useState(false);
  useEffect(() => {
    if (!busy) return setShow(false);
    const timer = setTimeout(() => setShow(true), 150);
    return () => clearTimeout(timer);
  }, [busy]);
  if (!busy || !show) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="flex items-center gap-3 rounded-lg border border-line-strong bg-raised px-5 py-4 shadow-2xl">
        <Loader2 size={20} className="animate-spin text-accent" />
        <span className="text-sm">{busy}</span>
      </div>
    </div>
  );
}

export function DropOverlay({ active }: { active: boolean }) {
  if (!active) return null;
  return (
    <div className="pointer-events-none fixed inset-0 z-50 flex items-center justify-center bg-bg/80 p-10">
      <div className="flex h-full w-full flex-col items-center justify-center rounded-2xl border-2 border-dashed border-accent/70 bg-accent-soft">
        <Upload size={44} className="text-accent" />
        <p className="mt-4 font-display text-3xl font-semibold tracking-wide">{t("Zum Installieren loslassen")}</p>
        <p className="mt-2 text-sm text-muted">{t("ZIP, 7z, RAR, PAK/UTOC/UCAS, DLL oder Ordner")}</p>
      </div>
    </div>
  );
}
