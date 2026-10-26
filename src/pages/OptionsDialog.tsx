import { useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { ImageOff } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Button, Modal } from "../components/ui";
import type { GroupKind, ModOption, ModView, OptionGroup } from "../types";
import { t } from "../i18n";

const kindHint: Record<GroupKind, string> = {
  exactlyOne: "Genau eine auswählen",
  atMostOne: "Höchstens eine auswählen",
  atLeastOne: "Mindestens eine auswählen",
  any: "Beliebig viele auswählen",
  all: "Alle werden installiert",
};

function initialSelection(mod: ModView) {
  const sel: Record<string, string[]> = {};
  for (const g of mod.layout.groups) {
    const chosen = mod.selection[g.id];
    if (chosen) sel[g.id] = chosen;
    else {
      const d = g.options.filter((o) => o.defaultSelected).map((o) => o.id);
      sel[g.id] = d.length || !(g.kind === "exactlyOne" || g.kind === "atLeastOne") ? d : [g.options[0].id];
    }
    if (g.kind === "all") sel[g.id] = g.options.map((o) => o.id);
  }
  return sel;
}

function groupError(g: OptionGroup, chosen: string[]) {
  if (g.kind === "exactlyOne" && chosen.length !== 1) return t("Bitte genau eine Option wählen.");
  if (g.kind === "atLeastOne" && chosen.length < 1) return t("Bitte mindestens eine Option wählen.");
  if (g.kind === "atMostOne" && chosen.length > 1) return t("Höchstens eine Option erlaubt.");
  return null;
}

export function OptionsDialog({ modId, onClose }: { modId: string; onClose: () => void }) {
  const { state, run, setState } = useStore();
  const mod = state?.mods.find((m) => m.id === modId);
  const [sel, setSel] = useState<Record<string, string[]>>(() => (mod ? initialSelection(mod) : {}));
  const [focus, setFocus] = useState<ModOption | null>(null);

  const steps = useMemo(() => {
    const map = new Map<string, OptionGroup[]>();
    for (const g of mod?.layout.groups ?? []) {
      const k = g.step ?? "";
      map.set(k, [...(map.get(k) ?? []), g]);
    }
    return [...map.entries()];
  }, [mod]);

  if (!mod || !state) return null;
  const errors = mod.layout.groups.map((g) => groupError(g, sel[g.id] ?? [])).filter(Boolean);

  const toggle = (g: OptionGroup, o: ModOption) => {
    if (g.kind === "all") return;
    setSel((s) => {
      const cur = s[g.id] ?? [];
      const single = g.kind === "exactlyOne" || g.kind === "atMostOne";
      let next: string[];
      if (single) next = cur.includes(o.id) && g.kind === "atMostOne" ? [] : [o.id];
      else next = cur.includes(o.id) ? cur.filter((x) => x !== o.id) : [...cur, o.id];
      return { ...s, [g.id]: next };
    });
    setFocus(o);
  };

  const imageSrc = (o: ModOption | null) => {
    if (!o?.image) return null;
    const parts = [state.dataDir, "mods", mod.id, "files", mod.layout.root, o.image].filter(Boolean);
    return convertFileSrc(parts.join("\\").replace(/\//g, "\\"));
  };
  const hasImages = mod.layout.groups.some((g) => g.options.some((o) => o.image));
  const preview =
    focus ??
    mod.layout.groups.flatMap((g) => g.options.filter((o) => (sel[g.id] ?? []).includes(o.id))).find((o) => o.image) ??
    null;

  const save = async () => {
    const s = await run(() => api.setSelection(mod.id, sel));
    if (s) {
      setState(s);
      onClose();
    }
  };

  return (
    <Modal
      title={<>{t("Optionen ·")} <span className="text-accent">{mod.name}</span></>}
      onClose={onClose}
      width={hasImages ? "max-w-5xl" : "max-w-2xl"}
      footer={
        <>
          {errors.length > 0 && <span className="mr-auto self-center text-xs text-danger">{errors[0]}</span>}
          <Button variant="ghost" onClick={onClose}>{t("Abbrechen")}</Button>
          <Button variant="primary" onClick={save} disabled={errors.length > 0}>{t("Übernehmen")}</Button>
        </>
      }
    >
      <div className={hasImages ? "grid grid-cols-[1fr_340px] gap-6" : ""}>
        <div className="space-y-6">
          <p className="text-[13px] leading-relaxed text-muted">
            {mod.layout.fomod
              ? t("Diese Mod bringt einen Installer (FOMOD) mit. Wähle die gewünschten Komponenten.")
              : t("S2MM hat in dieser Mod mehrere Varianten bzw. optionale Ordner erkannt.")}{" "}
            {t("Du kannst die Auswahl jederzeit ändern – danach „Anwenden“ klicken.")}
          </p>
          {steps.map(([step, groups]) => (
            <div key={step} className="space-y-4">
              {step && <h3 className="font-display text-lg font-semibold tracking-wide text-accent">{step}</h3>}
              {groups.map((g) => {
                const single = g.kind === "exactlyOne" || g.kind === "atMostOne";
                return (
                  <fieldset key={g.id} className="rounded-lg border border-line bg-bg/40 p-3">
                    <legend className="px-1.5">
                      <span className="text-sm font-medium">{t(g.name)}</span>
                      <span className="ml-2 text-xs text-dim">{t(kindHint[g.kind])}</span>
                    </legend>
                    <div className="space-y-1">
                      {g.options.map((o) => {
                        const checked = (sel[g.id] ?? []).includes(o.id);
                        return (
                          <label
                            key={o.id}
                            onMouseEnter={() => o.image && setFocus(o)}
                            className={`flex cursor-pointer items-start gap-3 rounded-md px-2.5 py-2 transition-colors ${
                              checked ? "bg-accent-soft" : "hover:bg-hover/60"
                            }`}
                          >
                            <input
                              type={single ? "radio" : "checkbox"}
                              name={g.id}
                              checked={checked}
                              disabled={g.kind === "all"}
                              onChange={() => toggle(g, o)}
                              onClick={() => {
                                if (single && checked && g.kind === "atMostOne") toggle(g, o);
                              }}
                              className="mt-0.5 accent-[var(--color-accent)]"
                            />
                            <span className="min-w-0">
                              <span className="block text-sm">{o.name}</span>
                              {o.description && <span className="mt-0.5 block whitespace-pre-line text-xs leading-relaxed text-muted">{o.description}</span>}
                            </span>
                          </label>
                        );
                      })}
                    </div>
                  </fieldset>
                );
              })}
            </div>
          ))}
        </div>
        {hasImages && (
          <div className="sticky top-0 self-start">
            <div className="flex aspect-video items-center justify-center overflow-hidden rounded-lg border border-line bg-bg">
              {imageSrc(preview) ? (
                <img src={imageSrc(preview)!} alt={preview?.name ?? ""} className="h-full w-full object-contain" />
              ) : (
                <ImageOff size={28} className="text-dim" />
              )}
            </div>
            {preview && <p className="mt-2 text-sm font-medium">{preview.name}</p>}
          </div>
        )}
      </div>
    </Modal>
  );
}
