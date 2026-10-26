import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Keyboard, Pencil, Search } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { comboId } from "../keys";
import { KeyCapture, KeyChip } from "../components/KeyCapture";
import { Badge, Empty, IconButton } from "../components/ui";
import type { Binding } from "../types";

const sourceLabel: Record<Binding["source"], { label: string; tone: "neutral" | "info" | "accent" | "ok" }> = {
  ue4ss: { label: "UE4SS", tone: "info" },
  config: { label: "Config", tone: "neutral" },
  mcm: { label: "MCM", tone: "accent" },
  game: { label: "Spiel", tone: "ok" },
};

export function KeybindsPage() {
  const { run, refresh, toast } = useStore();
  const [list, setList] = useState<Binding[] | null>(null);
  const [query, setQuery] = useState("");
  const [onlyActive, setOnlyActive] = useState(true);
  const [editing, setEditing] = useState<Binding | null>(null);

  useEffect(() => {
    run(() => api.getKeybinds()).then((l) => l && setList(l));
  }, [run]);

  // Same combination used by more than one active binding.
  const conflicts = useMemo(() => {
    const map = new Map<string, Binding[]>();
    for (const b of list ?? []) {
      if (!b.enabled) continue;
      const id = comboId(b.key, b.modifiers);
      map.set(id, [...(map.get(id) ?? []), b]);
    }
    return new Map([...map].filter(([, v]) => new Set(v.map((b) => `${b.modName}|${b.action}`)).size > 1));
  }, [list]);

  const groups = useMemo(() => {
    const q = query.trim().toLowerCase();
    const map = new Map<string, Binding[]>();
    for (const b of list ?? []) {
      if (onlyActive && !b.enabled) continue;
      if (q && !`${b.modName} ${b.action} ${b.key}`.toLowerCase().includes(q)) continue;
      map.set(b.modName, [...(map.get(b.modName) ?? []), b]);
    }
    // Game defaults last.
    return [...map].sort((a, b) => (a[1][0].source === "game" ? 1 : 0) - (b[1][0].source === "game" ? 1 : 0) || a[0].localeCompare(b[0]));
  }, [list, query, onlyActive]);

  const save = async (b: Binding, key: string, modifiers: string[]) => {
    setEditing(null);
    const l = await run(() => api.rebindKey(b, key, modifiers));
    if (l) {
      setList(l);
      await refresh();
      toast("success", b.source === "mcm" ? "Taste in den MCM-Einstellungen gespeichert." : "Taste geändert – zum Übernehmen ins Spiel „Anwenden“ klicken.");
    }
  };

  const modConflicts = [...conflicts.values()].filter((v) => v.some((b) => b.source !== "game")).length;

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-4xl px-8 py-7">
        <h1 className="font-display text-3xl font-semibold tracking-wide">Tastenbelegung</h1>
        <p className="mt-2 max-w-2xl text-[13px] leading-relaxed text-muted">
          Alle Tasten, die deine Mods belegen: UE4SS-Lua-Mods (<code className="font-mono text-text">RegisterKeyBind</code>), Hotkeys in
          Konfigurationsdateien und Tasten aus dem Mod Configuration Menu. Änderungen werden direkt in die Datei der Mod geschrieben.
        </p>

        <div className="mt-5 flex items-center gap-3">
          <div className="relative w-72">
            <Search size={15} className="pointer-events-none absolute left-3 top-2.5 text-dim" />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Mod, Aktion oder Taste"
              aria-label="Tasten durchsuchen"
              className="h-9 w-full rounded-md border border-line-strong bg-bg pl-9 pr-3 text-sm placeholder:text-dim focus:border-accent focus:outline-none"
            />
          </div>
          <label className="flex items-center gap-2 text-sm text-muted">
            <input type="checkbox" checked={onlyActive} onChange={(e) => setOnlyActive(e.target.checked)} className="accent-[var(--color-accent)]" />
            Nur aktive Mods
          </label>
          {modConflicts > 0 && (
            <span className="ml-auto flex items-center gap-1.5 text-sm text-danger">
              <AlertTriangle size={15} /> {modConflicts} doppelt belegte Taste{modConflicts === 1 ? "" : "n"}
            </span>
          )}
        </div>

        {list === null ? (
          <p className="mt-8 text-sm text-muted">Durchsuche Mods …</p>
        ) : groups.length === 0 || list.every((b) => b.source === "game") ? (
          <Empty icon={<Keyboard size={44} />} title="Keine Mod-Tasten gefunden">
            Sobald Mods Tasten über UE4SS, Konfigurationsdateien oder MCM belegen, erscheinen sie hier.
          </Empty>
        ) : null}

        <div className="mt-6 space-y-6">
          {groups.map(([mod, items]) => (
            <section key={mod}>
              <h2 className="mb-2 flex items-center gap-2 text-sm font-medium">
                {mod}
                {items[0].source === "game" && <span className="text-xs font-normal text-dim">– nur zur Konfliktprüfung</span>}
              </h2>
              <div className="divide-y divide-line rounded-lg border border-line bg-panel">
                {items.map((b) => {
                  const clash = conflicts.get(comboId(b.key, b.modifiers));
                  const others = clash?.filter((o) => o.id !== b.id) ?? [];
                  return (
                    <div key={b.id} className={`flex items-center gap-3 px-4 py-2.5 ${b.enabled ? "" : "opacity-50"}`}>
                      <div className="min-w-0 flex-1">
                        <div className="truncate text-sm">{b.action}</div>
                        {b.file && b.source !== "mcm" && (
                          <div className="truncate font-mono text-[10.5px] text-dim">
                            {b.file}:{b.line}
                          </div>
                        )}
                        {others.length > 0 && (
                          <div className="mt-0.5 text-xs text-danger">
                            Auch belegt von: {others.map((o) => `${o.modName} (${o.action})`).join(", ")}
                          </div>
                        )}
                      </div>
                      <Badge tone={sourceLabel[b.source].tone}>{sourceLabel[b.source].label}</Badge>
                      <KeyChip k={b.key} modifiers={b.modifiers} />
                      {b.editable ? (
                        <IconButton label="Taste ändern" onClick={() => setEditing(b)}>
                          <Pencil size={14} />
                        </IconButton>
                      ) : (
                        <span className="w-8" />
                      )}
                    </div>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      </div>
      {editing && (
        <KeyCapture
          title={`${editing.modName}: ${editing.action}`}
          source={editing.source}
          style={editing.style}
          allowModifiers={editing.source === "ue4ss"}
          onClose={() => setEditing(null)}
          onDone={(k, m) => save(editing, k, m)}
        />
      )}
    </div>
  );
}
