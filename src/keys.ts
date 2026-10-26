import { t } from "./i18n";

// Key name conversions between the browser, UE4SS (`Key.F5`), Unreal FKey names (`F5`,
// `SpaceBar`, `NumPadOne`) and Windows virtual-key codes (`0x74`).

type Names = { ue4ss: string; fkey: string; vk: number };

const special: Record<string, Names> = {
  Space: { ue4ss: "SPACE", fkey: "SpaceBar", vk: 0x20 },
  Enter: { ue4ss: "RETURN", fkey: "Enter", vk: 0x0d },
  NumpadEnter: { ue4ss: "RETURN", fkey: "Enter", vk: 0x0d },
  Escape: { ue4ss: "ESCAPE", fkey: "Escape", vk: 0x1b },
  Tab: { ue4ss: "TAB", fkey: "Tab", vk: 0x09 },
  Backspace: { ue4ss: "BACKSPACE", fkey: "BackSpace", vk: 0x08 },
  Insert: { ue4ss: "INS", fkey: "Insert", vk: 0x2d },
  Delete: { ue4ss: "DEL", fkey: "Delete", vk: 0x2e },
  Home: { ue4ss: "HOME", fkey: "Home", vk: 0x24 },
  End: { ue4ss: "END", fkey: "End", vk: 0x23 },
  PageUp: { ue4ss: "PAGE_UP", fkey: "PageUp", vk: 0x21 },
  PageDown: { ue4ss: "PAGE_DOWN", fkey: "PageDown", vk: 0x22 },
  ArrowLeft: { ue4ss: "LEFT_ARROW", fkey: "Left", vk: 0x25 },
  ArrowUp: { ue4ss: "UP_ARROW", fkey: "Up", vk: 0x26 },
  ArrowRight: { ue4ss: "RIGHT_ARROW", fkey: "Right", vk: 0x27 },
  ArrowDown: { ue4ss: "DOWN_ARROW", fkey: "Down", vk: 0x28 },
  CapsLock: { ue4ss: "CAPS_LOCK", fkey: "CapsLock", vk: 0x14 },
  Pause: { ue4ss: "PAUSE", fkey: "Pause", vk: 0x13 },
  ScrollLock: { ue4ss: "SCROLL_LOCK", fkey: "ScrollLock", vk: 0x91 },
  NumLock: { ue4ss: "NUM_LOCK", fkey: "NumLock", vk: 0x90 },
  NumpadAdd: { ue4ss: "ADD", fkey: "Add", vk: 0x6b },
  NumpadSubtract: { ue4ss: "SUBTRACT", fkey: "Subtract", vk: 0x6d },
  NumpadMultiply: { ue4ss: "MULTIPLY", fkey: "Multiply", vk: 0x6a },
  NumpadDivide: { ue4ss: "DIVIDE", fkey: "Divide", vk: 0x6f },
  NumpadDecimal: { ue4ss: "DECIMAL", fkey: "Decimal", vk: 0x6e },
  Minus: { ue4ss: "OEM_MINUS", fkey: "Hyphen", vk: 0xbd },
  Equal: { ue4ss: "OEM_PLUS", fkey: "Equals", vk: 0xbb },
  Comma: { ue4ss: "OEM_COMMA", fkey: "Comma", vk: 0xbc },
  Period: { ue4ss: "OEM_PERIOD", fkey: "Period", vk: 0xbe },
};

const digitWords = ["ZERO", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE"];
const cap = (s: string) => s[0] + s.slice(1).toLowerCase();

export function namesFor(code: string): Names | null {
  if (special[code]) return special[code];
  let m = /^Key([A-Z])$/.exec(code);
  if (m) return { ue4ss: m[1], fkey: m[1], vk: m[1].charCodeAt(0) };
  m = /^Digit(\d)$/.exec(code);
  if (m) {
    const d = Number(m[1]);
    return { ue4ss: digitWords[d], fkey: cap(digitWords[d]), vk: 0x30 + d };
  }
  m = /^Numpad(\d)$/.exec(code);
  if (m) {
    const d = Number(m[1]);
    return { ue4ss: `NUM_${digitWords[d]}`, fkey: `NumPad${cap(digitWords[d])}`, vk: 0x60 + d };
  }
  m = /^F(\d{1,2})$/.exec(code);
  if (m) return { ue4ss: code, fkey: code, vk: 0x6f + Number(m[1]) };
  return null;
}

/** Name in the format a binding uses. */
export function keyForStyle(code: string, source: string, style: string): string | null {
  const n = namesFor(code);
  if (!n) return null;
  if (source === "mcm") return n.fkey;
  if (style === "ue4ss") return n.ue4ss;
  if (style === "vk") return `0x${n.vk.toString(16).toUpperCase().padStart(2, "0")}`;
  return n.fkey;
}

const aliases: Record<string, string> = {
  SPACEBAR: "SPACE", ENTER: "RETURN", INSERT: "INS", DELETE: "DEL", LEFT: "LEFTARROW", RIGHT: "RIGHTARROW",
  UP: "UPARROW", DOWN: "DOWNARROW", HYPHEN: "OEMMINUS", EQUALS: "OEMPLUS", COMMA: "OEMCOMMA", PERIOD: "OEMPERIOD",
  LEFTSHIFT: "LEFTSHIFT", ADD: "ADD",
};

/** Canonical form for conflict detection across styles. */
export function canonicalKey(key: string): string {
  if (/^0x[0-9a-f]+$/i.test(key)) {
    const vk = parseInt(key, 16);
    for (const code of [...Object.keys(special), ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").map((c) => `Key${c}`), ...[...Array(10).keys()].map((d) => `Digit${d}`), ...[...Array(24).keys()].map((i) => `F${i + 1}`), ...[...Array(10).keys()].map((d) => `Numpad${d}`)]) {
      const n = namesFor(code);
      if (n?.vk === vk) return canonicalKey(n.ue4ss);
    }
    return key.toUpperCase();
  }
  let k = key.toUpperCase().replace(/_/g, "");
  const num = /^NUMPAD(ZERO|ONE|TWO|THREE|FOUR|FIVE|SIX|SEVEN|EIGHT|NINE)$/.exec(k);
  if (num) k = `NUM${num[1]}`;
  return aliases[k] ?? k;
}

const labels: Record<string, string> = {
  SPACE: "Leertaste", RETURN: "Enter", ESCAPE: "Esc", BACKSPACE: "Rücktaste", INS: "Einfg", DEL: "Entf", HOME: "Pos1",
  END: "Ende", PAGEUP: "Bild ↑", PAGEDOWN: "Bild ↓", LEFTARROW: "←", RIGHTARROW: "→", UPARROW: "↑", DOWNARROW: "↓",
  LEFTSHIFT: "Shift links", TAB: "Tab", CAPSLOCK: "Feststell", ZERO: "0", ONE: "1", TWO: "2", THREE: "3", FOUR: "4",
  FIVE: "5", SIX: "6", SEVEN: "7", EIGHT: "8", NINE: "9", CONTROL: "Strg", SHIFT: "Shift", ALT: "Alt",
};

export function keyLabel(key: string): string {
  const c = canonicalKey(key);
  const num = /^NUM(ZERO|ONE|TWO|THREE|FOUR|FIVE|SIX|SEVEN|EIGHT|NINE)$/.exec(c);
  if (num) return `Num ${labels[num[1]]}`;
  return labels[c] ? t(labels[c]) : c.length <= 3 ? c : cap(key.replace(/_/g, " "));
}

export function comboId(key: string, modifiers: string[]) {
  return [...modifiers.map((m) => m.toUpperCase()).sort(), canonicalKey(key)].join("+");
}
