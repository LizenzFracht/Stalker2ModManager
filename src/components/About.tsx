import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ExternalLink, ScrollText } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { t } from "../i18n";
import { Button, Modal } from "./ui";

export const REPO_URL = "https://github.com/LizenzFracht/Stalker2ModManager";

/** People and projects S2MM builds on. Names are not translated. */
const CREDITS: { name: string; url: string; what: string }[] = [
  { name: "joric – Stalker 2 interactive map", url: "https://github.com/joric/stalker", what: "Live-Karte: Kartenseite, Markierungen und Kacheln (Unlicense)" },
  { name: "UE4SS (RE-UE4SS)", url: "https://github.com/UE4SS-RE/RE-UE4SS", what: "Lua-Scripting-Framework und Stalker-2-Spielkonfiguration (MIT)" },
  { name: "Mod Configuration Menu (MCM)", url: "https://www.nexusmods.com/stalker2heartofchornobyl/search/?gsearch=Mod%20Configuration%20Menu", what: "Mod-Einstellungen im Spiel – S2MM bearbeitet dessen Einstellungsdatei" },
  { name: "uesave – trumank", url: "https://github.com/trumank/uesave-rs", what: "Lesen und Schreiben von Unreal-SaveGames (MIT)" },
  { name: "UnRAR – Alexander Roshal (RARLAB)", url: "https://www.rarlab.com/", what: "Entpacken von RAR-Archiven (UnRAR-Lizenz)" },
  { name: "Tauri, React, Tailwind CSS, dnd-kit, Lucide", url: "https://tauri.app/", what: "App-Gerüst und Oberfläche" },
  { name: "Barlow Condensed, IBM Plex Mono", url: "https://fonts.google.com/", what: "Schriftarten (SIL Open Font License)" },
  { name: "Definitive Mod Manager & CDUMM (Crimson Desert)", url: "https://github.com/faisalkindi/CrimsonDesert-UltimateModsManager", what: "Vorbilder für Funktionsumfang und Bedienkonzept" },
];

export function AboutSection() {
  const { state, run } = useStore();
  const [notices, setNotices] = useState<string | null>(null);
  if (!state) return null;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-3 text-[13px]">
        <span className="font-mono text-muted">S2MM v{state.appVersion}</span>
        <span className="text-dim">·</span>
        <span className="text-muted">{t("Open Source unter der MIT-Lizenz")}</span>
        <Button size="sm" variant="ghost" icon={<ExternalLink size={14} />} className="ml-auto" onClick={() => run(() => api.openUrl(REPO_URL))}>
          GitHub
        </Button>
        <Button
          size="sm"
          icon={<ScrollText size={14} />}
          onClick={async () => setNotices(await invoke<string>("third_party_notices"))}
        >
          {t("Lizenzen von Drittanbietern")}
        </Button>
      </div>

      <div>
        <h3 className="label-caps mb-2 text-xs text-dim">{t("Danke an")}</h3>
        <ul className="space-y-1.5 text-[13px]">
          {CREDITS.map((c) => (
            <li key={c.name}>
              <button className="text-accent hover:underline" onClick={() => run(() => api.openUrl(c.url))}>
                {c.name}
              </button>
              <span className="text-muted"> – {t(c.what)}</span>
            </li>
          ))}
        </ul>
      </div>

      <p className="text-xs leading-relaxed text-dim">
        {t("Inoffizielles Fanprojekt. Nicht verbunden mit GSC Game World, Nexus Mods oder Valve. S.T.A.L.K.E.R. ist eine Marke von GSC Game World. Entwickelt mit Unterstützung von KI (Claude, Anthropic).")}
      </p>

      {notices !== null && (
        <Modal title={t("Lizenzen von Drittanbietern")} onClose={() => setNotices(null)} width="max-w-4xl">
          <pre className="whitespace-pre-wrap break-words font-mono text-[11px] leading-5 text-muted">{notices}</pre>
        </Modal>
      )}
    </div>
  );
}
