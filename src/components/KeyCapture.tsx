import { useEffect, useState } from "react";
import { keyForStyle, keyLabel } from "../keys";
import { Button, Modal } from "./ui";
import { t } from "../i18n";

export function KeyChip({ k, modifiers = [] }: { k: string; modifiers?: string[] }) {
  return (
    <span className="inline-flex items-center gap-1">
      {[...modifiers.map((m) => keyLabel(m)), keyLabel(k)].map((part, i) => (
        <kbd
          key={i}
          className="min-w-7 rounded border border-b-2 border-line-strong bg-raised px-1.5 py-0.5 text-center font-mono text-[11.5px] text-text"
        >
          {part}
        </kbd>
      ))}
    </span>
  );
}

/** Waits for a key press and returns it in the binding's own naming style. */
export function KeyCapture({
  title,
  source,
  style,
  allowModifiers,
  onDone,
  onClose,
}: {
  title: string;
  source: string;
  style: string;
  allowModifiers: boolean;
  onDone: (key: string, modifiers: string[]) => void;
  onClose: () => void;
}) {
  const [picked, setPicked] = useState<{ key: string; mods: string[] } | null>(null);
  const [hint, setHint] = useState("");

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (["ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"].includes(e.code)) return;
      if (e.code === "Escape" && !picked) return onClose();
      const key = keyForStyle(e.code, source, style);
      if (!key) {
        setHint(t("Die Taste „{key}“ wird von diesem Format nicht unterstützt.", { key: e.code }));
        return;
      }
      const mods = allowModifiers ? [e.ctrlKey && "CONTROL", e.shiftKey && "SHIFT", e.altKey && "ALT"].filter(Boolean) as string[] : [];
      setHint(!allowModifiers && (e.ctrlKey || e.shiftKey || e.altKey) ? t("Dieses Format unterstützt keine Zusatztasten – sie werden ignoriert.") : "");
      setPicked({ key, mods });
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [source, style, allowModifiers, onClose, picked]);

  return (
    <Modal
      title={title}
      onClose={onClose}
      width="max-w-md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>{t("Abbrechen")}</Button>
          <Button variant="primary" disabled={!picked} onClick={() => picked && onDone(picked.key, picked.mods)}>{t("Übernehmen")}</Button>
        </>
      }
    >
      <div className="flex min-h-32 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-line-strong bg-bg py-6">
        {picked ? <KeyChip k={picked.key} modifiers={picked.mods} /> : <span className="pulse-dot text-sm text-muted">{t("Neue Taste drücken …")}</span>}
        {allowModifiers && <span className="text-xs text-dim">{t("Strg, Shift und Alt können kombiniert werden.")}</span>}
      </div>
      {hint && <p className="mt-3 text-xs text-accent">{hint}</p>}
    </Modal>
  );
}
