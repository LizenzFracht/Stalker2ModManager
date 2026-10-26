import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText } from "./api";
import { invoke } from "@tauri-apps/api/core";
import { getLang, resolveLang, setLang, type Lang } from "./i18n";
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

  const [lang, setLangState] = useState<Lang>(getLang());

  // Every state update passes through here so a language change applies at once.
  const applyState = useCallback((s: AppState) => {
    const l = resolveLang(s.settings.language);
    if (l !== getLang() || !document.documentElement.lang) {
      setLang(l);
      setLangState(l);
      invoke("set_language", { lang: l }).catch(() => {});
    }
    setState(s);
  }, []);

  const refresh = useCallback(async () => {
    try {
      applyState(await api.getState());
    } catch (e) {
      toast("error", errorText(e));
    }
  }, [toast, applyState]);

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
      setState: applyState,
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
    // `lang` re-renders every consumer when the language changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [state, applyState, refresh, run, busy, toasts, toast, dismissToast, downloads, page, selectedMod, optionsFor, mcmFilter, lang],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useStore() {
  const s = useContext(Ctx);
  if (!s) throw new Error("StoreProvider missing");
  return s;
}
