import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Loader2 } from "lucide-react";
import { api } from "./api";
import { useStore } from "./store";
import { BusyOverlay, DropOverlay, Sidebar, Toasts, TopBar } from "./components/Shell";
import { ModsPage } from "./pages/ModsPage";
import { ConflictsPage } from "./pages/ConflictsPage";
import { TweaksPage } from "./pages/TweaksPage";
import { DownloadsPage } from "./pages/DownloadsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { SetupScreen } from "./pages/SetupScreen";
import { OptionsDialog } from "./pages/OptionsDialog";
import { KeybindsPage } from "./pages/KeybindsPage";
import { McmPage } from "./pages/McmPage";
import { LiveMapPage } from "./pages/LiveMapPage";
import { VanillaDialog } from "./components/VanillaDialog";

export function useInstall() {
  const { run, setState, toast, setOptionsFor, setSelectedMod } = useStore();
  return async (paths: string[]) => {
    if (paths.length === 0) return;
    const r = await run(() => api.addMods(paths), {
      busy: paths.length === 1 ? "Mod wird installiert …" : `${paths.length} Mods werden installiert …`,
    });
    if (!r) return;
    setState(r.state);
    r.errors.forEach((e) => toast("error", e));
    if (r.added.length) {
      toast("success", r.added.length === 1 ? "Mod installiert – zum Aktivieren „Anwenden“ klicken." : `${r.added.length} Mods installiert.`);
      setSelectedMod(r.added[r.added.length - 1]);
      const setup = r.state.mods.find((m) => r.added.includes(m.id) && m.needsSetup);
      if (setup) setOptionsFor(setup.id);
    }
  };
}

export default function App() {
  const { state, page, optionsFor, setOptionsFor } = useStore();
  const [dragging, setDragging] = useState(false);
  // Decided once when the state first arrives, so the dialog can show its result afterwards.
  const [welcome, setWelcome] = useState<boolean | null>(null);
  useEffect(() => {
    if (state && welcome === null) setWelcome(!state.settings.firstRunDone);
  }, [state, welcome]);
  const install = useInstall();

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") setDragging(true);
      else if (p.type === "leave") setDragging(false);
      else if (p.type === "drop") {
        setDragging(false);
        install(p.paths);
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!state) {
    return (
      <div className="flex h-full items-center justify-center text-muted">
        <Loader2 className="mr-3 animate-spin text-accent" /> Lade …
      </div>
    );
  }

  if (!state.game || !state.game.valid) {
    return (
      <>
        <SetupScreen />
        <Toasts />
        <BusyOverlay />
      </>
    );
  }

  const conflictCount = state.mods.filter((m) => m.enabled && m.conflicts.loses > 0).length;

  return (
    <div className="flex h-full">
      <Sidebar conflictCount={conflictCount} />
      <div className="flex min-w-0 flex-1 flex-col">
        <TopBar />
        <main className="min-h-0 flex-1">
          {page === "mods" && <ModsPage />}
          {page === "conflicts" && <ConflictsPage />}
          {page === "tweaks" && <TweaksPage />}
          {page === "keybinds" && <KeybindsPage />}
          {page === "mcm" && <McmPage />}
          {page === "livemap" && <LiveMapPage />}
          {page === "downloads" && <DownloadsPage />}
          {page === "settings" && <SettingsPage />}
        </main>
      </div>
      {welcome && <VanillaDialog welcome onClose={() => setWelcome(false)} />}
      {optionsFor && <OptionsDialog key={optionsFor} modId={optionsFor} onClose={() => setOptionsFor(null)} />}
      <DropOverlay active={dragging} />
      <Toasts />
      <BusyOverlay />
    </div>
  );
}
