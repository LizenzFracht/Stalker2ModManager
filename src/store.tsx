import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText } from "./api";
import type { AppState, Download } from "./types";

export type Page = "mods" | "conflicts" | "tweaks" | "keybinds" | "mcm" | "livemap" | "downloads" | "settings";
export type ToastKind = "info" | "success" | "error";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

interface RunOptions {
  busy?: string;
  success?: string;
}

interface Store {
  state: AppState | null;
  setState: (s: AppState) => void;
  refresh: () => Promise<void>;
  run: <T>(fn: () => Promise<T>, opts?: RunOptions) => Promise<T | undefined>;
  busy: string | null;
  toasts: Toast[];
  toast: (kind: ToastKind, message: string) => void;
  dismissToast: (id: number) => void;
  downloads: Download[];
  page: Page;
  setPage: (p: Page) => void;
  selectedMod: string | null;
  setSelectedMod: (id: string | null) => void;
  optionsFor: string | null;
  setOptionsFor: (id: string | null) => void;
  mcmFilter: string;
  setMcmFilter: (f: string) => void;
}

const Ctx = createContext<Store | null>(null);

export function StoreProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AppState | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [downloads, setDownloads] = useState<Download[]>([]);
  const [page, setPage] = useState<Page>("mods");
  const [selectedMod, setSelectedMod] = useState<string | null>(null);
  const [optionsFor, setOptionsFor] = useState<string | null>(null);
  const [mcmFilter, setMcmFilter] = useState("");
  const nextId = useRef(1);

  const dismissToast = useCallback((id: number) => setToasts((t) => t.filter((x) => x.id !== id)), []);

  const toast = useCallback(
    (kind: ToastKind, message: string) => {
      const id = nextId.current++;
      setToasts((t) => [...t.slice(-4), { id, kind, message }]);
      setTimeout(() => dismissToast(id), kind === "error" ? 9000 : 4500);
    },
    [dismissToast],
  );

  const refresh = useCallback(async () => {
    try {
      setState(await api.getState());
    } catch (e) {
      toast("error", errorText(e));
    }
  }, [toast]);

  const run = useCallback(
    async <T,>(fn: () => Promise<T>, opts: RunOptions = {}) => {
      if (opts.busy) setBusy(opts.busy);
      try {
        const result = await fn();
        if (opts.success) toast("success", opts.success);
        return result;
      } catch (e) {
        toast("error", errorText(e));
        return undefined;
      } finally {
        if (opts.busy) setBusy(null);
      }
    },
    [toast],
  );

  useEffect(() => {
    refresh();
    api.getDownloads().then(setDownloads).catch(() => {});
    const unlisten = [
      listen<{ kind: ToastKind; message: string }>("notice", (e) => toast(e.payload.kind, e.payload.message)),
      listen<Download[]>("downloads-changed", (e) => setDownloads(e.payload)),
      listen("state-changed", () => refresh()),
    ];
    return () => {
      unlisten.forEach((p) => p.then((f) => f()));
    };
  }, [refresh, toast]);

  const value = useMemo(
    () => ({
      state,
      setState,
      refresh,
      run,
      busy,
      toasts,
      toast,
      dismissToast,
      downloads,
      page,
      setPage,
      selectedMod,
      setSelectedMod,
      optionsFor,
      setOptionsFor,
      mcmFilter,
      setMcmFilter,
    }),
    [state, refresh, run, busy, toasts, toast, dismissToast, downloads, page, selectedMod, optionsFor, mcmFilter],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useStore() {
  const s = useContext(Ctx);
  if (!s) throw new Error("StoreProvider fehlt");
  return s;
}
