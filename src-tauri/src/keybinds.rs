//! Finds key bindings declared by mods (UE4SS Lua `RegisterKeyBind`, `Key.X` config
//! assignments, hotkey entries in ini/json/lua configs) and rewrites them on rebind.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub id: String,
    /// ue4ss | config | mcm | game
    pub source: String,
    pub mod_id: Option<String>,
    pub mod_name: String,
    pub enabled: bool,
    /// File relative to the mod's files folder.
    pub file: Option<String>,
    pub line: usize,
    pub action: String,
    pub key: String,
    pub modifiers: Vec<String>,
    /// ue4ss (Key.F5 + ModifierKey.X) | plain (F5) | quoted ("F5") | vk (0x74)
    pub style: String,
    pub editable: bool,
}

/// Key names UE4SS understands (Key.<NAME>) mapped to readable labels.
const UE4SS_KEYS: &[&str] = &[
    "LEFT_MOUSE_BUTTON", "RIGHT_MOUSE_BUTTON", "MIDDLE_MOUSE_BUTTON", "XBUTTON_ONE", "XBUTTON_TWO", "BACKSPACE", "TAB",
    "RETURN", "PAUSE", "CAPS_LOCK", "ESCAPE", "SPACE", "PAGE_UP", "PAGE_DOWN", "END", "HOME", "LEFT_ARROW", "UP_ARROW",
    "RIGHT_ARROW", "DOWN_ARROW", "PRINT_SCREEN", "INS", "DEL", "ZERO", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX",
    "SEVEN", "EIGHT", "NINE", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "NUM_ZERO", "NUM_ONE", "NUM_TWO", "NUM_THREE", "NUM_FOUR", "NUM_FIVE", "NUM_SIX",
    "NUM_SEVEN", "NUM_EIGHT", "NUM_NINE", "MULTIPLY", "ADD", "SUBTRACT", "DECIMAL", "DIVIDE", "F1", "F2", "F3", "F4", "F5",
    "F6", "F7", "F8", "F9", "F10", "F11", "F12", "F13", "F14", "F15", "F16", "F17", "F18", "F19", "F20", "F21", "F22",
    "F23", "F24", "NUM_LOCK", "SCROLL_LOCK", "OEM_ONE", "OEM_PLUS", "OEM_COMMA", "OEM_MINUS", "OEM_PERIOD", "OEM_TWO",
    "OEM_THREE", "OEM_FOUR", "OEM_FIVE", "OEM_SIX", "OEM_SEVEN",
];

/// Plain key names commonly used in ini/json configs (UE FKey style).
const PLAIN_KEYS: &[&str] = &[
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "Insert", "Delete", "Home", "End",
    "PageUp", "PageDown", "NumPadZero", "NumPadOne", "NumPadTwo", "NumPadThree", "NumPadFour", "NumPadFive", "NumPadSix",
    "NumPadSeven", "NumPadEight", "NumPadNine", "SpaceBar", "Enter", "Tab", "Escape", "LeftShift", "RightShift",
    "LeftControl", "RightControl", "LeftAlt", "RightAlt", "BackSpace", "CapsLock", "Up", "Down", "Left", "Right", "Zero",
    "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Add", "Subtract", "Multiply", "Divide",
    "Decimal", "MiddleMouseButton", "ThumbMouseButton", "ThumbMouseButton2",
];

fn is_plain_key(v: &str) -> bool {
    (v.len() == 1 && v.chars().all(|c| c.is_ascii_alphanumeric()))
        || PLAIN_KEYS.iter().any(|k| k.eq_ignore_ascii_case(v))
        || UE4SS_KEYS.iter().any(|k| k.eq_ignore_ascii_case(v))
}

fn ident_at(s: &str) -> &str {
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(s.len());
    &s[..end]
}

/// Parses `RegisterKeyBind(Key.X, {ModifierKey.A, ...}, callback)` starting at `rest`.
fn parse_register(rest: &str) -> Option<(String, Vec<String>, String)> {
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("Key.")?;
    let key = ident_at(rest).to_string();
    let mut after = rest[key.len()..].trim_start();
    let mut mods = Vec::new();
    if let Some(r) = after.strip_prefix(',') {
        let r = r.trim_start();
        if let Some(inner) = r.strip_prefix('{') {
            let close = inner.find('}')?;
            for part in inner[..close].split(',') {
                if let Some(m) = part.trim().strip_prefix("ModifierKey.") {
                    mods.push(ident_at(m).to_string());
                }
            }
            after = inner[close + 1..].trim_start();
            after = after.strip_prefix(',').unwrap_or(after);
        } else {
            after = r;
        }
    }
    let after = after.trim_start();
    let action = if after.starts_with("function") { String::new() } else { ident_at(after).to_string() };
    Some((key, mods, action))
}

fn hotkey_name(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("key") || n.contains("hotkey") || n.contains("bind") || n.contains("toggle") || n.contains("button")
}

/// Scans one text file of a mod.
pub fn scan_file(text: &str, rel: &str, mod_id: &str, mod_name: &str, enabled: bool, out: &mut Vec<Binding>) {
    let ext = crate::util::ext_lower(rel);
    let mut pending_comment = String::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        let mut push = |source: &str, action: String, key: String, modifiers: Vec<String>, style: &str| {
            out.push(Binding {
                id: format!("{mod_id}:{rel}:{i}:{}", out.len()),
                source: source.into(),
                mod_id: Some(mod_id.into()),
                mod_name: mod_name.into(),
                enabled,
                file: Some(rel.into()),
                line: i + 1,
                action,
                key,
                modifiers,
                style: style.into(),
                editable: true,
            });
        };
        if ext == "lua" {
            if let Some(c) = t.strip_prefix("--") {
                pending_comment = c.trim().to_string();
                continue;
            }
            if let Some(pos) = t.find("RegisterKeyBind(") {
                if let Some((key, mods, action)) = parse_register(&t[pos + "RegisterKeyBind(".len()..]) {
                    let action = if !action.is_empty() {
                        action
                    } else if !pending_comment.is_empty() {
                        pending_comment.clone()
                    } else {
                        format!("Zeile {}", i + 1)
                    };
                    push("ue4ss", action, key, mods, "ue4ss");
                }
            } else if let Some((lhs, rhs)) = t.split_once('=') {
                let lhs = lhs.trim().trim_start_matches("local ").trim();
                let rhs = rhs.trim().trim_end_matches([',', ';']).trim();
                if let Some(k) = rhs.strip_prefix("Key.") {
                    push("config", lhs.to_string(), ident_at(k).to_string(), Vec::new(), "ue4ss");
                } else if hotkey_name(lhs) {
                    let unq = rhs.trim_matches(['"', '\'']);
                    if unq.len() != rhs.len() && is_plain_key(unq) {
                        push("config", lhs.to_string(), unq.to_string(), Vec::new(), "quoted");
                    }
                }
            }
            pending_comment.clear();
        } else if matches!(ext.as_str(), "ini" | "cfg" | "txt" | "toml") {
            if t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
                continue;
            }
            if let Some((lhs, rhs)) = t.split_once('=') {
                let (lhs, rhs) = (lhs.trim(), rhs.trim());
                if !hotkey_name(lhs) {
                    continue;
                }
                let unq = rhs.trim_matches(['"', '\'']);
                if is_plain_key(unq) {
                    push("config", lhs.to_string(), unq.to_string(), Vec::new(), if unq.len() != rhs.len() { "quoted" } else { "plain" });
                } else if unq.starts_with("0x") && u32::from_str_radix(&unq[2..], 16).is_ok() {
                    push("config", lhs.to_string(), unq.to_string(), Vec::new(), "vk");
                }
            }
        } else if ext == "json" {
            // "ToggleKey": "F5"
            if let Some((lhs, rhs)) = t.split_once(':') {
                let lhs = lhs.trim().trim_matches('"');
                let rhs = rhs.trim().trim_end_matches(',').trim();
                let unq = rhs.trim_matches('"');
                if hotkey_name(lhs) && unq.len() != rhs.len() && is_plain_key(unq) {
                    push("config", lhs.to_string(), unq.to_string(), Vec::new(), "quoted");
                }
            }
        }
    }
}

/// Rewrites a binding in its file. `key` is in the binding's own style.
pub fn rebind(path: &Path, b: &Binding, key: &str, modifiers: &[String]) -> Result<()> {
    let ok_key = key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !key.is_empty();
    if !ok_key {
        bail!("Ungültiger Tastenname: {key}");
    }
    let text = std::fs::read_to_string(path)?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let line = lines.get_mut(b.line - 1).ok_or_else(|| anyhow!("Zeile {} nicht gefunden", b.line))?;
    match (b.source.as_str(), b.style.as_str()) {
        ("ue4ss", _) => {
            let start = line.find("RegisterKeyBind(").ok_or_else(|| anyhow!("Datei wurde verändert"))? + "RegisterKeyBind(".len();
            let rest = &line[start..];
            let key_start = start + rest.find("Key.").ok_or_else(|| anyhow!("Datei wurde verändert"))?;
            let after_key = key_start + 4 + ident_at(&line[key_start + 4..]).len();
            // Drop an existing modifier table directly after the key.
            let tail = &line[after_key..];
            let mut tail_start = after_key;
            let trimmed = tail.trim_start();
            if let Some(r) = trimmed.strip_prefix(',') {
                if r.trim_start().starts_with('{') {
                    let brace = after_key + (tail.len() - trimmed.len()) + 1 + (r.len() - r.trim_start().len());
                    let close = line[brace..].find('}').ok_or_else(|| anyhow!("Datei wurde verändert"))?;
                    tail_start = brace + close + 1;
                }
            }
            let mods = if modifiers.is_empty() {
                String::new()
            } else {
                format!(", {{{}}}", modifiers.iter().map(|m| format!("ModifierKey.{m}")).collect::<Vec<_>>().join(", "))
            };
            *line = format!("{}Key.{key}{mods}{}", &line[..key_start], &line[tail_start..]);
        }
        (_, "ue4ss") => {
            let pos = line.find("Key.").ok_or_else(|| anyhow!("Datei wurde verändert"))?;
            let end = pos + 4 + ident_at(&line[pos + 4..]).len();
            *line = format!("{}Key.{key}{}", &line[..pos], &line[end..]);
        }
        _ => {
            // Replace the old value after the separator.
            let sep = line.find(['=', ':']).ok_or_else(|| anyhow!("Datei wurde verändert"))?;
            let value_pos = line[sep..].find(&b.key).map(|p| p + sep).ok_or_else(|| anyhow!("Datei wurde verändert"))?;
            *line = format!("{}{key}{}", &line[..value_pos], &line[value_pos + b.key.len()..]);
        }
    }
    std::fs::write(path, lines.join(newline) + if text.ends_with('\n') { newline } else { "" })?;
    Ok(())
}

/// A small, conservative list of default game keys used for conflict hints.
pub fn game_defaults() -> Vec<Binding> {
    [
        ("Bewegen", "W"), ("Bewegen", "A"), ("Bewegen", "S"), ("Bewegen", "D"), ("Springen", "SPACE"),
        ("Ducken", "C"), ("Sprinten", "LEFT_SHIFT"), ("Nachladen", "R"), ("PDA / Inventar", "TAB"),
        ("Schnellspeichern", "F5"), ("Schnellladen", "F9"), ("Menü", "ESCAPE"), ("Waffe 1", "ONE"),
        ("Waffe 2", "TWO"), ("Waffe 3", "THREE"), ("Waffe 4", "FOUR"),
    ]
    .iter()
    .enumerate()
    .map(|(i, (action, key))| Binding {
        id: format!("game:{i}"),
        source: "game".into(),
        mod_id: None,
        mod_name: "Stalker 2 (Standard)".into(),
        enabled: true,
        file: None,
        line: 0,
        action: action.to_string(),
        key: key.to_string(),
        modifiers: Vec::new(),
        style: "ue4ss".into(),
        editable: false,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_rebinds_lua() {
        let lua = "local UEHelpers = require(\"UEHelpers\")\n-- Noclip umschalten\nRegisterKeyBind(Key.C, {ModifierKey.ALT}, function() end)\nRegisterKeyBind(Key.F6, toggleSpeed)\nlocal OpenKey = Key.F7\n";
        let mut out = Vec::new();
        scan_file(lua, "Scripts/main.lua", "m", "Mod", true, &mut out);
        assert_eq!(out.len(), 3);
        assert_eq!((out[0].key.as_str(), out[0].modifiers.clone(), out[0].action.as_str()), ("C", vec!["ALT".to_string()], "Noclip umschalten"));
        assert_eq!(out[1].action, "toggleSpeed");
        assert_eq!(out[2].action, "OpenKey");

        let tmp = std::env::temp_dir().join(format!("s2mm-kb-{}.lua", std::process::id()));
        std::fs::write(&tmp, lua).unwrap();
        rebind(&tmp, &out[0], "N", &["CONTROL".into(), "SHIFT".into()]).unwrap();
        rebind(&tmp, &out[1], "F8", &["ALT".into()]).unwrap();
        rebind(&tmp, &out[2], "HOME", &[]).unwrap();
        let text = std::fs::read_to_string(&tmp).unwrap();
        assert!(text.contains("RegisterKeyBind(Key.N, {ModifierKey.CONTROL, ModifierKey.SHIFT}, function() end)"), "{text}");
        assert!(text.contains("RegisterKeyBind(Key.F8, {ModifierKey.ALT}, toggleSpeed)"), "{text}");
        assert!(text.contains("local OpenKey = Key.HOME"), "{text}");
        std::fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn finds_ini_hotkeys() {
        let ini = "[Settings]\nToggleKey=F5\nMenuHotkey = \"Insert\"\nSpeed=2\nVkKey=0x74\n";
        let mut out = Vec::new();
        scan_file(ini, "config.ini", "m", "Mod", true, &mut out);
        let keys: Vec<_> = out.iter().map(|b| (b.action.as_str(), b.key.as_str(), b.style.as_str())).collect();
        assert_eq!(keys, vec![("ToggleKey", "F5", "plain"), ("MenuHotkey", "Insert", "quoted"), ("VkKey", "0x74", "vk")]);
    }
}
