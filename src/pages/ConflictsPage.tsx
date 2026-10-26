import { useEffect, useState } from "react";
import { ArrowRightLeft, ChevronRight, ShieldCheck, Swords } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Button, Empty, SectionTitle } from "../components/ui";
import type { ConflictPair, ConflictReport } from "../types";

export function ConflictsPage() {
  const { state, run, setState, setPage, setSelectedMod } = useStore();
  const [report, setReport] = useState<ConflictReport | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const key = state?.mods.map((m) => `${m.id}:${m.enabled}:${m.updatedAt}:${JSON.stringify(m.selection)}`).join("|");

  useEffect(() => {
    run(() => api.getConflicts()).then((r) => r && setReport(r));
  }, [key, run]);

  if (!state) return null;
  const nameOf = (id: string) => state.mods.find((m) => m.id === id)?.name ?? "?";
  const prio = (id: string) => state.mods.find((m) => m.id === id)?.priority ?? 0;

  const swap = async (p: ConflictPair) => {
    // Move the loser directly below the winner so it wins instead.
    const ids = state.mods.map((m) => m.id).filter((id) => id !== p.loser);
    ids.splice(ids.indexOf(p.winner) + 1, 0, p.loser);
    const s = await run(() => api.reorderMods(ids));
    if (s) setState(s);
  };

  const open = (id: string) => {
    setSelectedMod(id);
    setPage("mods");
  };

  const pairs = report?.pairs ?? [];
  const involved = new Set(pairs.flatMap((p) => [p.winner, p.loser])).size;

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-4xl px-8 py-7">
        <h1 className="font-display text-3xl font-semibold tracking-wide">Konflikte</h1>
        <p className="mt-2 max-w-2xl text-[13px] leading-relaxed text-muted">
          S2MM liest das Inhaltsverzeichnis jeder aktiven .pak/.utoc-Datei. Ändern zwei Mods dieselbe Datei (z. B. dieselbe .cfg), gilt
          nur die Version der Mod, die weiter unten in der Ladereihenfolge steht. Bei .cfg-Dateien heißt das meist: Die Änderungen der
          oberen Mod sind komplett wirkungslos.
        </p>

        {report === null ? (
          <p className="mt-8 text-sm text-muted">Analysiere Pak-Dateien …</p>
        ) : pairs.length === 0 && report.errors.length === 0 ? (
          <Empty icon={<ShieldCheck size={44} className="text-ok" />} title="Keine Konflikte">
            Keine aktive Mod überschreibt Dateien einer anderen.
          </Empty>
        ) : (
          <>
            {pairs.length > 0 && (
              <div className="mt-6 flex items-center gap-2 text-sm">
                <Swords size={16} className="text-accent" />
                <span>
                  {pairs.length} Überschneidung{pairs.length === 1 ? "" : "en"} zwischen {involved} Mods
                </span>
              </div>
            )}
            <ul className="mt-4 space-y-2">
              {pairs.map((p) => {
                const id = `${p.winner}-${p.loser}-${p.kind}`;
                const isOpen = expanded === id;
                return (
                  <li key={id} className="rounded-lg border border-line bg-panel">
                    <div className="flex items-center gap-3 px-4 py-3">
                      <button
                        onClick={() => setExpanded(isOpen ? null : id)}
                        aria-expanded={isOpen}
                        aria-label="Dateien anzeigen"
                        className="text-dim hover:text-text"
                      >
                        <ChevronRight size={16} className={`transition-transform ${isOpen ? "rotate-90" : ""}`} />
                      </button>
                      <div className="min-w-0 flex-1 text-sm">
                        <button onClick={() => open(p.winner)} className="font-medium text-ok hover:underline">
                          {nameOf(p.winner)}
                        </button>
                        <span className="font-mono text-[11px] text-dim"> #{prio(p.winner)}</span>
                        <span className="text-muted"> überschreibt </span>
                        <button onClick={() => open(p.loser)} className="font-medium text-danger hover:underline">
                          {nameOf(p.loser)}
                        </button>
                        <span className="font-mono text-[11px] text-dim"> #{prio(p.loser)}</span>
                        <div className="mt-0.5 text-xs text-dim">
                          {p.count} {p.kind === "asset" ? (p.count === 1 ? "Datei in Paks" : "Dateien in Paks") : p.count === 1 ? "Datei im Spielordner" : "Dateien im Spielordner"}
                        </div>
                      </div>
                      <Button size="sm" variant="ghost" icon={<ArrowRightLeft size={14} />} onClick={() => swap(p)} title="Verlierer direkt unter den Gewinner schieben">
                        Umkehren
                      </Button>
                    </div>
                    {isOpen && (
                      <div className="max-h-80 overflow-y-auto border-t border-line px-5 py-3">
                        {p.items.map((it) => (
                          <div key={it} className="truncate font-mono text-[11.5px] leading-5 text-muted" title={it}>
                            {it}
                          </div>
                        ))}
                        {p.count > p.items.length && <div className="mt-1 text-xs text-dim">… und {p.count - p.items.length} weitere</div>}
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>

            {report.errors.length > 0 && (
              <div className="mt-10">
                <SectionTitle>Nicht prüfbar</SectionTitle>
                <ul className="space-y-1.5">
                  {report.errors.map((e, i) => (
                    <li key={i} className="rounded-md border border-line bg-panel px-4 py-2.5 text-[13px]">
                      <span className="font-medium">{nameOf(e.modId)}</span>
                      <span className="ml-2 font-mono text-xs text-muted">{e.file}</span>
                      <div className="mt-0.5 text-xs text-danger">{e.error}</div>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
