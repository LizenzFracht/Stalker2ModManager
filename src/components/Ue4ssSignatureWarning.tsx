import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Download, FileArchive, PowerOff } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { t } from "../i18n";
import { Button } from "./ui";
import type { AppState } from "../types";

/** Lets the user pick their own UE4SS package (e.g. a Stalker 2 build from Nexus Mods). */
export async function installCustomUe4ss(
  run: <T>(fn: () => Promise<T>, opts?: { busy?: string; success?: string }) => Promise<T | undefined>,
): Promise<AppState | undefined> {
  const file = await open({
    multiple: false,
    title: t("Eigenes UE4SS-Paket wählen"),
    filters: [{ name: "UE4SS", extensions: ["zip", "7z", "rar"] }],
  });
  if (!file || Array.isArray(file)) return undefined;
  return run(() => api.ue4ssInstallFile(file), {
    busy: t("UE4SS wird installiert …"),
    success: t("Eigenes UE4SS installiert – zum Übernehmen „Anwenden“ klicken."),
  });
}

/** Warns when UE4SS's game signatures no longer match the installed Stalker 2 exe – the
 *  usual cause of the game hanging or crashing on start after a game update. */
export function Ue4ssSignatureWarning({ className = "" }: { className?: string }) {
  const { state, run, setState, toast } = useStore();
  const sig = state?.tools.ue4ssSignatures;
  if (!state || !sig || sig.missing.length === 0) return null;
  const managed = !!state.tools.ue4ss.modId;
  // A library copy that is switched off cannot crash the game.
  if (sig.source === "library" && !state.tools.ue4ss.enabled) return null;

  const disable = async () => {
    const s = await run(() => api.setToolEnabled("ue4ss", false));
    if (!s) return;
    setState(s);
    const r = await run(() => api.apply(), { busy: t("Mods werden angewendet …") });
    if (r) {
      setState(r.state);
      toast("success", t("UE4SS wurde aus dem Spiel entfernt. Mods, die UE4SS brauchen, sind bis zu einem Update inaktiv."));
    }
  };

  const update = async () => {
    const s = await run(() => api.ue4ssInstall(), { busy: t("UE4SS wird heruntergeladen …") });
    if (!s) return;
    setState(s);
    const still = s.tools.ue4ssSignatures?.missing.length ?? 0;
    toast(still ? "info" : "success", still ? t("Noch kein passendes UE4SS-Update verfügbar.") : t("UE4SS aktualisiert – zum Übernehmen „Anwenden“ klicken."));
  };

  return (
    <div className={`flex items-start gap-3 rounded-md border border-danger/40 bg-danger-soft px-3.5 py-3 text-[13px] leading-snug ${className}`}>
      <AlertTriangle size={17} className="mt-0.5 shrink-0 text-danger" />
      <div className="min-w-0 flex-1">
        <p className="font-medium text-text">{t("UE4SS passt nicht zur aktuellen Spielversion")}</p>
        <p className="mt-1 text-muted">
          {t("Die UE4SS-Signaturen {names} werden in der Spiel-EXE nicht gefunden (meist nach einem Spiel-Update). Das Spiel kann dann beim Start hängen oder abstürzen. Deaktiviere UE4SS, bis ein passendes Update erscheint.", {
            names: sig.missing.join(", "),
          })}
        </p>
        <div className="mt-2.5 flex flex-wrap gap-2">
          {managed && state.tools.ue4ss.enabled && (
            <Button size="sm" variant="danger" icon={<PowerOff size={14} />} onClick={disable}>
              {t("UE4SS deaktivieren")}
            </Button>
          )}
          {managed && (
            <Button size="sm" icon={<Download size={14} />} onClick={update}>
              {t("Nach UE4SS-Update suchen")}
            </Button>
          )}
          <Button
            size="sm"
            icon={<FileArchive size={14} />}
            onClick={async () => {
              const s = await installCustomUe4ss(run);
              if (s) setState(s);
            }}
          >
            {t("Eigenes UE4SS installieren …")}
          </Button>
          {!managed && <span className="self-center text-xs text-dim">{t("UE4SS wurde manuell installiert – „Vanilla herstellen“ in den Einstellungen entfernt es.")}</span>}
        </div>
      </div>
    </div>
  );
}
