import { useEffect, useState, type ReactNode } from "react";
import { CheckCircle2, Circle, Download, ExternalLink, Map as MapIcon, Play, Square, Upload } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Button, Toggle } from "../components/ui";
import type { LiveStatus } from "../types";
import { t } from "../i18n";

function Step({ done, title, children, action }: { done: boolean; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <li className="flex items-start gap-3 px-4 py-3.5">
      {done ? <CheckCircle2 size={18} className="mt-0.5 shrink-0 text-ok" /> : <Circle size={18} className="mt-0.5 shrink-0 text-dim" />}
      <div className="min-w-0 flex-1">
        <div className="text-sm font-medium">{title}</div>
        {children && <div className="mt-0.5 text-xs leading-relaxed text-muted">{children}</div>}
      </div>
      {action}
    </li>
  );
}

export function LiveMapPage() {
  const { state, run, setState, toast } = useStore();
  const [status, setStatus] = useState<LiveStatus | null>(null);

  useEffect(() => {
    let alive = true;
    const poll = () => api.livemapStatus().then((s) => alive && setStatus(s)).catch(() => {});
    poll();
    const timer = setInterval(poll, 1500);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, []);

  if (!state) return null;
  const { ue4ss, ue4ssInGame, liveMapMod } = state.tools;
  const ue4ssReady = (!!ue4ss.modId && ue4ss.enabled) || (!ue4ss.modId && ue4ssInGame);
  const modReady = !!liveMapMod.modId && liveMapMod.enabled;
  const applied = ue4ssReady && modReady && !state.pending;
  const pos = status?.position;
  const live = !!pos?.ok && pos.age < 6;

  const apply = async () => {
    const r = await run(() => api.apply(), { busy: t("Mods werden angewendet …") });
    if (r) setState(r.state);
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-3xl px-8 py-7">
        <h1 className="font-display text-3xl font-semibold tracking-wide">{t("Live-Karte")}</h1>
        <p className="mt-2 max-w-2xl text-[13px] leading-relaxed text-muted">
          {t("Zeigt deine Position im Spiel live auf")}{" "}
          <button className="text-accent hover:underline" onClick={() => run(() => api.openUrl("https://github.com/joric/stalker"))}>
            {t("joric's interaktiver Stalker-2-Karte")}
          </button>
          {t(". S2MM startet dafür einen lokalen Server (nur auf diesem PC erreichbar), der die Karte im Browser öffnet. Eine kleine UE4SS-Mod meldet die Position einmal pro Sekunde.")}
        </p>

        <ol className="mt-6 divide-y divide-line rounded-lg border border-line bg-panel">
          <Step
            done={ue4ssReady}
            title={t("UE4SS")}
            action={
              ue4ss.modId ? (
                <Toggle
                  label={t("UE4SS aktiv")}
                  checked={ue4ss.enabled}
                  onChange={async (v) => {
                    const s = await run(() => api.setToolEnabled("ue4ss", v));
                    if (s) setState(s);
                  }}
                />
              ) : (
                !ue4ssInGame && (
                  <Button size="sm" icon={<Download size={14} />} onClick={async () => {
                    const s = await run(() => api.ue4ssInstall(), { busy: t("UE4SS wird heruntergeladen …"), success: "UE4SS installiert." });
                    if (s) setState(s);
                  }}>
                    {t("Installieren")}
                  </Button>
                )
              )
            }
          >
            {ue4ss.modId
              ? t("Von S2MM verwaltet{v}.", { v: ue4ss.version ? ` (${ue4ss.version})` : "" })
              : ue4ssInGame
                ? "Manuell installiert gefunden."
                : t("Lua-Scripting-Framework – wird von GitHub geladen, inklusive Stalker-2-Konfiguration.")}
          </Step>
          <Step
            done={modReady}
            title={t("S2MM-Live-Map-Mod")}
            action={
              liveMapMod.modId ? (
                <Toggle
                  label={t("Live-Map-Mod aktiv")}
                  checked={liveMapMod.enabled}
                  onChange={async (v) => {
                    const s = await run(() => api.setToolEnabled("livemap", v));
                    if (s) setState(s);
                  }}
                />
              ) : (
                <Button size="sm" icon={<Download size={14} />} onClick={async () => {
                  const s = await run(() => api.livemapInstallMod(), { success: t("Live-Map-Mod hinzugefügt.") });
                  if (s) setState(s);
                }}>
                  {t("Hinzufügen")}
                </Button>
              )
            }
          >
            {t("Kleines Lua-Skript, das die Position nach")} <code className="font-mono">{"%LOCALAPPDATA%\\S2MM\\live.json"}</code> {t("schreibt.")}
          </Step>
          <Step
            done={applied}
            title={t("Im Spiel anwenden")}
            action={state.pending && <Button size="sm" variant="primary" icon={<Upload size={14} />} onClick={apply}>{t("Anwenden")}</Button>}
          >
            {t("Danach Stalker 2 (neu) starten.")}
          </Step>
          <Step
            done={!!status?.running}
            title={t("Karten-Server")}
            action={
              status?.running ? (
                <div className="flex gap-2">
                  <Button size="sm" variant="primary" icon={<ExternalLink size={14} />} onClick={() => run(() => api.livemapStart(true))}>
                    {t("Im Browser öffnen")}
                  </Button>
                  <Button size="sm" icon={<Square size={13} />} onClick={() => run(() => api.livemapStop()).then((s) => s && setStatus(s))}>
                    {t("Stoppen")}
                  </Button>
                </div>
              ) : (
                <Button
                  size="sm"
                  variant="primary"
                  icon={<Play size={14} />}
                  onClick={async () => {
                    const s = await run(() => api.livemapStart(true));
                    if (s) {
                      setStatus(s);
                      toast("info", t("Karte wird im Browser geöffnet. Der erste Start lädt die Kartendaten (ca. 50 MB)."));
                    }
                  }}
                >
                  {t("Starten")}
                </Button>
              )
            }
          >
            {status?.url ? <span className="font-mono">{status.url}</span> : t("Port {port} (änderbar in den Einstellungen)", { port: state.settings.mapPort })}
          </Step>
        </ol>

        <div className="mt-6 flex items-center gap-4 rounded-lg border border-line bg-panel px-5 py-4">
          <MapIcon size={28} className={live ? "text-accent" : "text-dim"} />
          <div className="flex-1">
            <div className="flex items-center gap-2 text-sm font-medium">
              <span className={`h-2 w-2 rounded-full ${live ? "pulse-dot bg-ok" : "bg-dim"}`} />
              {live ? t("Spiel verbunden") : pos?.ok ? t("Letzte Position vor {n} s", { n: pos.age }) : t("Noch keine Positionsdaten")}
            </div>
            {pos?.ok && (
              <div className="mt-1 font-mono text-xs text-muted">
                X {Math.round(pos.x)} · Y {Math.round(pos.y)} · Z {Math.round(pos.z)} · {t("Blick")} {Math.round(pos.yaw)}°
              </div>
            )}
          </div>
        </div>
        <p className="mt-3 text-xs leading-relaxed text-dim">
          {t("Kartendaten und -kacheln stammen von joric.github.io (Unlicense) und werden lokal zwischengespeichert. Im Browser lässt sich „Folgen“ ein- und ausschalten; Ziehen der Karte beendet das Folgen.")}
        </p>
      </div>
    </div>
  );
}
