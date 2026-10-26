import { useEffect, useMemo, useState } from "react";
import { ExternalLink, FileWarning, History, ListChecks, Pencil, RefreshCw, Save, Search } from "lucide-react";
import { api, NEXUS_GAME_URL } from "../api";
import { useStore } from "../store";
import { KeyCapture, KeyChip } from "../components/KeyCapture";
import { Button, Empty, IconButton, Toggle, confirmDialog, formatDate } from "../components/ui";
import type { SaveFileInfo, SaveLeaf, SaveView } from "../types";

type Draft = Record<string, boolean | number | string>;

function groupKey(l: SaveLeaf) {
  return l.path.filter((p) => p !== "").join(" › ") || "Allgemein";
}

export function McmPage() {
  const { state, run, setState, mcmFilter, setMcmFilter, setPage, toast } = useStore();
  const [files, setFiles] = useState<SaveFileInfo[] | null>(null);
  const [dir, setDir] = useState<string | null>(null);
  const [current, setCurrent] = useState<string | null>(null);
  const [view, setView] = useState<SaveView | null>(null);
  const [draft, setDraft] = useState<Draft>({});
  const [keyLeaf, setKeyLeaf] = useState<SaveLeaf | null>(null);
  const [showText, setShowText] = useState(false);

  const loadList = async () => {
    const r = await run(() => api.listSaves());
    if (!r) return;
    setFiles(r.files);
    setDir(r.dir);
    if (!current && r.files.length) setCurrent(r.files[0].path);
  };
  useEffect(() => {
    loadList();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => {
    if (!current) return;
    setView(null);
    setDraft({});
    run(() => api.readSave(current)).then((v) => v && setView(v));
  }, [current, run]);

  const visible = useMemo(() => {
    const q = mcmFilter.trim().toLowerCase();
    const leaves = (view?.leaves ?? []).filter((l) => showText || l.kind !== "string" || l.label.toLowerCase().includes("value"));
    const map = new Map<string, SaveLeaf[]>();
    for (const l of leaves) {
      const g = groupKey(l);
      if (q && !`${g} ${l.label}`.toLowerCase().includes(q)) continue;
      map.set(g, [...(map.get(g) ?? []), l]);
    }
    return [...map];
  }, [view, mcmFilter, showText]);

  if (!state) return null;
  const mcm = state.tools.mcm;
  const value = (l: SaveLeaf) => (l.pointer in draft ? draft[l.pointer] : l.value);
  const dirty = Object.keys(draft).length;

  const save = async () => {
    if (!current) return;
    const edits = Object.entries(draft).map(([pointer, v]) => ({ pointer, value: v }));
    const v = await run(() => api.writeSave(current, edits), { success: "Einstellungen gespeichert – wirken beim nächsten Spielstart." });
    if (v) {
      setView(v);
      setDraft({});
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-4xl px-8 py-7">
        <h1 className="font-display text-3xl font-semibold tracking-wide">Mod-Einstellungen</h1>
        <p className="mt-2 max-w-2xl text-[13px] leading-relaxed text-muted">
          Einstellungen aus dem <strong className="text-text">Mod Configuration Menu (MCM)</strong> und anderen Mods, die ihre Werte als
          SaveGame speichern – bearbeitbar ohne das Spiel zu starten. Vor der ersten Änderung wird eine Sicherung angelegt.
        </p>

        <div className="mt-5 flex flex-wrap items-center gap-3 rounded-lg border border-line bg-panel px-4 py-3">
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium">Mod Configuration Menu</div>
            <div className="text-xs text-muted">
              {mcm.modId ? (mcm.enabled ? "Installiert und aktiv – Menü im Spiel mit B öffnen." : "Installiert, aber deaktiviert.") : "Nicht in S2MM installiert."}
            </div>
          </div>
          {mcm.modId ? (
            <Toggle
              label="MCM aktiv"
              checked={mcm.enabled}
              onChange={async (v) => {
                const s = await run(() => api.setToolEnabled("mcm", v));
                if (s) {
                  setState(s);
                  toast("info", "Zum Übernehmen ins Spiel „Anwenden“ klicken.");
                }
              }}
            />
          ) : (
            <Button
              size="sm"
              icon={<ExternalLink size={14} />}
              onClick={() => run(() => api.openUrl(`${NEXUS_GAME_URL}/search/?gsearch=Mod%20Configuration%20Menu`))}
            >
              Auf Nexus suchen
            </Button>
          )}
        </div>

        {files === null ? (
          <p className="mt-8 text-sm text-muted">Lade …</p>
        ) : files.length === 0 ? (
          <Empty icon={<ListChecks size={44} />} title="Noch keine Mod-Einstellungen">
            MCM legt <code className="font-mono">MCM_Settings.sav</code> an, sobald du im Spiel einmal Einstellungen änderst.
            <br />
            <span className="font-mono text-[11px] text-dim">{dir}</span>
          </Empty>
        ) : (
          <>
            <div className="mt-6 flex flex-wrap items-center gap-2">
              <select
                value={current ?? ""}
                onChange={(e) => setCurrent(e.target.value)}
                aria-label="Einstellungsdatei"
                className="h-9 rounded-md border border-line-strong bg-bg px-2.5 text-sm focus:border-accent focus:outline-none"
              >
                {files.map((f) => (
                  <option key={f.path} value={f.path}>
                    {f.name} · {formatDate(f.modified)}
                  </option>
                ))}
              </select>
              <IconButton label="Neu laden" onClick={() => current && run(() => api.readSave(current)).then((v) => v && (setView(v), setDraft({})))}>
                <RefreshCw size={15} />
              </IconButton>
              <div className="relative ml-auto w-64">
                <Search size={15} className="pointer-events-none absolute left-3 top-2.5 text-dim" />
                <input
                  value={mcmFilter}
                  onChange={(e) => setMcmFilter(e.target.value)}
                  placeholder="Mod oder Einstellung"
                  aria-label="Einstellungen filtern"
                  className="h-9 w-full rounded-md border border-line-strong bg-bg pl-9 pr-3 text-sm placeholder:text-dim focus:border-accent focus:outline-none"
                />
              </div>
              <label className="flex items-center gap-2 text-xs text-muted">
                <input type="checkbox" checked={showText} onChange={(e) => setShowText(e.target.checked)} className="accent-[var(--color-accent)]" />
                Textfelder zeigen
              </label>
            </div>

            {!view ? (
              <p className="mt-6 text-sm text-muted">Lese SaveGame …</p>
            ) : visible.length === 0 ? (
              <Empty icon={<FileWarning size={40} />} title="Keine passenden Einträge" />
            ) : (
              <div className="mt-5 space-y-5">
                {visible.map(([group, leaves]) => (
                  <section key={group}>
                    <h2 className="mb-2 truncate text-sm font-medium text-accent" title={group}>{group}</h2>
                    <div className="divide-y divide-line rounded-lg border border-line bg-panel">
                      {leaves.map((l) => {
                        const v = value(l);
                        const changed = l.pointer in draft;
                        return (
                          <div key={l.pointer} className="flex items-center gap-4 px-4 py-2.5">
                            <div className="min-w-0 flex-1 text-sm">
                              {l.label}
                              {changed && <span className="ml-2 text-xs text-accent">geändert</span>}
                            </div>
                            {l.kind === "bool" && (
                              <Toggle label={l.label} checked={Boolean(v)} onChange={(nv) => setDraft((d) => ({ ...d, [l.pointer]: nv }))} />
                            )}
                            {(l.kind === "int" || l.kind === "float") && (
                              <input
                                type="number"
                                step={l.kind === "int" ? 1 : "any"}
                                value={String(v)}
                                aria-label={l.label}
                                onChange={(e) => setDraft((d) => ({ ...d, [l.pointer]: e.target.value === "" ? 0 : Number(e.target.value) }))}
                                className="h-8 w-32 rounded-md border border-line-strong bg-bg px-2 text-right font-mono text-xs focus:border-accent focus:outline-none"
                              />
                            )}
                            {l.kind === "string" && (
                              <input
                                value={String(v)}
                                aria-label={l.label}
                                onChange={(e) => setDraft((d) => ({ ...d, [l.pointer]: e.target.value }))}
                                className="h-8 w-56 rounded-md border border-line-strong bg-bg px-2 font-mono text-xs focus:border-accent focus:outline-none"
                              />
                            )}
                            {l.kind === "key" && (
                              <>
                                <KeyChip k={String(v)} />
                                <IconButton label="Taste ändern" onClick={() => setKeyLeaf(l)}>
                                  <Pencil size={14} />
                                </IconButton>
                              </>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  </section>
                ))}
              </div>
            )}

            <div className="sticky bottom-0 mt-8 flex items-center gap-3 border-t border-line bg-bg/95 py-4 backdrop-blur">
              <span className="text-xs text-muted">{dirty ? `${dirty} Änderung(en) nicht gespeichert` : "Alles gespeichert"}</span>
              <Button
                variant="ghost"
                size="sm"
                icon={<History size={14} />}
                onClick={async () => {
                  if (!current || !(await confirmDialog("Die Einstellungsdatei auf den Stand vor der ersten Änderung durch S2MM zurücksetzen?", "Zurücksetzen"))) return;
                  const v = await run(() => api.restoreSave(current), { success: "Original wiederhergestellt." });
                  if (v) {
                    setView(v);
                    setDraft({});
                  }
                }}
              >
                Original wiederherstellen
              </Button>
              <Button variant="ghost" className="ml-auto" disabled={!dirty} onClick={() => setDraft({})}>
                Verwerfen
              </Button>
              <Button variant="primary" icon={<Save size={16} />} disabled={!dirty} onClick={save}>
                Speichern
              </Button>
            </div>
          </>
        )}
        <p className="mt-4 text-xs text-dim">
          Tipp: Tasten aus MCM erscheinen auch unter{" "}
          <button className="text-accent hover:underline" onClick={() => setPage("keybinds")}>Tastenbelegung</button>.
        </p>
      </div>
      {keyLeaf && (
        <KeyCapture
          title={keyLeaf.label}
          source="mcm"
          style="fkey"
          allowModifiers={false}
          onClose={() => setKeyLeaf(null)}
          onDone={(k) => {
            setDraft((d) => ({ ...d, [keyLeaf.pointer]: k }));
            setKeyLeaf(null);
          }}
        />
      )}
    </div>
  );
}
