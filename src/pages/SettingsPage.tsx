import { useEffect, useState, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Download, ExternalLink, FolderOpen, FolderSearch, KeyRound, LogOut, ShieldAlert } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Button, Field, Input, Toggle, confirmDialog, platformLabel } from "../components/ui";
import type { DetectedInstall, Settings } from "../types";

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mt-8">
      <h2 className="label-caps mb-3 text-[15px] text-muted">{title}</h2>
      <div className="space-y-5 rounded-lg border border-line bg-panel px-5 py-5">{children}</div>
    </section>
  );
}

function Row({ label, desc, children }: { label: string; desc?: ReactNode; children: ReactNode }) {
  return (
    <div className="flex items-center gap-6">
      <div className="min-w-0 flex-1">
        <div className="text-sm font-medium">{label}</div>
        {desc && <div className="mt-0.5 text-xs leading-relaxed text-muted">{desc}</div>}
      </div>
      {children}
    </div>
  );
}

const selectCls = "h-9 rounded-md border border-line-strong bg-bg px-2.5 text-sm focus:border-accent focus:outline-none";

export function SettingsPage() {
  const { state, run, setState, toast } = useStore();
  const [key, setKey] = useState("");
  const [detected, setDetected] = useState<DetectedInstall[]>([]);
  const [args, setArgs] = useState(state?.settings.launchArgs ?? "");
  const [configDir, setConfigDir] = useState(state?.settings.configDir ?? "");
  const [port, setPort] = useState(String(state?.settings.mapPort ?? 27420));

  useEffect(() => {
    api.detectGames().then(setDetected).catch(() => {});
  }, []);

  if (!state) return null;
  const s = state.settings;
  const game = state.game;

  const save = async (patch: Partial<Settings>) => {
    const next = await run(() => api.saveSettings({ ...s, ...patch }));
    if (next) setState(next);
    return next;
  };

  const browseGame = async () => {
    const dir = await open({ directory: true, title: "Stalker-2-Spielordner wählen" });
    if (typeof dir === "string") {
      const next = await run(() => api.setGamePath(dir), { success: "Spielordner geändert." });
      if (next) setState(next);
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-3xl px-8 py-7 pb-16">
        <h1 className="font-display text-3xl font-semibold tracking-wide">Einstellungen</h1>

        <Section title="Spiel">
          <Field label="Spielordner">
            <div className="flex gap-2">
              <Input readOnly value={game?.path ?? ""} className="font-mono text-xs" />
              <Button icon={<FolderSearch size={15} />} onClick={browseGame}>Ändern</Button>
              <Button variant="ghost" icon={<FolderOpen size={15} />} onClick={() => run(() => api.openLocation("game"))} aria-label="Spielordner öffnen" />
            </div>
          </Field>
          {detected.filter((d) => d.path.toLowerCase() !== game?.path.toLowerCase()).length > 0 && (
            <div className="text-xs text-muted">
              Weitere Installationen:{" "}
              {detected
                .filter((d) => d.path.toLowerCase() !== game?.path.toLowerCase())
                .map((d) => (
                  <button
                    key={d.path}
                    className="mr-3 text-accent hover:underline"
                    onClick={async () => {
                      const next = await run(() => api.setGamePath(d.path, d.platform));
                      if (next) setState(next);
                    }}
                  >
                    {platformLabel[d.platform]} ({d.path})
                  </button>
                ))}
            </div>
          )}
          <Row label="Plattform" desc="Bestimmt Startmethode und Konfigurationsordner.">
            <select className={selectCls} value={s.platform} onChange={(e) => save({ platform: e.target.value as Settings["platform"] })}>
              {Object.entries(platformLabel).map(([k, v]) => (
                <option key={k} value={k}>{k === "unknown" ? "Automatisch" : v}</option>
              ))}
            </select>
          </Row>
          {game && (
            <dl className="grid grid-cols-[150px_1fr] gap-x-3 gap-y-1.5 text-xs">
              <dt className="text-dim">Mods-Ordner</dt>
              <dd className="font-mono text-muted">{game.modsDir}</dd>
              <dt className="text-dim">UE4SS</dt>
              <dd className="text-muted">{game.ue4ssInstalled ? `installiert (${game.ue4ssModsDir})` : "nicht installiert – Lua-Mods benötigen UE4SS"}</dd>
              {game.build && (
                <>
                  <dt className="text-dim">Steam-Build</dt>
                  <dd className="font-mono text-muted">{game.build}</dd>
                </>
              )}
            </dl>
          )}
          <Field label="Konfigurationsordner (Engine.ini)" hint={`Leer lassen für automatisch: ${game?.configDir ?? "–"}`}>
            <div className="flex gap-2">
              <Input value={configDir} onChange={(e) => setConfigDir(e.target.value)} placeholder="Automatisch" className="font-mono text-xs" />
              <Button disabled={configDir === (s.configDir ?? "")} onClick={() => save({ configDir: configDir.trim() || null })}>Übernehmen</Button>
            </div>
          </Field>
        </Section>

        <Section title="Spielstart">
          <Row label="Startmethode" desc="„Automatisch“ startet Steam-Versionen über Steam, andere direkt über die EXE.">
            <select className={selectCls} value={s.launchMethod} onChange={(e) => save({ launchMethod: e.target.value as Settings["launchMethod"] })}>
              <option value="auto">Automatisch</option>
              <option value="steam">Über Steam</option>
              <option value="direct">Direkt (EXE)</option>
            </select>
          </Row>
          <Field label="Startparameter" hint="Zum Beispiel -dx12 oder -nosplash.">
            <div className="flex gap-2">
              <Input value={args} onChange={(e) => setArgs(e.target.value)} className="font-mono text-xs" />
              <Button disabled={args === s.launchArgs} onClick={() => save({ launchArgs: args })}>Übernehmen</Button>
            </div>
          </Field>
        </Section>

        <Section title="Installation">
          <Row
            label="Harte Links verwenden"
            desc="Mods werden ohne Kopie in den Spielordner verlinkt (spart Platz und Zeit). Funktioniert nur auf demselben Laufwerk, sonst wird kopiert."
          >
            <Toggle label="Harte Links verwenden" checked={s.useHardlinks} onChange={(v) => save({ useHardlinks: v })} />
          </Row>
          <Row label="„_P“-Suffix für Paks erzwingen" desc="Pak-Dateien ohne _P im Namen werden sonst eventuell von den Spieldateien überschrieben.">
            <Toggle label="_P-Suffix erzwingen" checked={s.addPSuffix} onChange={(v) => save({ addPSuffix: v })} />
          </Row>
        </Section>

        <Section title="UE4SS & Werkzeuge">
          <Row
            label="UE4SS"
            desc={
              state.tools.ue4ss.modId
                ? `Von S2MM verwaltet${state.tools.ue4ss.version ? ` · ${state.tools.ue4ss.version}` : ""}. Ausschalten entfernt UE4SS beim nächsten „Anwenden“ wieder aus dem Spiel.`
                : state.tools.ue4ssInGame
                  ? "Manuell installiertes UE4SS gefunden. Über „Installieren“ übernimmt S2MM die Verwaltung (die manuelle Version wird gesichert)."
                  : "Lua-Scripting-Framework für Mods wie die Live-Karte. Wird bei Bedarf von GitHub geladen (inkl. Stalker-2-Konfiguration)."
            }
          >
            <div className="flex items-center gap-3">
              <Button
                size="sm"
                icon={<Download size={14} />}
                onClick={async () => {
                  const next = await run(() => api.ue4ssInstall(), { busy: "UE4SS wird heruntergeladen …", success: "UE4SS installiert – zum Aktivieren „Anwenden“ klicken." });
                  if (next) setState(next);
                }}
              >
                {state.tools.ue4ss.modId ? "Aktualisieren" : "Installieren"}
              </Button>
              {state.tools.ue4ss.modId && (
                <Toggle
                  label="UE4SS aktiv"
                  checked={state.tools.ue4ss.enabled}
                  onChange={async (v) => {
                    const next = await run(() => api.setToolEnabled("ue4ss", v));
                    if (next) setState(next);
                  }}
                />
              )}
            </div>
          </Row>
          <Field label="Port der Live-Karte" hint="Der Server lauscht nur auf 127.0.0.1 (nicht im Netzwerk erreichbar).">
            <div className="flex gap-2">
              <Input value={port} onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))} inputMode="numeric" className="w-32 font-mono text-xs" />
              <Button
                disabled={!port || Number(port) === s.mapPort || Number(port) < 1024 || Number(port) > 65535}
                onClick={() => save({ mapPort: Number(port) })}
              >
                Übernehmen
              </Button>
            </div>
          </Field>
        </Section>

        <Section title="Nexus Mods">
          {state.nexusUser || s.nexusApiKey ? (
            <Row
              label={state.nexusUser ? `Verbunden als ${state.nexusUser.name}` : "API-Schlüssel hinterlegt"}
              desc={state.nexusUser ? (state.nexusUser.isPremium ? "Premium: Updates können direkt heruntergeladen werden." : "Kostenloses Konto: Downloads über „Mod Manager Download“ auf der Website.") : "Wird beim nächsten Zugriff geprüft."}
            >
              <Button
                variant="ghost"
                icon={<LogOut size={15} />}
                onClick={async () => {
                  const next = await run(() => api.nexusLogout());
                  if (next) setState(next);
                }}
              >
                Abmelden
              </Button>
            </Row>
          ) : (
            <Field
              label="Persönlicher API-Schlüssel"
              hint={
                <>
                  Zu finden unter{" "}
                  <button className="text-accent hover:underline" onClick={() => run(() => api.openUrl("https://www.nexusmods.com/users/myaccount?tab=api%20access"))}>
                    Nexus → Konto → API-Zugriff <ExternalLink size={11} className="inline" />
                  </button>
                  . Der Schlüssel wird nur lokal gespeichert.
                </>
              }
            >
              <div className="flex gap-2">
                <Input type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder="API-Schlüssel einfügen" className="font-mono text-xs" />
                <Button
                  variant="primary"
                  icon={<KeyRound size={15} />}
                  disabled={key.trim().length < 10}
                  onClick={async () => {
                    const next = await run(() => api.nexusLogin(key), { busy: "Prüfe Schlüssel …" });
                    if (next) {
                      setState(next);
                      setKey("");
                      toast("success", `Verbunden als ${next.nexusUser?.name}.`);
                    }
                  }}
                >
                  Verbinden
                </Button>
              </div>
            </Field>
          )}
          <Row
            label="„Mod Manager Download“-Links öffnen"
            desc={
              <>
                Registriert S2MM für <code className="font-mono">nxm://</code>-Links. Beim Ausschalten wird ein zuvor registrierter Manager (z. B.
                Vortex) wiederhergestellt.
              </>
            }
          >
            <Toggle label="nxm-Links" checked={s.nxmHandler} onChange={(v) => save({ nxmHandler: v })} />
          </Row>
          <Row label="Downloads automatisch installieren">
            <Toggle label="Automatisch installieren" checked={s.autoInstallDownloads} onChange={(v) => save({ autoInstallDownloads: v })} />
          </Row>
          <Row label="Beim Start nach Updates suchen">
            <Toggle label="Updates beim Start" checked={s.checkUpdatesOnStart} onChange={(v) => save({ checkUpdatesOnStart: v })} />
          </Row>
        </Section>

        <Section title="Daten">
          <Row label="Mod-Speicher" desc={<span className="font-mono">{state.dataDir}</span>}>
            <Button variant="ghost" icon={<FolderOpen size={15} />} onClick={() => run(() => api.openLocation("data"))}>Öffnen</Button>
          </Row>
          <p className="text-xs leading-relaxed text-dim">
            Portabler Modus: Lege eine leere Datei <code className="font-mono">portable.txt</code> neben die S2MM.exe – dann werden alle Daten
            im Unterordner „S2MM-Daten“ gespeichert.
          </p>
        </Section>

        <Section title="Zurücksetzen">
          <Row
            label="Alle Mods aus dem Spiel entfernen"
            desc={`Entfernt alle ${state.deployedFiles} von S2MM platzierten Dateien und stellt gesicherte Originale wieder her. Deine Mod-Bibliothek bleibt erhalten.`}
          >
            <Button
              variant="danger"
              icon={<ShieldAlert size={15} />}
              disabled={state.deployedFiles === 0}
              onClick={async () => {
                if (!(await confirmDialog("Alle Mods aus dem Spielordner entfernen (Vanilla-Zustand)?", "Entfernen"))) return;
                const next = await run(() => api.purge(), { busy: "Entferne Mods …", success: "Spiel ist wieder im Originalzustand." });
                if (next) setState(next);
              }}
            >
              Vanilla herstellen
            </Button>
          </Row>
        </Section>
      </div>
    </div>
  );
}
