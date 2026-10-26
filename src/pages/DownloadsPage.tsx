import { useState } from "react";
import { CheckCircle2, Download as DownloadIcon, ExternalLink, FolderOpen, Link2, Trash2, XCircle } from "lucide-react";
import { api, NEXUS_GAME_URL } from "../api";
import { useStore } from "../store";
import { Badge, Button, Empty, IconButton, Input, Progress, formatBytes } from "../components/ui";

export function DownloadsPage() {
  const { state, downloads, run, setPage } = useStore();
  const [link, setLink] = useState("");
  if (!state) return null;
  const connected = !!state.settings.nexusApiKey;

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-4xl px-8 py-7">
        <div className="flex items-start gap-4">
          <div className="flex-1">
            <h1 className="font-display text-3xl font-semibold tracking-wide">Downloads</h1>
            <p className="mt-2 max-w-2xl text-[13px] leading-relaxed text-muted">
              Klicke auf Nexus Mods bei einer Datei auf <strong className="text-text">„Mod Manager Download“</strong> – S2MM lädt sie
              herunter und installiert sie automatisch. Updates ersetzen die vorhandene Mod und behalten Reihenfolge und Optionen bei.
            </p>
          </div>
          <Button icon={<ExternalLink size={15} />} onClick={() => run(() => api.openUrl(`${NEXUS_GAME_URL}/mods/`))}>
            Nexus öffnen
          </Button>
        </div>

        <div className="mt-5 grid grid-cols-2 gap-3">
          <Status ok={connected} label="Nexus-Konto" detail={state.nexusUser ? `${state.nexusUser.name}${state.nexusUser.isPremium ? " · Premium" : ""}` : connected ? "Schlüssel hinterlegt" : "Nicht verbunden"} />
          <Status ok={state.settings.nxmHandler} label="nxm-Links" detail={state.settings.nxmHandler ? "S2MM öffnet „Mod Manager Download“" : "Nicht als Handler registriert"} />
        </div>
        {(!connected || !state.settings.nxmHandler) && (
          <Button className="mt-3" size="sm" onClick={() => setPage("settings")}>
            In den Einstellungen einrichten
          </Button>
        )}

        <div className="mt-6 flex gap-2">
          <div className="relative flex-1">
            <Link2 size={15} className="pointer-events-none absolute left-3 top-2.5 text-dim" />
            <Input
              value={link}
              onChange={(e) => setLink(e.target.value)}
              placeholder="nxm://stalker2heartofchornobyl/mods/…  (Link manuell einfügen)"
              className="pl-9 font-mono text-xs"
            />
          </div>
          <Button
            disabled={!link.trim().startsWith("nxm://")}
            onClick={async () => {
              await run(() => api.handleNxm(link.trim()));
              setLink("");
            }}
          >
            Herunterladen
          </Button>
        </div>

        <div className="mt-6">
          {downloads.length === 0 ? (
            <Empty icon={<DownloadIcon size={40} />} title="Keine Downloads">
              Downloads aus dieser Sitzung erscheinen hier.
            </Empty>
          ) : (
            <ul className="space-y-2">
              {downloads.map((d) => (
                <li key={d.id} className="rounded-lg border border-line bg-panel px-4 py-3">
                  <div className="flex items-center gap-3">
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-sm font-medium">{d.modName ?? d.fileName}</div>
                      <div className="truncate font-mono text-[11px] text-dim">
                        {d.fileName}
                        {d.version && ` · v${d.version}`}
                      </div>
                    </div>
                    {d.state === "running" && <span className="font-mono text-xs text-muted">{formatBytes(d.received)}{d.total ? ` / ${formatBytes(d.total)}` : ""}</span>}
                    {d.state === "done" && <Badge tone="accent">Heruntergeladen</Badge>}
                    {d.state === "installed" && <Badge tone="ok"><CheckCircle2 size={11} /> Installiert</Badge>}
                    {d.state === "failed" && <Badge tone="danger"><XCircle size={11} /> Fehler</Badge>}
                    {(d.state === "done" || d.state === "failed") && d.received > 0 && (
                      <Button size="sm" variant={d.state === "done" ? "primary" : "secondary"} onClick={() => run(() => api.installDownload(d.id), { busy: "Installiere …" })}>
                        Installieren
                      </Button>
                    )}
                    {d.state !== "running" && (
                      <>
                        <IconButton label="Download-Ordner öffnen" onClick={() => run(() => api.openLocation("downloads"))}>
                          <FolderOpen size={15} />
                        </IconButton>
                        <IconButton label="Aus Liste entfernen und Datei löschen" onClick={() => run(() => api.removeDownload(d.id, true))}>
                          <Trash2 size={15} />
                        </IconButton>
                      </>
                    )}
                  </div>
                  {d.state === "running" && (
                    <div className="mt-2.5">
                      <Progress value={d.total ? d.received / d.total : null} />
                    </div>
                  )}
                  {d.error && <p className="mt-2 whitespace-pre-line text-xs text-danger">{d.error}</p>}
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}

function Status({ ok, label, detail }: { ok: boolean; label: string; detail: string }) {
  return (
    <div className="flex items-center gap-3 rounded-lg border border-line bg-panel px-4 py-3">
      <span className={`h-2.5 w-2.5 rounded-full ${ok ? "bg-ok" : "bg-dim"}`} />
      <div className="min-w-0">
        <div className="text-sm font-medium">{label}</div>
        <div className="truncate text-xs text-muted">{detail}</div>
      </div>
    </div>
  );
}
