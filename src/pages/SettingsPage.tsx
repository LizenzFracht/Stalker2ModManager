import { useEffect, useState, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Download, ExternalLink, FolderOpen, FolderSearch, KeyRound, LogOut, ShieldAlert } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Button, Field, Input, Toggle, platformLabel } from "../components/ui";
import { VanillaDialog } from "../components/VanillaDialog";
import { AboutSection } from "../components/About";
import type { DetectedInstall, Settings } from "../types";
import { LANGUAGES, t } from "../i18n";

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
  const [vanilla, setVanilla] = useState(false);

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
    const dir = await open({ directory: true, title: t("Stalker-2-Spielordner wählen") });
    if (typeof dir === "string") {
      const next = await run(() => api.setGamePath(dir), { success: t("Spielordner geändert.") });
      if (next) setState(next);
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-3xl px-8 py-7 pb-16">
        <h1 className="font-display text-3xl font-semibold tracking-wide">{t("Einstellungen")}</h1>

        <Section title={t("Sprache")}>
          <Row label={t("Sprache der Oberfläche")} desc={t("„Automatisch“ folgt der Systemsprache.")}>
            <select className={selectCls} value={s.language} onChange={(e) => save({ language: e.target.value })} aria-label={t("Sprache")}>
              <option value="auto">{t("Automatisch")}</option>
              {LANGUAGES.map((l) => (
                <option key={l.id} value={l.id}>{l.label}</option>
              ))}
            </select>
          </Row>
        </Section>

        <Section title={t("Spiel")}>
          <Field label={t("Spielordner")}>
            <div className="flex gap-2">
              <Input readOnly value={game?.path ?? ""} className="font-mono text-xs" />
              <Button icon={<FolderSearch size={15} />} onClick={browseGame}>{t("Ändern")}</Button>
              <Button variant="ghost" icon={<FolderOpen size={15} />} onClick={() => run(() => api.openLocation("game"))} aria-label={t("Spielordner öffnen")} />
            </div>
          </Field>
          {detected.filter((d) => d.path.toLowerCase() !== game?.path.toLowerCase()).length > 0 && (
            <div className="text-xs text-muted">
              {t("Weitere Installationen:")}{" "}
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
          <Row label={t("Plattform")} desc={t("Bestimmt Startmethode und Konfigurationsordner.")}>
            <select className={selectCls} value={s.platform} onChange={(e) => save({ platform: e.target.value as Settings["platform"] })}>
              {Object.entries(platformLabel).map(([k, v]) => (
                <option key={k} value={k}>{k === "unknown" ? t("Automatisch") : t(v)}</option>
              ))}
            </select>
          </Row>
          {game && (
            <dl className="grid grid-cols-[150px_1fr] gap-x-3 gap-y-1.5 text-xs">
              <dt className="text-dim">{t("Mods-Ordner")}</dt>
              <dd className="font-mono text-muted">{game.modsDir}</dd>
              <dt className="text-dim">UE4SS</dt>
              <dd className="text-muted">{game.ue4ssInstalled ? t("installiert ({dir})", { dir: game.ue4ssModsDir }) : t("nicht installiert – Lua-Mods benötigen UE4SS")}</dd>
              {game.build && (
                <>
                  <dt className="text-dim">{t("Steam-Build")}</dt>
                  <dd className="font-mono text-muted">{game.build}</dd>
                </>
              )}
            </dl>
          )}
          <Field label={t("Konfigurationsordner (Engine.ini)")} hint={t("Leer lassen für automatisch: {dir}", { dir: game?.configDir ?? "–" })}>
            <div className="flex gap-2">
              <Input value={configDir} onChange={(e) => setConfigDir(e.target.value)} placeholder={t("Automatisch")} className="font-mono text-xs" />
              <Button disabled={configDir === (s.configDir ?? "")} onClick={() => save({ configDir: configDir.trim() || null })}>{t("Übernehmen")}</Button>
            </div>
          </Field>
        </Section>

        <Section title={t("Spielstart")}>
          <Row label={t("Startmethode")} desc={t("„Automatisch“ startet Steam-Versionen über Steam, andere direkt über die EXE.")}>
            <select className={selectCls} value={s.launchMethod} onChange={(e) => save({ launchMethod: e.target.value as Settings["launchMethod"] })}>
              <option value="auto">{t("Automatisch")}</option>
              <option value="steam">{t("Über Steam")}</option>
              <option value="direct">{t("Direkt (EXE)")}</option>
            </select>
          </Row>
          <Field label={t("Startparameter")} hint={t("Zum Beispiel -dx12 oder -nosplash.")}>
            <div className="flex gap-2">
              <Input value={args} onChange={(e) => setArgs(e.target.value)} className="font-mono text-xs" />
              <Button disabled={args === s.launchArgs} onClick={() => save({ launchArgs: args })}>{t("Übernehmen")}</Button>
            </div>
          </Field>
        </Section>

        <Section title={t("Installation")}>
          <Row
            label={t("Harte Links verwenden")}
            desc={t("Mods werden ohne Kopie in den Spielordner verlinkt (spart Platz und Zeit). Funktioniert nur auf demselben Laufwerk, sonst wird kopiert.")}
          >
            <Toggle label={t("Harte Links verwenden")} checked={s.useHardlinks} onChange={(v) => save({ useHardlinks: v })} />
          </Row>
          <Row label={t("„_P“-Suffix für Paks erzwingen")} desc={t("Pak-Dateien ohne _P im Namen werden sonst eventuell von den Spieldateien überschrieben.")}>
            <Toggle label={t("_P-Suffix erzwingen")} checked={s.addPSuffix} onChange={(v) => save({ addPSuffix: v })} />
          </Row>
        </Section>

        <Section title={t("UE4SS & Werkzeuge")}>
          <Row
            label={t("UE4SS")}
            desc={
              state.tools.ue4ss.modId
                ? t("Von S2MM verwaltet{v}. Ausschalten entfernt UE4SS beim nächsten „Anwenden“ wieder aus dem Spiel.", { v: state.tools.ue4ss.version ? ` · ${state.tools.ue4ss.version}` : "" })
                : state.tools.ue4ssInGame
                  ? t("Manuell installiertes UE4SS gefunden. Über „Installieren“ übernimmt S2MM die Verwaltung (die manuelle Version wird gesichert).")
                  : t("Lua-Scripting-Framework für Mods wie die Live-Karte. Wird bei Bedarf von GitHub geladen (inkl. Stalker-2-Konfiguration).")
            }
          >
            <div className="flex items-center gap-3">
              <Button
                size="sm"
                icon={<Download size={14} />}
                onClick={async () => {
                  const next = await run(() => api.ue4ssInstall(), { busy: t("UE4SS wird heruntergeladen …"), success: t("UE4SS installiert – zum Aktivieren „Anwenden“ klicken.") });
                  if (next) setState(next);
                }}
              >
                {state.tools.ue4ss.modId ? "Aktualisieren" : "Installieren"}
              </Button>
              {state.tools.ue4ss.modId && (
                <Toggle
                  label={t("UE4SS aktiv")}
                  checked={state.tools.ue4ss.enabled}
                  onChange={async (v) => {
                    const next = await run(() => api.setToolEnabled("ue4ss", v));
                    if (next) setState(next);
                  }}
                />
              )}
            </div>
          </Row>
          <Field label={t("Port der Live-Karte")} hint={t("Der Server lauscht nur auf 127.0.0.1 (nicht im Netzwerk erreichbar).")}>
            <div className="flex gap-2">
              <Input value={port} onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))} inputMode="numeric" className="w-32 font-mono text-xs" />
              <Button
                disabled={!port || Number(port) === s.mapPort || Number(port) < 1024 || Number(port) > 65535}
                onClick={() => save({ mapPort: Number(port) })}
              >
                {t("Übernehmen")}
              </Button>
            </div>
          </Field>
        </Section>

        <Section title={t("Nexus Mods")}>
          {!state.nexusEnabled ? (
            <p className="text-[13px] leading-relaxed text-muted">
              {t("Die direkte Nexus-Anbindung (Login, „Mod Manager Download“, Update-Prüfung) ist in dieser Version noch deaktiviert, bis S2MM bei Nexus Mods als App registriert ist. Lade Mods im Browser herunter und ziehe die Archive einfach in S2MM – Nexus-IDs werden aus dem Dateinamen erkannt.")}
            </p>
          ) : (
            <>
              {state.nexusUser || s.nexusApiKey ? (
                <Row
                  label={state.nexusUser ? t("Verbunden als {name}", { name: state.nexusUser.name }) : t("API-Schlüssel hinterlegt")}
                  desc={state.nexusUser ? (state.nexusUser.isPremium ? t("Premium: Updates können direkt heruntergeladen werden.") : t("Kostenloses Konto: Downloads über „Mod Manager Download“ auf der Website.")) : t("Wird beim nächsten Zugriff geprüft.")}
                >
                  <Button
                    variant="ghost"
                    icon={<LogOut size={15} />}
                    onClick={async () => {
                      const next = await run(() => api.nexusLogout());
                      if (next) setState(next);
                    }}
                  >
                    {t("Abmelden")}
                  </Button>
                </Row>
              ) : (
                <Field
                  label={t("Persönlicher API-Schlüssel")}
                  hint={
                    <>
                      <button className="text-accent hover:underline" onClick={() => run(() => api.openUrl("https://www.nexusmods.com/users/myaccount?tab=api%20access"))}>
                        {t("Nexus → Konto → API-Zugriff")} <ExternalLink size={11} className="inline" />
                      </button>{" "}
                      {t("– dort ganz unten den persönlichen API-Schlüssel kopieren. Er wird nur lokal gespeichert.")}
                    </>
                  }
                >
                  <div className="flex gap-2">
                    <Input type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder={t("API-Schlüssel einfügen")} className="font-mono text-xs" />
                    <Button
                      variant="primary"
                      icon={<KeyRound size={15} />}
                      disabled={key.trim().length < 10}
                      onClick={async () => {
                        const next = await run(() => api.nexusLogin(key), { busy: t("Prüfe Schlüssel …") });
                        if (next) {
                          setState(next);
                          setKey("");
                          toast("success", t("Verbunden als {name}.", { name: next.nexusUser?.name ?? "" }));
                        }
                      }}
                    >
                      {t("Verbinden")}
                    </Button>
                  </div>
                </Field>
              )}
              <Row
                label={t("„Mod Manager Download“-Links öffnen")}
                desc={t("Registriert S2MM für nxm://-Links. Beim Ausschalten wird ein zuvor registrierter Manager (z. B. Vortex) wiederhergestellt.")}
              >
                <Toggle label={t("nxm-Links")} checked={s.nxmHandler} onChange={(v) => save({ nxmHandler: v })} />
              </Row>
              <Row label={t("Downloads automatisch installieren")}>
                <Toggle label={t("Automatisch installieren")} checked={s.autoInstallDownloads} onChange={(v) => save({ autoInstallDownloads: v })} />
              </Row>
              <Row label={t("Beim Start nach Updates suchen")}>
                <Toggle label={t("Updates beim Start")} checked={s.checkUpdatesOnStart} onChange={(v) => save({ checkUpdatesOnStart: v })} />
              </Row>
            </>
          )}
        </Section>

        <Section title={t("Daten")}>
          <Row label={t("Mod-Speicher")} desc={<span className="font-mono">{state.dataDir}</span>}>
            <Button variant="ghost" icon={<FolderOpen size={15} />} onClick={() => run(() => api.openLocation("data"))}>{t("Öffnen")}</Button>
          </Row>
          <p className="text-xs leading-relaxed text-dim">
            {t("Portabler Modus: Lege eine leere Datei")} <code className="font-mono">portable.txt</code> {t("neben das S2MM-Programm (bzw. die AppImage) – dann werden alle Daten im Unterordner „S2MM-Data“ gespeichert. Die Portable-Version bringt diese Datei schon mit.")}
          </p>
        </Section>

        <Section title={t("Zurücksetzen")}>
          <Row
            label={t("Vanilla wiederherstellen")}
            desc={t("Entfernt alle {n} von S2MM platzierten Dateien, auf Wunsch auch UE4SS, Engine.ini-Tweaks und fremde Mods, und aktiviert das Profil „Vanilla“. Fremde Dateien werden gesichert, nicht gelöscht; deine Mod-Bibliothek bleibt erhalten.", { n: state.deployedFiles })}
          >
            <Button variant="danger" icon={<ShieldAlert size={15} />} onClick={() => setVanilla(true)}>
              {t("Vanilla herstellen")}
            </Button>
          </Row>
          {vanilla && <VanillaDialog onClose={() => setVanilla(false)} />}
        </Section>

        <Section title={t("Über S2MM")}>
          <AboutSection />
        </Section>
      </div>
    </div>
  );
}
