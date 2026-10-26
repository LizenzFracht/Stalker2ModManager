import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  AlertTriangle,
  ArrowDownCircle,
  ArrowUpCircle,
  FilePlus2,
  FolderPlus,
  GripVertical,
  Package,
  RefreshCw,
  Search,
  Settings2,
  Sparkles,
  Wand2,
} from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { useInstall } from "../App";
import { Badge, Button, Empty, Toggle, kindLabel } from "../components/ui";
import type { ModView } from "../types";
import { ModDetail } from "./ModDetail";
import { AutosortDialog } from "./AutosortDialog";
import { t } from "../i18n";

type Filter = "all" | "enabled" | "disabled" | "updates" | "issues";

const filters: { id: Filter; label: string }[] = [
  { id: "all", label: "Alle" },
  { id: "enabled", label: "Aktiv" },
  { id: "disabled", label: "Inaktiv" },
  { id: "updates", label: "Updates" },
  { id: "issues", label: "Probleme" },
];

export function hasIssue(m: ModView) {
  return m.needsSetup || m.plan.files === 0 || (m.enabled && m.conflicts.loses > 0);
}

export function ModThumb({ mod, size = 36 }: { mod: ModView; size?: number }) {
  const [failed, setFailed] = useState(false);
  if (mod.pictureUrl && !failed) {
    return (
      <img
        src={mod.pictureUrl}
        alt=""
        onError={() => setFailed(true)}
        className="shrink-0 rounded object-cover"
        style={{ width: size, height: size }}
      />
    );
  }
  return (
    <div
      className="flex shrink-0 items-center justify-center rounded bg-raised font-display font-semibold text-dim"
      style={{ width: size, height: size, fontSize: size * 0.42 }}
    >
      {mod.name.slice(0, 1).toUpperCase()}
    </div>
  );
}

export function useToggleMods() {
  const { state, run, setState, toast } = useStore();
  return async (ids: string[], enabled: boolean) => {
    const before = state?.activeProfile;
    const s = await run(() => api.setEnabled(ids, enabled));
    if (!s) return;
    setState(s);
    if (before === "vanilla" && s.activeProfile !== before) {
      const name = s.profiles.find((p) => p.id === s.activeProfile)?.name;
      toast("info", t("Das Vanilla-Profil bleibt ohne Mods – zu Profil „{name}“ gewechselt.", { name: name ?? "" }));
    }
  };
}

function ModRow({ mod, selected, draggable }: { mod: ModView; selected: boolean; draggable: boolean }) {
  const { setSelectedMod, setOptionsFor } = useStore();
  const toggleMods = useToggleMods();
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: mod.id,
    disabled: !draggable,
  });
  const style = { transform: CSS.Translate.toString(transform), transition };

  return (
    <div
      ref={setNodeRef}
      style={style}
      onClick={() => setSelectedMod(mod.id)}
      className={`group flex h-[54px] cursor-default items-center gap-3 border-b border-line px-3 transition-colors ${
        selected ? "bg-raised" : "hover:bg-hover/50"
      } ${isDragging ? "relative z-10 bg-raised shadow-2xl ring-1 ring-accent/50" : ""} ${mod.enabled ? "" : "text-muted"}`}
    >
      <button
        {...attributes}
        {...listeners}
        aria-label={`${mod.name} verschieben`}
        disabled={!draggable}
        className="cursor-grab text-dim hover:text-muted active:cursor-grabbing disabled:cursor-not-allowed disabled:opacity-20"
        onClick={(e) => e.stopPropagation()}
      >
        <GripVertical size={16} />
      </button>
      <span className="w-7 text-right font-mono text-xs text-dim">{String(mod.priority).padStart(2, "0")}</span>
      <Toggle
        checked={mod.enabled}
        label={mod.enabled ? "Deaktivieren" : "Aktivieren"}
        onChange={(v) => toggleMods([mod.id], v)}
      />
      <ModThumb mod={mod} />
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className={`truncate font-medium ${mod.enabled ? "text-text" : ""}`}>{mod.name}</span>
          {mod.version && <span className="shrink-0 font-mono text-[11px] text-dim">v{mod.version.replace(/^v/i, "")}</span>}
        </div>
        <div className="truncate text-xs text-dim">
          {mod.author ? t("von {author}", { author: mod.author }) : mod.archiveName ?? t("Manuell hinzugefügt")}
          {mod.category && <span> · {mod.category}</span>}
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-1.5">
        {mod.needsSetup && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              setOptionsFor(mod.id);
            }}
          >
            <Badge tone="accent" title={t("Diese Mod hat Optionen, die noch nicht bestätigt wurden")}>
              <Settings2 size={11} /> {t("Optionen")}
            </Badge>
          </button>
        )}
        {mod.update?.available && (
          <Badge tone="info" title={t("Neue Version {v} auf Nexus", { v: mod.update.latestVersion ?? "" })}>
            <Sparkles size={11} /> Update
          </Badge>
        )}
        {mod.plan.files === 0 && (
          <Badge tone="danger" title={t("Keine Dateien gefunden, die installiert werden können")}>
            <AlertTriangle size={11} /> {t("Leer")}
          </Badge>
        )}
        {mod.enabled && mod.conflicts.wins > 0 && (
          <span className="flex items-center gap-0.5 font-mono text-[11px] text-ok" title={t("Überschreibt {n} Dateien anderer Mods", { n: mod.conflicts.wins })}>
            <ArrowUpCircle size={13} />
            {mod.conflicts.wins}
          </span>
        )}
        {mod.enabled && mod.conflicts.loses > 0 && (
          <span className="flex items-center gap-0.5 font-mono text-[11px] text-danger" title={t("{n} Dateien werden von anderen Mods überschrieben", { n: mod.conflicts.loses })}>
            <ArrowDownCircle size={13} />
            {mod.conflicts.loses}
          </span>
        )}
        {mod.plan.kinds.map((k) => (
          <Badge key={k} title={t(kindLabel[k].title)}>
            {t(kindLabel[k].label)}
          </Badge>
        ))}
      </div>
    </div>
  );
}

export function ModsPage() {
  const { state, run, setState, selectedMod, toast, setPage } = useStore();
  const install = useInstall();
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [sorting, setSorting] = useState(false);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const mods = state?.mods ?? [];
  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return mods.filter((m) => {
      if (q && !`${m.name} ${m.author ?? ""} ${m.category ?? ""}`.toLowerCase().includes(q)) return false;
      switch (filter) {
        case "enabled":
          return m.enabled;
        case "disabled":
          return !m.enabled;
        case "updates":
          return !!m.update?.available;
        case "issues":
          return hasIssue(m);
        default:
          return true;
      }
    });
  }, [mods, query, filter]);

  if (!state) return null;
  const draggable = !query && filter === "all";
  const selected = mods.find((m) => m.id === selectedMod) ?? null;
  const updates = mods.filter((m) => m.update?.available).length;
  const active = mods.filter((m) => m.enabled).length;

  const onDragEnd = async (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const ids = mods.map((m) => m.id);
    const next = arrayMove(ids, ids.indexOf(String(e.active.id)), ids.indexOf(String(e.over.id)));
    setState({ ...state, mods: next.map((id, i) => ({ ...mods.find((m) => m.id === id)!, priority: i + 1 })) });
    const s = await run(() => api.reorderMods(next));
    if (s) setState(s);
  };

  const addFiles = async () => {
    const files = await open({
      multiple: true,
      title: t("Mods hinzufügen"),
      filters: [{ name: "Mods", extensions: ["zip", "7z", "rar", "pak", "utoc", "ucas", "dll", "asi"] }],
    });
    if (files) install(Array.isArray(files) ? files : [files]);
  };

  const addFolder = async () => {
    const dir = await open({ directory: true, title: t("Mod-Ordner hinzufügen") });
    if (typeof dir === "string") install([dir]);
  };

  return (
    <div className="flex h-full">
      <section className="flex min-w-0 flex-1 flex-col">
        <div className="flex flex-wrap items-center gap-2 px-5 pb-3 pt-4">
          <Button variant="primary" icon={<FilePlus2 size={16} />} onClick={addFiles}>
            {t("Mod hinzufügen")}
          </Button>
          <Button icon={<FolderPlus size={16} />} onClick={addFolder}>
            {t("Ordner")}
          </Button>
          {state.nexusEnabled && state.settings.nexusApiKey && (
            <Button
              icon={<RefreshCw size={15} />}
              onClick={async () => {
                const s = await run(() => api.checkUpdates(), { busy: t("Prüfe Nexus auf Updates …") });
                if (s) {
                  setState(s);
                  const n = s.mods.filter((m) => m.update?.available).length;
                  toast(n ? "info" : "success", n ? t("{n} Update(s) verfügbar", { n }) : t("Alle Nexus-Mods sind aktuell."));
                }
              }}
            >
              {t("Updates prüfen")}
            </Button>
          )}
          <Button icon={<Wand2 size={15} />} onClick={() => setSorting(true)} disabled={mods.length < 2}>
            {t("Automatisch sortieren")}
          </Button>
          <div className="relative ml-auto w-64">
            <Search size={15} className="pointer-events-none absolute left-3 top-2.5 text-dim" />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("Mods durchsuchen")}
              aria-label={t("Mods durchsuchen")}
              className="h-9 w-full rounded-md border border-line-strong bg-bg pl-9 pr-3 text-sm placeholder:text-dim focus:border-accent focus:outline-none"
            />
          </div>
        </div>

        <div className="flex items-center gap-1 px-5 pb-3">
          {filters.filter((f) => f.id !== "updates" || state.nexusEnabled).map((f) => (
            <button
              key={f.id}
              onClick={() => setFilter(f.id)}
              className={`h-7 rounded-full px-3 text-xs transition-colors ${
                filter === f.id ? "bg-accent text-accent-ink font-semibold" : "text-muted hover:bg-hover hover:text-text"
              }`}
            >
              {t(f.label)}
              {f.id === "updates" && updates > 0 && <span className="ml-1 font-mono">{updates}</span>}
            </button>
          ))}
          <span className="ml-auto text-xs text-dim">
            {t("{n} von {total} aktiv", { n: active, total: mods.length })}
            {!draggable && mods.length > 0 && " · " + t("Sortieren nur ohne Filter")}
          </span>
        </div>

        {state.activeProfile === "vanilla" && (
          <Banner
            tone="info"
            action={
              <Button
                size="sm"
                onClick={async () => {
                  const target = state.profiles.find((p) => p.id !== "vanilla");
                  if (!target) return;
                  const s = await run(() => api.switchProfile(target.id));
                  if (s) setState(s);
                }}
              >
                Zu „{state.profiles.find((p) => p.id !== "vanilla")?.name}“ wechseln
              </Button>
            }
          >
            {t("Profil „Vanilla“ ist aktiv: Das Spiel läuft ohne Mods. Dieses Profil ist schreibgeschützt – wenn du eine Mod aktivierst, wechselt S2MM automatisch in dein Mod-Profil.")}
          </Banner>
        )}
        {state.gameUpdated && (
          <Banner tone="info">
            {t("Stalker 2 wurde seit dem letzten Anwenden aktualisiert. Prüfe, ob deine Mods noch zur Spielversion passen, und klicke danach auf „Anwenden“.")}
          </Banner>
        )}
        {state.unmanaged.length > 0 && (
          <Banner
            tone="accent"
            action={
              <Button
                size="sm"
                onClick={async () => {
                  const r = await run(() => api.importUnmanaged(state.unmanaged), { busy: t("Übernehme Dateien …") });
                  if (r) {
                    setState(r.state);
                    r.errors.forEach((e) => toast("error", e));
                    if (r.added.length) toast("success", t("{n} Mod(s) übernommen. Zum Aktivieren „Anwenden“ klicken.", { n: r.added.length }));
                  }
                }}
              >
                {t("In S2MM übernehmen")}
              </Button>
            }
          >
            {t("{n} Datei(en) im ~mods-Ordner werden nicht von S2MM verwaltet (z. B. manuell installiert). Übernommene Dateien kannst du hier aktivieren, sortieren und auf Konflikte prüfen.", { n: state.unmanaged.length })}
          </Banner>
        )}
        {state.conflictErrors > 0 && (
          <Banner tone="accent" action={<Button size="sm" onClick={() => setPage("conflicts")}>Details</Button>}>
            {t("{n} Pak-Datei(en) konnten nicht auf Konflikte geprüft werden.", { n: state.conflictErrors })}
          </Banner>
        )}

        <div className="min-h-0 flex-1 overflow-y-auto border-t border-line">
          {mods.length === 0 ? (
            <Empty icon={<Package size={44} />} title={t("Noch keine Mods")}>
              {state.nexusEnabled ? t("Ziehe Archive (ZIP, 7z, RAR), Pak-Dateien oder Ordner in dieses Fenster, oder nutze „Mod Manager Download“ auf Nexus Mods.") : t("Ziehe Archive (ZIP, 7z, RAR), Pak-Dateien oder Ordner in dieses Fenster.")}
            </Empty>
          ) : visible.length === 0 ? (
            <Empty icon={<Search size={40} />} title={t("Keine Treffer")} />
          ) : (
            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
              <SortableContext items={visible.map((m) => m.id)} strategy={verticalListSortingStrategy}>
                {visible.map((m) => (
                  <ModRow key={m.id} mod={m} selected={m.id === selectedMod} draggable={draggable} />
                ))}
              </SortableContext>
            </DndContext>
          )}
          {mods.length > 1 && (
            <p className="px-5 py-4 text-xs leading-relaxed text-dim">
              {t("Ladereihenfolge: Weiter unten stehende Mods werden später geladen und gewinnen bei Konflikten. Pak-Dateien erhalten dafür beim Anwenden ein Nummern-Präfix.")}
            </p>
          )}
        </div>
      </section>
      {selected && <ModDetail key={selected.id} mod={selected} />}
      {sorting && <AutosortDialog onClose={() => setSorting(false)} />}
    </div>
  );
}

export function Banner({
  tone,
  children,
  action,
}: {
  tone: "accent" | "info" | "danger";
  children: React.ReactNode;
  action?: React.ReactNode;
}) {
  const cls = {
    accent: "border-accent/30 bg-accent-soft",
    info: "border-info/30 bg-info-soft",
    danger: "border-danger/30 bg-danger-soft",
  }[tone];
  return (
    <div className={`mx-5 mb-3 flex items-center gap-3 rounded-md border px-3.5 py-2.5 text-[13px] leading-snug ${cls}`}>
      <AlertTriangle size={16} className={tone === "info" ? "text-info" : tone === "danger" ? "text-danger" : "text-accent"} />
      <p className="flex-1">{children}</p>
      {action}
    </div>
  );
}
