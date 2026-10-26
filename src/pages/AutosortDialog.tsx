import { useEffect, useState } from "react";
import { ArrowDown, ArrowUp, Wand2 } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Button, Modal } from "../components/ui";
import type { SortEntry } from "../types";

export function AutosortDialog({ onClose }: { onClose: () => void }) {
  const { state, run, setState, toast } = useStore();
  const [entries, setEntries] = useState<SortEntry[] | null>(null);

  useEffect(() => {
    run(() => api.autosortPreview()).then((e) => (e ? setEntries(e) : onClose()));
  }, [run, onClose]);

  if (!state) return null;
  const nameOf = (id: string) => state.mods.find((m) => m.id === id)?.name ?? "?";
  const enabled = (id: string) => state.mods.find((m) => m.id === id)?.enabled ?? false;
  const changed = entries?.filter((e) => e.from !== e.to).length ?? 0;

  const accept = async () => {
    if (!entries) return;
    const s = await run(() => api.reorderMods(entries.map((e) => e.id)));
    if (s) {
      setState(s);
      toast("success", "Reihenfolge übernommen – zum Aktivieren „Anwenden“ klicken.");
      onClose();
    }
  };

  return (
    <Modal
      title="Automatisch sortieren"
      onClose={onClose}
      width="max-w-2xl"
      footer={
        <>
          <span className="mr-auto self-center text-xs text-muted">
            {entries ? (changed ? `${changed} Mod(s) werden verschoben` : "Die Reihenfolge ist bereits optimal") : ""}
          </span>
          <Button variant="ghost" onClick={onClose}>Abbrechen</Button>
          <Button variant="primary" icon={<Wand2 size={15} />} disabled={!changed} onClick={accept}>
            Übernehmen
          </Button>
        </>
      }
    >
      <p className="text-[13px] leading-relaxed text-muted">
        Grundlagen (UE4SS, MCM, Frameworks) zuerst, dann Skript-Mods, dann Pak-Mods – große Überarbeitungen vor kleinen, gezielten
        Änderungen, damit diese gewinnen. Patches und Kompatibilitäts-Mods kommen ans Ende bzw. direkt hinter die Mod, die sie
        betreffen.
      </p>
      {!entries ? (
        <p className="mt-4 text-sm text-muted">Analysiere Mods …</p>
      ) : (
        <ol className="mt-4 divide-y divide-line rounded-lg border border-line">
          {entries.map((e) => {
            const delta = e.from - e.to;
            return (
              <li key={e.id} className={`flex items-center gap-3 px-3.5 py-2 ${enabled(e.id) ? "" : "opacity-50"}`}>
                <span className="w-6 text-right font-mono text-xs text-dim">{String(e.to + 1).padStart(2, "0")}</span>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm">{nameOf(e.id)}</div>
                  <div className="truncate text-xs text-dim">{e.reason}</div>
                </div>
                {delta !== 0 && (
                  <span className={`flex items-center gap-0.5 font-mono text-xs ${delta > 0 ? "text-ok" : "text-accent"}`}>
                    {delta > 0 ? <ArrowUp size={13} /> : <ArrowDown size={13} />}
                    {Math.abs(delta)}
                  </span>
                )}
              </li>
            );
          })}
        </ol>
      )}
    </Modal>
  );
}
