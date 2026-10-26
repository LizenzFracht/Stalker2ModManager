import { open } from "@tauri-apps/plugin-dialog";
import { FolderSearch, HardDrive } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Logo } from "../components/Shell";
import { Button, platformLabel } from "../components/ui";
import { t } from "../i18n";

export function SetupScreen() {
  const { state, run, setState } = useStore();
  if (!state) return null;

  const choose = async (path: string, platform?: (typeof state.detected)[number]["platform"]) => {
    const s = await run(() => api.setGamePath(path, platform));
    if (s) setState(s);
  };

  const browse = async () => {
    const dir = await open({ directory: true, title: t("Stalker-2-Spielordner wählen") });
    if (typeof dir === "string") choose(dir);
  };

  return (
    <div className="pda-texture flex h-full items-center justify-center p-8">
      <div className="w-full max-w-xl">
        <div className="mb-8 flex items-center gap-3 text-accent">
          <Logo />
          <span className="font-display text-3xl font-bold tracking-wider text-text">S2MM</span>
        </div>
        <h1 className="font-display text-4xl font-semibold tracking-wide">{t("Wo liegt die Zone?")}</h1>
        <p className="mt-3 text-[15px] leading-relaxed text-muted">
          {t("Wähle deine Stalker-2-Installation. S2MM legt Mods im Ordner")}{" "}
          <code className="font-mono text-[13px] text-text">Stalker2\Content\Paks\~mods</code> {t("ab und verändert keine Originaldateien, ohne sie vorher zu sichern.")}
        </p>
        {state.game && !state.game.valid && (
          <p className="mt-4 rounded-md border border-danger/40 bg-danger-soft px-3 py-2 text-sm text-danger">
            {t("Im Ordner {path} wurde kein Stalker 2 gefunden.", { path: state.game.path })}
          </p>
        )}

        <div className="mt-8 space-y-2">
          {state.detected.length > 0 && <div className="label-caps mb-2 text-xs text-dim">{t("Gefundene Installationen")}</div>}
          {state.detected.map((d) => (
            <button
              key={d.path}
              onClick={() => choose(d.path, d.platform)}
              className="flex w-full items-center gap-4 rounded-lg border border-line-strong bg-panel px-4 py-3.5 text-left transition-colors hover:border-accent/60 hover:bg-raised"
            >
              <HardDrive size={20} className="text-accent" />
              <div className="min-w-0 flex-1">
                <div className="font-medium">{t(platformLabel[d.platform])}</div>
                <div className="truncate font-mono text-xs text-muted">{d.path}</div>
              </div>
              <span className="text-sm text-accent">{t("Verwenden")}</span>
            </button>
          ))}
          {state.detected.length === 0 && (
            <p className="rounded-lg border border-line bg-panel px-4 py-3.5 text-sm text-muted">
              {t("Keine Installation automatisch gefunden (Steam, Game Pass, GOG und Epic wurden durchsucht).")}
            </p>
          )}
        </div>
        <Button className="mt-5" icon={<FolderSearch size={16} />} onClick={browse}>
          {t("Ordner manuell wählen …")}
        </Button>
      </div>
    </div>
  );
}
