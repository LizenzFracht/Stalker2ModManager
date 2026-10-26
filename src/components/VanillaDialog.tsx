import { useState } from "react";
import { FolderOpen, ShieldCheck } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Button, Modal } from "./ui";
import type { VanillaReport } from "../types";
import { t } from "../i18n";

const optionList = [
  { key: "ue4ss", title: "UE4SS entfernen", desc: "Auch manuell installiertes UE4SS (dwmapi.dll und ue4ss-Ordner) aus dem Spielordner nehmen." },
  { key: "engineIni", title: "Engine.ini-Tweaks zurücksetzen", desc: "Alle Tweak-Abschnitte entfernen – die vom Spiel geschriebenen Einträge bleiben. Gilt für Steam- und Game-Pass-Konfiguration." },
  { key: "foreignMods", title: "Fremde Mods entfernen", desc: "Manuell installierte Dateien in ~mods und Stalker2\\Mods." },
] as const;

type Options = { ue4ss: boolean; engineIni: boolean; foreignMods: boolean };

/** Restores an unmodded game; `welcome` turns it into the first-start dialog. */
export function VanillaDialog({ welcome = false, onClose }: { welcome?: boolean; onClose: () => void }) {
  const { run, setState } = useStore();
  const [opts, setOpts] = useState<Options>({ ue4ss: true, engineIni: true, foreignMods: true });
  const [report, setReport] = useState<VanillaReport | null>(null);

  const skip = async () => {
    if (welcome) {
      const s = await run(() => api.finishFirstRun());
      if (s) setState(s);
    }
    onClose();
  };

  const restore = async () => {
    const r = await run(() => api.restoreVanilla(opts), { busy: t("Stelle Vanilla-Zustand her …") });
    if (r) {
      setState(r.state);
      setReport(r.report);
    }
  };

  if (report) {
    return (
      <Modal title={t("Vanilla hergestellt")} onClose={onClose} width="max-w-lg" footer={<Button variant="primary" onClick={onClose}>{t("Fertig")}</Button>}>
        <div className="flex items-start gap-3">
          <ShieldCheck size={22} className="mt-0.5 shrink-0 text-ok" />
          <div className="space-y-2 text-[13px] leading-relaxed">
            <p>{t("Das Spiel ist jetzt im Originalzustand, das Profil „Vanilla“ ist aktiv.")}</p>
            <ul className="list-disc space-y-0.5 pl-4 text-muted">
              <li>{t("{n} von S2MM platzierte Dateien entfernt", { n: report.removedDeployed })}</li>
              {report.moved.length > 0 && <li>{t("{n} fremde Dateien/Ordner (Mods, UE4SS) beiseitegelegt", { n: report.moved.length })}</li>}
              {report.engineIniReset.length > 0 && <li>{t("Engine.ini-Tweaks zurückgesetzt ({n}×)", { n: report.engineIniReset.length })}</li>}
              {report.removedDeployed === 0 && report.moved.length === 0 && report.engineIniReset.length === 0 && <li>{t("Es war bereits alles im Originalzustand.")}</li>}
            </ul>
            {report.quarantine && (
              <p className="text-muted">
                {t("Nichts wurde gelöscht – alles liegt in einem Sicherungsordner.")}{" "}
                <button className="inline-flex items-center gap-1 text-accent hover:underline" onClick={() => run(() => api.openFolder(report.quarantine!))}>
                  <FolderOpen size={13} /> {t("Öffnen")}
                </button>
              </p>
            )}
            <p className="text-muted">{t("Deine Mod-Bibliothek bleibt erhalten. Wechsle das Profil, um Mods wieder zu nutzen.")}</p>
          </div>
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      title={welcome ? "Willkommen bei S2MM" : "Vanilla herstellen"}
      onClose={skip}
      width="max-w-lg"
      footer={
        <>
          <Button variant="ghost" onClick={skip}>{welcome ? t("Überspringen") : t("Abbrechen")}</Button>
          <Button variant="primary" icon={<ShieldCheck size={16} />} onClick={restore}>
            {t("Vanilla wiederherstellen")}
          </Button>
        </>
      }
    >
      <p className="text-[13px] leading-relaxed text-muted">
        {welcome
          ? t("Für einen sauberen Start empfiehlt es sich, das Spiel einmal in den Originalzustand zu bringen. Danach installierst und verwaltest du alle Mods über S2MM.")
          : t("Bringt das Spiel in den Originalzustand und aktiviert das Profil „Vanilla“.")}{" "}
        {t("Alle von S2MM installierten Mods werden aus dem Spielordner entfernt. Nichts wird gelöscht – fremde Dateien landen in einem Sicherungsordner.")}
      </p>
      <div className="mt-4 space-y-2">
        {optionList.map((o) => (
          <label key={o.key} className="flex cursor-pointer items-start gap-3 rounded-lg border border-line bg-bg px-3.5 py-2.5 hover:border-line-strong">
            <input
              type="checkbox"
              checked={opts[o.key]}
              onChange={(e) => setOpts((x) => ({ ...x, [o.key]: e.target.checked }))}
              className="mt-0.5 accent-[var(--color-accent)]"
            />
            <span>
              <span className="block text-sm font-medium">{t(o.title)}</span>
              <span className="mt-0.5 block text-xs leading-relaxed text-muted">{t(o.desc)}</span>
            </span>
          </label>
        ))}
      </div>
      <p className="mt-3 text-xs text-dim">{t("Das Spiel muss dafür geschlossen sein.")}</p>
    </Modal>
  );
}
