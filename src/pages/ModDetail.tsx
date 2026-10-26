import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ArrowDownCircle,
  ArrowUpCircle,
  ChevronRight,
  ExternalLink,
  FileCode2,
  FolderOpen,
  ListChecks,
  Pencil,
  RefreshCcw,
  Save,
  Settings2,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { api, nexusModUrl } from "../api";
import { useStore } from "../store";
import { Badge, Button, Field, IconButton, Input, Modal, Toggle, confirmDialog, formatBytes, formatDate, kindLabel } from "../components/ui";
import type { ConflictReport, ModDetails, ModView } from "../types";
import { ModThumb } from "./ModsPage";

type Tab = "overview" | "files" | "config";

export function ModDetail({ mod }: { mod: ModView }) {
  const { setSelectedMod } = useStore();
  const [tab, setTab] = useState<Tab>("overview");
  const tabs: { id: Tab; label: string }[] = [
    { id: "overview", label: "Übersicht" },
    { id: "files", label: "Dateien" },
    { id: "config", label: "Konfiguration" },
  ];
  return (
    <aside className="flex w-[440px] shrink-0 flex-col border-l border-line bg-panel">
      <div className="flex items-start gap-3 px-5 pb-3 pt-4">
        <ModThumb mod={mod} size={52} />
        <div className="min-w-0 flex-1">
          <h2 className="font-display text-[22px] font-semibold leading-tight tracking-wide">{mod.name}</h2>
          <div className="mt-0.5 text-xs text-muted">
            {mod.version && <span className="font-mono">v{mod.version.replace(/^v/i, "")}</span>}
            {mod.version && mod.author && " · "}
            {mod.author}
          </div>
        </div>
        <IconButton label="Details schließen" onClick={() => setSelectedMod(null)}>
          <X size={17} />
        </IconButton>
      </div>
      <div role="tablist" className="flex gap-1 border-b border-line px-4">
        {tabs.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={tab === t.id}
            onClick={() => setTab(t.id)}
            className={`-mb-px border-b-2 px-2.5 pb-2.5 pt-1 text-sm transition-colors ${
              tab === t.id ? "border-accent text-text" : "border-transparent text-muted hover:text-text"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {tab === "overview" && <Overview mod={mod} />}
        {tab === "files" && <Files mod={mod} />}
        {tab === "config" && <Config mod={mod} />}
      </div>
    </aside>
  );
}

function Overview({ mod }: { mod: ModView }) {
  const { state, run, setState, setOptionsFor, setSelectedMod, toast } = useStore();
  const [notes, setNotes] = useState(mod.notes);
  const [editing, setEditing] = useState(false);
  const [report, setReport] = useState<ConflictReport | null>(null);

  useEffect(() => {
    if (!mod.enabled || (mod.conflicts.wins === 0 && mod.conflicts.loses === 0)) return setReport(null);
    api.getConflicts().then(setReport).catch(() => setReport(null));
  }, [mod.id, mod.enabled, mod.conflicts.wins, mod.conflicts.loses, mod.priority]);

  const nameOf = (id: string) => state?.mods.find((m) => m.id === id)?.name ?? "?";
  const pairs = report?.pairs.filter((p) => p.winner === mod.id || p.loser === mod.id) ?? [];

  const remove = async () => {
    if (!(await confirmDialog(`„${mod.name}“ aus S2MM entfernen? Die Dateien der Mod werden gelöscht. Klicke danach auf „Anwenden“, um sie auch aus dem Spiel zu entfernen.`, "Entfernen"))) return;
    const s = await run(() => api.removeMod(mod.id));
    if (s) {
      setState(s);
      setSelectedMod(null);
    }
  };

  const reinstall = async () => {
    const file = await open({
      title: `Neue Version von „${mod.name}“ wählen`,
      filters: [{ name: "Mods", extensions: ["zip", "7z", "rar", "pak", "utoc", "ucas", "dll", "asi"] }],
    });
    if (typeof file !== "string") return;
    const s = await run(() => api.reinstallMod(mod.id, file), { busy: "Mod wird ersetzt …", success: "Mod ersetzt – Optionen und Reihenfolge wurden beibehalten." });
    if (s) setState(s);
  };

  const selectedOptions = mod.layout.groups.map((g) => {
    const chosen = mod.selection[g.id];
    const names = g.options
      .filter((o) => (chosen ? chosen.includes(o.id) : o.defaultSelected))
      .map((o) => o.name);
    return { group: g.name, names };
  });

  return (
    <div className="space-y-5 px-5 py-4">
      <div className="flex flex-wrap items-center gap-2">
        <label className="mr-1 flex items-center gap-2 text-sm">
          <Toggle
            checked={mod.enabled}
            label="Aktiv"
            onChange={async (v) => {
              const s = await run(() => api.setEnabled([mod.id], v));
              if (s) setState(s);
            }}
          />
          {mod.enabled ? "Aktiv" : "Inaktiv"}
        </label>
        {mod.layout.groups.length > 0 && (
          <Button size="sm" variant={mod.needsSetup ? "primary" : "secondary"} icon={<Settings2 size={14} />} onClick={() => setOptionsFor(mod.id)}>
            Optionen
          </Button>
        )}
        <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => run(() => api.openLocation(`mod:${mod.id}`))}>
          Ordner
        </Button>
        {mod.nexus && (
          <Button size="sm" icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl(nexusModUrl(mod.nexus!.modId)))}>
            Nexus
          </Button>
        )}
        <IconButton label="Bearbeiten" onClick={() => setEditing(true)}>
          <Pencil size={15} />
        </IconButton>
      </div>

      {mod.update?.available && (
        <div className="flex items-center gap-3 rounded-md border border-info/30 bg-info-soft px-3.5 py-3">
          <Sparkles size={17} className="text-info" />
          <div className="flex-1 text-[13px]">
            Update verfügbar{mod.update.latestVersion && <span className="font-mono"> · v{mod.update.latestVersion}</span>}
          </div>
          <Button
            size="sm"
            onClick={async () => {
              const premium = state?.nexusUser?.isPremium && mod.update?.latestFileId;
              await run(() => api.nexusUpdateMod(mod.id));
              toast("info", premium ? "Update wird heruntergeladen …" : "Nexus geöffnet – nutze dort „Mod Manager Download“ für die neue Datei.");
            }}
          >
            Aktualisieren
          </Button>
        </div>
      )}

      {mod.summary && <p className="text-[13px] leading-relaxed text-muted">{mod.summary}</p>}

      <dl className="grid grid-cols-[110px_1fr] gap-x-3 gap-y-2 text-[13px]">
        <dt className="text-dim">Typ</dt>
        <dd className="flex flex-wrap gap-1">
          {mod.plan.kinds.length ? mod.plan.kinds.map((k) => <Badge key={k} title={kindLabel[k].title}>{kindLabel[k].label}</Badge>) : <span className="text-danger">Nichts Installierbares gefunden</span>}
        </dd>
        <dt className="text-dim">Priorität</dt>
        <dd className="font-mono">{mod.priority}</dd>
        <dt className="text-dim">Dateien</dt>
        <dd>
          {mod.plan.files}
          {mod.plan.unmapped.length > 0 && <span className="text-dim"> · {mod.plan.unmapped.length} nicht installiert</span>}
        </dd>
        {mod.category && (
          <>
            <dt className="text-dim">Kategorie</dt>
            <dd>{mod.category}</dd>
          </>
        )}
        <dt className="text-dim">Installiert</dt>
        <dd>{formatDate(mod.installedAt)}</dd>
        {mod.updatedAt !== mod.installedAt && (
          <>
            <dt className="text-dim">Aktualisiert</dt>
            <dd>{formatDate(mod.updatedAt)}</dd>
          </>
        )}
        {mod.archiveName && (
          <>
            <dt className="text-dim">Quelle</dt>
            <dd className="truncate font-mono text-xs" title={mod.archiveName}>{mod.archiveName}</dd>
          </>
        )}
        {mod.nexus && (
          <>
            <dt className="text-dim">Nexus-ID</dt>
            <dd className="font-mono text-xs">
              {mod.nexus.modId}
              {mod.nexus.fileId && <span className="text-dim"> / Datei {mod.nexus.fileId}</span>}
            </dd>
          </>
        )}
      </dl>

      {selectedOptions.length > 0 && (
        <div>
          <h3 className="label-caps mb-2 text-xs text-dim">Gewählte Optionen</h3>
          <ul className="space-y-1.5 text-[13px]">
            {selectedOptions.map((o, i) => (
              <li key={i} className="flex gap-2">
                <span className="text-dim">{o.group}:</span>
                <span>{o.names.length ? o.names.join(", ") : <em className="text-dim">keine</em>}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {pairs.length > 0 && (
        <div>
          <h3 className="label-caps mb-2 text-xs text-dim">Konflikte</h3>
          <ul className="space-y-1.5 text-[13px]">
            {pairs.map((p) => {
              const wins = p.winner === mod.id;
              return (
                <li key={`${p.winner}-${p.loser}-${p.kind}`} className="flex items-center gap-2">
                  {wins ? <ArrowUpCircle size={14} className="shrink-0 text-ok" /> : <ArrowDownCircle size={14} className="shrink-0 text-danger" />}
                  <span className="min-w-0 truncate">
                    {wins ? "Überschreibt" : "Überschrieben von"} <strong className="font-medium">{nameOf(wins ? p.loser : p.winner)}</strong>
                  </span>
                  <span className="ml-auto shrink-0 font-mono text-xs text-dim">{p.count}</span>
                </li>
              );
            })}
          </ul>
        </div>
      )}

      <div>
        <h3 className="label-caps mb-2 text-xs text-dim">Notizen</h3>
        <textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          onBlur={async () => {
            if (notes === mod.notes) return;
            const s = await run(() => api.updateMod(mod.id, { notes }));
            if (s) setState(s);
          }}
          rows={3}
          placeholder="Eigene Notizen zu dieser Mod …"
          className="w-full resize-y rounded-md border border-line-strong bg-bg px-3 py-2 text-[13px] placeholder:text-dim focus:border-accent focus:outline-none"
        />
      </div>

      <div className="flex gap-2 border-t border-line pt-4">
        <Button size="sm" icon={<RefreshCcw size={14} />} onClick={reinstall}>
          Neue Version einspielen
        </Button>
        <Button size="sm" variant="danger" icon={<Trash2 size={14} />} onClick={remove} className="ml-auto">
          Entfernen
        </Button>
      </div>

      {editing && <EditDialog mod={mod} onClose={() => setEditing(false)} />}
    </div>
  );
}

function EditDialog({ mod, onClose }: { mod: ModView; onClose: () => void }) {
  const { run, setState } = useStore();
  const [name, setName] = useState(mod.name);
  const [version, setVersion] = useState(mod.version ?? "");
  const [category, setCategory] = useState(mod.category ?? "");
  const [nexusId, setNexusId] = useState(mod.nexus ? String(mod.nexus.modId) : "");
  const save = async () => {
    const id = parseInt(nexusId, 10);
    const s = await run(() =>
      api.updateMod(mod.id, {
        name,
        version,
        category,
        nexusModId: nexusId.trim() === "" ? 0 : Number.isFinite(id) && id !== mod.nexus?.modId ? id : undefined,
      }),
    );
    if (s) {
      setState(s);
      onClose();
    }
  };
  return (
    <Modal
      title="Mod bearbeiten"
      onClose={onClose}
      width="max-w-lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>Abbrechen</Button>
          <Button variant="primary" onClick={save}>Speichern</Button>
        </>
      }
    >
      <div className="space-y-4">
        <Field label="Name"><Input value={name} onChange={(e) => setName(e.target.value)} /></Field>
        <div className="grid grid-cols-2 gap-3">
          <Field label="Version"><Input value={version} onChange={(e) => setVersion(e.target.value)} /></Field>
          <Field label="Kategorie"><Input value={category} onChange={(e) => setCategory(e.target.value)} placeholder="z. B. Waffen" /></Field>
        </div>
        <Field label="Nexus-Mod-ID" hint="Die Zahl aus der Nexus-Adresse (…/mods/1234). Damit kann S2MM nach Updates suchen.">
          <Input value={nexusId} onChange={(e) => setNexusId(e.target.value.replace(/\D/g, ""))} inputMode="numeric" />
        </Field>
      </div>
    </Modal>
  );
}

function useDetails(id: string) {
  const { run, state } = useStore();
  const [details, setDetails] = useState<ModDetails | null>(null);
  const mod = state?.mods.find((m) => m.id === id);
  // Re-read when options or order change.
  const key = `${JSON.stringify(mod?.selection)}|${mod?.priority}|${mod?.updatedAt}`;
  useEffect(() => {
    run(() => api.modDetails(id)).then((d) => d && setDetails(d));
  }, [id, run, key]);
  return details;
}

function Files({ mod }: { mod: ModView }) {
  const details = useDetails(mod.id);
  const [openPak, setOpenPak] = useState<string | null>(null);
  if (!details) return <p className="px-5 py-4 text-sm text-muted">Lade …</p>;
  return (
    <div className="space-y-5 px-5 py-4">
      <p className="text-xs text-dim">Größe im Mod-Speicher: {formatBytes(details.size)}</p>
      <div>
        <h3 className="label-caps mb-2 text-xs text-dim">Wird installiert nach ({details.files.length})</h3>
        <ul className="space-y-1">
          {details.files.map((f) => (
            <li key={f.target} className="rounded-md bg-bg px-2.5 py-1.5">
              <div className="flex items-center gap-2">
                <Badge>{kindLabel[f.kind].label}</Badge>
                <span className="min-w-0 flex-1 truncate font-mono text-[11.5px]" title={f.target}>{f.target}</span>
                <span className="shrink-0 font-mono text-[11px] text-dim">{formatBytes(f.size)}</span>
              </div>
              <div className="mt-0.5 truncate pl-1 font-mono text-[10.5px] text-dim" title={f.logical}>← {f.logical}</div>
            </li>
          ))}
        </ul>
      </div>
      {details.unmapped.length > 0 && (
        <div>
          <h3 className="label-caps mb-2 text-xs text-dim">Nicht installiert ({details.unmapped.length})</h3>
          <p className="mb-2 text-xs leading-relaxed text-muted">
            Dokumentation, Bilder oder Dateien, deren Ziel S2MM nicht erkennen konnte. Lose .cfg-Dateien funktionieren in Stalker 2 nur in einer .pak.
          </p>
          <ul className="space-y-0.5">
            {details.unmapped.map((u) => (
              <li key={u} className="truncate font-mono text-[11.5px] text-muted" title={u}>{u}</li>
            ))}
          </ul>
        </div>
      )}
      {details.paks.length > 0 && (
        <div>
          <h3 className="label-caps mb-2 text-xs text-dim">Pak-Inhalt</h3>
          {details.paks.map((p) => (
            <div key={p.file} className="mb-1.5 rounded-md border border-line">
              <button
                onClick={() => setOpenPak(openPak === p.file ? null : p.file)}
                className="flex w-full items-center gap-2 px-2.5 py-2 text-left"
                aria-expanded={openPak === p.file}
              >
                <ChevronRight size={14} className={`text-dim transition-transform ${openPak === p.file ? "rotate-90" : ""}`} />
                <span className="min-w-0 flex-1 truncate font-mono text-xs">{p.file}</span>
                <span className="font-mono text-[11px] text-dim">{p.error ? "Fehler" : `${p.assets.length} Assets`}</span>
              </button>
              {openPak === p.file && (
                <div className="max-h-72 overflow-y-auto border-t border-line px-3 py-2">
                  {p.error && <p className="text-xs text-danger">{p.error}</p>}
                  {p.assets.map((a) => (
                    <div key={a} className="truncate font-mono text-[11px] leading-5 text-muted" title={a}>{a}</div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function Config({ mod }: { mod: ModView }) {
  const { run, setState, toast, setMcmFilter, setPage } = useStore();
  const details = useDetails(mod.id);
  const [file, setFile] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [original, setOriginal] = useState("");

  useEffect(() => {
    if (!file) return;
    run(() => api.readModFile(mod.id, file)).then((t) => {
      if (t !== undefined) {
        setText(t);
        setOriginal(t);
      }
    });
  }, [file, mod.id, run]);

  const mcmLink = (
    <div className="mb-4 flex items-center gap-3 rounded-md border border-line bg-bg px-3 py-2.5 text-[13px]">
      <ListChecks size={16} className="shrink-0 text-accent" />
      <span className="flex-1 text-muted">Nutzt die Mod das Mod Configuration Menu? Ihre Werte lassen sich hier bearbeiten.</span>
      <Button
        size="sm"
        onClick={() => {
          setMcmFilter(mod.name.split(/[\s-]+/)[0]);
          setPage("mcm");
        }}
      >
        MCM-Einstellungen
      </Button>
    </div>
  );
  if (!details) return <p className="px-5 py-4 text-sm text-muted">Lade …</p>;
  if (details.configFiles.length === 0) {
    return (
      <div className="px-5 py-4">
        {mcmLink}
        <div className="text-[13px] leading-relaxed text-muted">
          <FileCode2 size={28} className="mb-3 text-dim" />
          Diese Mod enthält keine bearbeitbaren Konfigurationsdateien (z. B. .ini, .lua, .json). Pak-Mods werden über ihre Optionen
          oder durch eine andere Variante angepasst.
        </div>
      </div>
    );
  }
  const dirty = text !== original;
  return (
    <div className="flex h-full flex-col px-5 py-4">
      {mcmLink}
      <select
        value={file ?? ""}
        onChange={(e) => setFile(e.target.value || null)}
        aria-label="Konfigurationsdatei"
        className="h-9 w-full rounded-md border border-line-strong bg-bg px-2 font-mono text-xs focus:border-accent focus:outline-none"
      >
        <option value="">Datei wählen …</option>
        {details.configFiles.map((f) => (
          <option key={f} value={f}>{f}</option>
        ))}
      </select>
      {file && (
        <>
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            spellCheck={false}
            aria-label="Dateiinhalt"
            className="mt-3 min-h-80 flex-1 resize-none rounded-md border border-line-strong bg-bg p-3 font-mono text-[12px] leading-5 focus:border-accent focus:outline-none"
          />
          <div className="mt-3 flex items-center gap-2">
            <span className="text-xs text-dim">{dirty ? "Ungespeicherte Änderungen" : "Gespeichert"}</span>
            <Button size="sm" variant="ghost" className="ml-auto" disabled={!dirty} onClick={() => setText(original)}>
              Verwerfen
            </Button>
            <Button
              size="sm"
              variant="primary"
              icon={<Save size={14} />}
              disabled={!dirty}
              onClick={async () => {
                const s = await run(() => api.writeModFile(mod.id, file, text));
                if (s) {
                  setState(s);
                  setOriginal(text);
                  toast("success", s.pending ? "Gespeichert – zum Übernehmen „Anwenden“ klicken." : "Gespeichert.");
                }
              }}
            >
              Speichern
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
