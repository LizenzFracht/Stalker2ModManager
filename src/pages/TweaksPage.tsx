import { useEffect, useMemo, useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { FileDown, FileUp, FolderOpen, History, Plus, RotateCcw, Save, Trash2 } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Badge, Button, IconButton, Modal, SectionTitle, Toggle, confirmDialog } from "../components/ui";
import type { TweakState } from "../types";
import { t } from "../i18n";

type Def =
  | { key: string; label: string; desc: string; kind: "toggle"; on: string }
  | { key: string; label: string; desc: string; kind: "number"; min: number; max: number; step: number; unit?: string; suggest: number };

const CATALOG: { title: string; items: Def[] }[] = [
  {
    title: "Bildqualität",
    items: [
      { key: "r.SceneColorFringeQuality", label: "Chromatische Aberration aus", desc: "Entfernt die Farbsäume an den Bildrändern.", kind: "toggle", on: "0" },
      { key: "r.Tonemapper.Quality", label: "Vignette & Filmkörnung aus", desc: "Reduziert den Tonemapper auf Kontrast – ohne Randabdunklung und Körnung.", kind: "toggle", on: "1" },
      { key: "r.FilmGrain", label: "Filmkörnung aus", desc: "Schaltet nur die Körnung ab.", kind: "toggle", on: "0" },
      { key: "r.DepthOfFieldQuality", label: "Tiefenunschärfe aus", desc: "Kein Weichzeichnen von Hintergrund/Vordergrund.", kind: "toggle", on: "0" },
      { key: "r.MotionBlurQuality", label: "Bewegungsunschärfe aus", desc: "Erzwingt keine Bewegungsunschärfe, auch bei Kamerabewegung.", kind: "toggle", on: "0" },
      { key: "r.LensFlareQuality", label: "Lens Flares aus", desc: "Keine Linsenreflexionen bei hellen Lichtquellen.", kind: "toggle", on: "0" },
      { key: "r.Tonemapper.Sharpen", label: "Nachschärfen", desc: "Wirkt gegen ein weiches Bild bei TAA/Upscaling. 0 = aus.", kind: "number", min: 0, max: 2, step: 0.1, suggest: 0.6 },
    ],
  },
  {
    title: "Leistung & Ruckler",
    items: [
      { key: "r.Streaming.PoolSize", label: "Textur-Streaming-Pool", desc: "Videospeicher für Texturen. Etwa VRAM minus 2–3 GB.", kind: "number", min: 1000, max: 20000, step: 500, unit: "MB", suggest: 6000 },
      { key: "r.Streaming.LimitPoolSizeToVRAM", label: "Pool auf VRAM begrenzen", desc: "Verhindert, dass der Pool den Grafikspeicher überschreitet.", kind: "toggle", on: "1" },
      { key: "D3D12.PSO.DiskCache", label: "Shader-Cache auf Festplatte", desc: "Speichert kompilierte Pipelines – weniger Nachladeruckler beim nächsten Start.", kind: "toggle", on: "1" },
      { key: "D3D12.PSO.DriverOptimizedDiskCache", label: "Treiberoptimierter Shader-Cache", desc: "Ergänzt den Shader-Cache um den Treiber-Cache.", kind: "toggle", on: "1" },
    ],
  },
  {
    title: "Weitsicht",
    items: [
      { key: "r.ViewDistanceScale", label: "Sichtweiten-Faktor", desc: "Höhere Werte zeigen Objekte auf größere Distanz. Kostet Leistung.", kind: "number", min: 0.5, max: 3, step: 0.1, suggest: 1.5 },
      { key: "foliage.LODDistanceScale", label: "Vegetations-Detaildistanz", desc: "Weniger sichtbares Umschalten der Detailstufen bei Gras und Büschen.", kind: "number", min: 0.5, max: 4, step: 0.25, suggest: 2 },
      { key: "r.Shadow.MaxResolution", label: "Max. Schattenauflösung", desc: "Schärfere Schatten bei höheren Werten.", kind: "number", min: 512, max: 4096, step: 512, suggest: 2048 },
    ],
  },
];

const known = new Set(CATALOG.flatMap((c) => c.items.map((i) => i.key.toLowerCase())));

export function TweaksPage() {
  const { state, run, toast, refresh } = useStore();
  const [tw, setTw] = useState<TweakState | null>(null);
  const [draft, setDraft] = useState<Record<string, string | null>>({});
  const [custom, setCustom] = useState<{ key: string; value: string }[]>([]);
  const [importing, setImporting] = useState(false);
  const [protectionBusy, setProtectionBusy] = useState(false);

  const load = (ts: TweakState) => {
    setTw(ts);
    setDraft({ ...ts.values });
    setCustom(Object.entries(ts.values).filter(([k]) => !known.has(k.toLowerCase())).map(([key, value]) => ({ key, value })));
  };

  useEffect(() => {
    run(() => api.getTweaks()).then((ts) => ts && load(ts));
  }, [run]);

  const get = (key: string) => {
    const hit = Object.keys(draft).find((k) => k.toLowerCase() === key.toLowerCase());
    return hit ? draft[hit] : null;
  };
  const set = (key: string, value: string | null) => {
    setDraft((d) => {
      const hit = Object.keys(d).find((k) => k.toLowerCase() === key.toLowerCase()) ?? key;
      return { ...d, [hit]: value };
    });
  };

  const changes = useMemo(() => {
    if (!tw) return {};
    const out: Record<string, string | null> = {};
    const merged: Record<string, string | null> = { ...draft };
    // Custom rows replace all unknown keys.
    for (const k of Object.keys(merged)) if (!known.has(k.toLowerCase())) merged[k] = null;
    for (const c of custom) if (c.key.trim()) merged[c.key.trim()] = c.value.trim();
    for (const [k, v] of Object.entries(merged)) {
      const cur = tw.values[k] ?? null;
      if ((v ?? null) !== cur && !(v === null && cur === null)) out[k] = v;
    }
    return out;
  }, [draft, custom, tw]);
  const dirty = Object.keys(changes).length > 0;

  if (!tw || !state) return <p className="p-8 text-sm text-muted">{t("Lade Engine.ini …")}</p>;

  const save = async () => {
    const ts = await run(() => api.setTweaks(changes), { success: t("Engine.ini gespeichert – wirkt beim nächsten Spielstart.") });
    if (ts) load(ts);
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-4xl px-8 py-7">
        <div className="flex items-start gap-4">
          <div className="flex-1">
            <h1 className="font-display text-3xl font-semibold tracking-wide">{t("Spiel-Tweaks")}</h1>
            <p className="mt-2 max-w-2xl text-[13px] leading-relaxed text-muted">
              {t("Einstellungen in")} <code className="font-mono text-text">Engine.ini</code> → <code className="font-mono text-text">{t("[SystemSettings]")}</code>{t(". Ausgeschaltete Tweaks werden aus der Datei entfernt, dann gilt wieder der Spielstandard. Vor der ersten Änderung legt S2MM eine Sicherung an.")}
            </p>
          </div>
        </div>
        <div className="mt-5 flex items-center gap-4 rounded-md border border-line bg-panel px-4 py-3">
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium">{t("Engine.ini vor Überschreiben schützen")}</div>
            <p className="mt-1 text-xs leading-relaxed text-muted">{t("Schützt die Datei nach dem Speichern, damit das Spiel deine Tweaks beim Beenden nicht überschreibt. S2MM kann sie weiterhin bearbeiten.")}</p>
          </div>
          <Toggle
            label={t("Engine.ini vor Überschreiben schützen")}
            checked={state.settings.protectEngineIni}
            disabled={protectionBusy}
            onChange={async (protect) => {
              setProtectionBusy(true);
              try {
                const ts = await run(() => api.setEngineIniProtection(protect));
                if (ts) {
                  setTw(ts); // keep draft/custom edits intact when only protection changes
                } else {
                  // Permissions may have changed before persistence failed. Reconcile
                  // actual file state without hiding the error or discarding edits.
                  const actual = await run(() => api.getTweaks());
                  if (actual) setTw(actual);
                }
                await refresh();
              } finally {
                setProtectionBusy(false);
              }
            }}
          />
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2 rounded-md border border-line bg-panel px-3.5 py-2.5 text-xs">
          <span className="min-w-0 flex-1 truncate font-mono text-muted" title={tw.configDir ?? ""}>{tw.configDir ?? "Konfigurationsordner unbekannt"}</span>
          {!tw.engineIniExists && <Badge tone="accent">{t("Engine.ini wird neu angelegt")}</Badge>}
          {tw.readOnly && <Badge tone="info">{t("schreibgeschützt")}</Badge>}
          <Button size="sm" variant="ghost" icon={<FolderOpen size={14} />} onClick={() => run(() => api.openLocation("config"))}>
            {t("Öffnen")}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            icon={<FileDown size={14} />}
            disabled={!tw.engineIniExists}
            onClick={async () => {
              const path = await saveDialog({ title: "Engine.ini exportieren", defaultPath: "Engine.ini", filters: [{ name: "INI", extensions: ["ini"] }] });
              if (path) await run(() => api.exportEngineIni(path), { success: "Engine.ini exportiert." });
            }}
          >
            {t("Export")}
          </Button>
          <Button size="sm" variant="ghost" icon={<FileUp size={14} />} onClick={() => setImporting(true)}>
            {t("Import")}
          </Button>
          {tw.backupExists && (
            <Button
              size="sm"
              variant="ghost"
              icon={<History size={14} />}
              onClick={async () => {
                if (!(await confirmDialog(t("Engine.ini auf den Stand vor der ersten Änderung durch S2MM zurücksetzen?"), t("Zurücksetzen")))) return;
                const ts = await run(() => api.restoreTweaks(), { success: t("Original-Engine.ini wiederhergestellt.") });
                if (ts) load(ts);
              }}
            >
              {t("Original wiederherstellen")}
            </Button>
          )}
        </div>

        {CATALOG.map((cat) => (
          <section key={cat.title} className="mt-8">
            <SectionTitle>{t(cat.title)}</SectionTitle>
            <div className="divide-y divide-line rounded-lg border border-line bg-panel">
              {cat.items.map((d) => {
                const value = get(d.key);
                return (
                  <div key={d.key} className="flex items-center gap-4 px-4 py-3.5">
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium">{t(d.label)}</div>
                      <div className="mt-0.5 text-xs leading-relaxed text-muted">{t(d.desc)}</div>
                      <div className="mt-1 font-mono text-[10.5px] text-dim">{d.key}</div>
                    </div>
                    {d.kind === "toggle" ? (
                      <Toggle label={d.label} checked={value === d.on} onChange={(on) => set(d.key, on ? d.on : null)} />
                    ) : (
                      <div className="flex items-center gap-2">
                        {value === null ? (
                          <Button size="sm" onClick={() => set(d.key, String(d.suggest))}>{t("Setzen")}</Button>
                        ) : (
                          <>
                            <input
                              type="range"
                              min={d.min}
                              max={d.max}
                              step={d.step}
                              value={Number(value) || d.min}
                              onChange={(e) => set(d.key, e.target.value)}
                              aria-label={d.label}
                              className="w-36 accent-[var(--color-accent)]"
                            />
                            <input
                              value={value}
                              onChange={(e) => set(d.key, e.target.value)}
                              aria-label={`${d.label} Wert`}
                              className="h-8 w-20 rounded-md border border-line-strong bg-bg px-2 text-right font-mono text-xs focus:border-accent focus:outline-none"
                            />
                            {d.unit && <span className="text-xs text-dim">{d.unit}</span>}
                            <IconButton label={t("Auf Spielstandard zurücksetzen")} onClick={() => set(d.key, null)}>
                              <RotateCcw size={14} />
                            </IconButton>
                          </>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </section>
        ))}

        <section className="mt-8">
          <SectionTitle right={<Button size="sm" variant="ghost" icon={<Plus size={14} />} onClick={() => setCustom((c) => [...c, { key: "", value: "" }])}>{t("Eintrag")}</Button>}>
            {t("Eigene Einträge")}
          </SectionTitle>
          {custom.length === 0 ? (
            <p className="text-xs text-dim">{t("Weitere Konsolenvariablen (CVars) für [SystemSettings], z. B. aus Tweak-Guides.")}</p>
          ) : (
            <div className="space-y-1.5">
              {custom.map((c, i) => (
                <div key={i} className="flex items-center gap-2">
                  <input
                    value={c.key}
                    placeholder={t("r.Beispiel")}
                    aria-label={t("Name")}
                    onChange={(e) => setCustom((l) => l.map((x, j) => (j === i ? { ...x, key: e.target.value } : x)))}
                    className="h-8 flex-1 rounded-md border border-line-strong bg-bg px-2.5 font-mono text-xs focus:border-accent focus:outline-none"
                  />
                  <span className="text-dim">=</span>
                  <input
                    value={c.value}
                    placeholder={t("Wert")}
                    aria-label={t("Wert")}
                    onChange={(e) => setCustom((l) => l.map((x, j) => (j === i ? { ...x, value: e.target.value } : x)))}
                    className="h-8 w-40 rounded-md border border-line-strong bg-bg px-2.5 font-mono text-xs focus:border-accent focus:outline-none"
                  />
                  <IconButton label={t("Eintrag entfernen")} onClick={() => setCustom((l) => l.filter((_, j) => j !== i))}>
                    <Trash2 size={14} />
                  </IconButton>
                </div>
              ))}
            </div>
          )}
        </section>

        {importing && (
          <Modal
            title={t("Engine.ini importieren")}
            width="max-w-lg"
            onClose={() => setImporting(false)}
            footer={<Button variant="ghost" onClick={() => setImporting(false)}>{t("Abbrechen")}</Button>}
          >
            <p className="text-[13px] leading-relaxed text-muted">
              {t("Wähle eine .ini-Datei, z. B. aus einem Tweak-Guide oder einen früheren Export. Vor der ersten Änderung wird die aktuelle Engine.ini gesichert.")}
            </p>
            <div className="mt-4 grid gap-2">
              {[
                { replace: false, title: "Zusammenführen", desc: "Übernimmt alle Einträge aller Abschnitte, vorhandene Werte werden aktualisiert." },
                { replace: true, title: "Komplett ersetzen", desc: "Engine.ini wird durch die gewählte Datei ersetzt." },
              ].map((m) => (
                <button
                  key={m.title}
                  onClick={async () => {
                    const path = await openDialog({ title: t("ini-Datei wählen"), filters: [{ name: "INI", extensions: ["ini", "txt"] }] });
                    if (typeof path !== "string") return;
                    setImporting(false);
                    const r = await run(() => api.importEngineIni(path, m.replace));
                    if (r) {
                      load(r.state);
                      toast("success", t("{n} Einträge importiert – wirkt beim nächsten Spielstart.", { n: r.entries }));
                    }
                  }}
                  className="rounded-lg border border-line-strong bg-raised px-4 py-3 text-left hover:border-accent/60"
                >
                  <div className="text-sm font-medium">{t(m.title)}</div>
                  <div className="mt-0.5 text-xs text-muted">{t(m.desc)}</div>
                </button>
              ))}
            </div>
          </Modal>
        )}

        <div className="sticky bottom-0 mt-8 flex items-center gap-3 border-t border-line bg-bg/95 py-4 backdrop-blur">
          <span className="text-xs text-muted">{dirty ? t("{n} Änderung(en) nicht gespeichert", { n: Object.keys(changes).length }) : t("Alles gespeichert")}</span>
          <Button variant="ghost" className="ml-auto" disabled={!dirty} onClick={() => tw && load(tw)}>
            {t("Verwerfen")}
          </Button>
          <Button
            variant="primary"
            icon={<Save size={16} />}
            disabled={!dirty}
            onClick={() => {
              if (Object.keys(changes).some((k) => !k.includes("."))) toast("info", t("Hinweis: CVar-Namen enthalten üblicherweise einen Punkt (z. B. r.Name)."));
              save();
            }}
          >
            {t("In Engine.ini speichern")}
          </Button>
        </div>
      </div>
    </div>
  );
}
