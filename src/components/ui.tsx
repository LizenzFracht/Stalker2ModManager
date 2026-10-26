import { useEffect, type ButtonHTMLAttributes, type InputHTMLAttributes, type ReactNode } from "react";
import { X } from "lucide-react";
import { ask } from "@tauri-apps/plugin-dialog";
import { locale, t } from "../i18n";

export function confirmDialog(message: string, okLabel?: string) {
  return ask(message, { title: "S2MM", kind: "warning", okLabel: okLabel ?? t("Ja"), cancelLabel: t("Abbrechen") });
}

type Variant = "primary" | "secondary" | "ghost" | "danger";

const variants: Record<Variant, string> = {
  primary: "bg-accent text-accent-ink hover:brightness-110 font-semibold",
  secondary: "bg-raised text-text border border-line-strong hover:bg-hover",
  ghost: "text-muted hover:text-text hover:bg-hover",
  danger: "bg-danger-soft text-danger border border-danger/40 hover:bg-danger/25",
};

export function Button({
  variant = "secondary",
  size = "md",
  icon,
  children,
  className = "",
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: "sm" | "md"; icon?: ReactNode }) {
  const sz = size === "sm" ? "h-7 px-2.5 text-xs gap-1.5" : "h-9 px-3.5 text-sm gap-2";
  return (
    <button
      className={`inline-flex shrink-0 items-center justify-center rounded-md transition-colors disabled:pointer-events-none disabled:opacity-40 ${sz} ${variants[variant]} ${className}`}
      {...rest}
    >
      {icon}
      {children}
    </button>
  );
}

export function IconButton({
  label,
  children,
  className = "",
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      aria-label={label}
      title={label}
      className={`inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted transition-colors hover:bg-hover hover:text-text disabled:opacity-40 ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
  disabled,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={(e) => {
        e.stopPropagation();
        onChange(!checked);
      }}
      className={`relative h-5 w-9 shrink-0 rounded-full transition-colors disabled:opacity-40 ${
        checked ? "bg-accent" : "bg-line-strong"
      }`}
    >
      <span
        className={`absolute top-0.5 h-4 w-4 rounded-full transition-all ${
          checked ? "left-[18px] bg-accent-ink" : "left-0.5 bg-muted"
        }`}
      />
    </button>
  );
}

type Tone = "neutral" | "accent" | "ok" | "danger" | "info";
const tones: Record<Tone, string> = {
  neutral: "bg-raised text-muted border-line-strong",
  accent: "bg-accent-soft text-accent border-accent/30",
  ok: "bg-ok-soft text-ok border-ok/30",
  danger: "bg-danger-soft text-danger border-danger/30",
  info: "bg-info-soft text-info border-info/30",
};

export function Badge({ tone = "neutral", children, title }: { tone?: Tone; children: ReactNode; title?: string }) {
  return (
    <span
      title={title}
      className={`inline-flex h-5 items-center gap-1 rounded border px-1.5 font-mono text-[10.5px] uppercase tracking-wide ${tones[tone]}`}
    >
      {children}
    </span>
  );
}

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className = "", ...rest } = props;
  return (
    <input
      className={`h-9 w-full rounded-md border border-line-strong bg-bg px-3 text-sm text-text placeholder:text-dim focus:border-accent focus:outline-none ${className}`}
      {...rest}
    />
  );
}

export function Field({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-[13px] font-medium text-text">{label}</span>
      {children}
      {hint && <span className="mt-1.5 block text-xs leading-relaxed text-muted">{hint}</span>}
    </label>
  );
}

export function SectionTitle({ children, right }: { children: ReactNode; right?: ReactNode }) {
  return (
    <div className="mb-3 flex items-center justify-between gap-3">
      <h2 className="label-caps text-[15px] text-muted">{children}</h2>
      {right}
    </div>
  );
}

export function Modal({
  title,
  onClose,
  children,
  footer,
  width = "max-w-2xl",
}: {
  title: ReactNode;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  width?: string;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 p-6" onMouseDown={onClose}>
      <div
        role="dialog"
        aria-modal="true"
        className={`flex max-h-full w-full ${width} flex-col overflow-hidden rounded-xl border border-line-strong bg-panel shadow-2xl`}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3.5">
          <h2 className="font-display text-xl font-semibold tracking-wide">{title}</h2>
          <IconButton label={t("Schließen")} onClick={onClose}>
            <X size={18} />
          </IconButton>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">{children}</div>
        {footer && <div className="flex justify-end gap-2 border-t border-line px-5 py-3">{footer}</div>}
      </div>
    </div>
  );
}

export function Progress({ value }: { value: number | null }) {
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-line">
      <div
        className={`h-full rounded-full bg-accent transition-[width] ${value === null ? "w-1/3 pulse-dot" : ""}`}
        style={value === null ? undefined : { width: `${Math.max(2, Math.min(100, value * 100))}%` }}
      />
    </div>
  );
}

export function Empty({ icon, title, children }: { icon: ReactNode; title: string; children?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center px-6 py-16 text-center">
      <div className="mb-4 text-dim">{icon}</div>
      <h3 className="font-display text-2xl font-semibold tracking-wide text-text">{title}</h3>
      {children && <div className="mt-2 max-w-md text-sm leading-relaxed text-muted">{children}</div>}
    </div>
  );
}

export function formatBytes(n: number) {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

export function formatDate(secs: number) {
  if (!secs) return "–";
  return new Date(secs * 1000).toLocaleString(locale(), { dateStyle: "medium", timeStyle: "short" });
}

export const platformLabel: Record<string, string> = {
  steam: "Steam",
  gamePass: "Game Pass / Xbox",
  gog: "GOG",
  epic: "Epic",
  unknown: "Unbekannt",
};

/** Labels are German source strings – render them through t(). */
export const kindLabel: Record<string, { label: string; title: string }> = {
  pak: { label: "PAK", title: "Pak-Mod (~mods-Ordner)" },
  zoneKit: { label: "Zone Kit", title: "Offizielles Mod-Format (Stalker2\\Mods)" },
  ue4ss: { label: "UE4SS", title: "UE4SS-Lua-Mod" },
  binary: { label: "DLL", title: "Dateien im Binaries-Ordner (z. B. Plugins, UE4SS)" },
  gameFile: { label: "Datei", title: "Ersetzt Dateien im Spielordner" },
};
