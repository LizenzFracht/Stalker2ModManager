//! Automatic load order: frameworks first, large pak overhauls before small targeted
//! mods, patches/compat files last and directly after the mod they patch.

use crate::layout::TargetKind;
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct SortInput {
    pub id: String,
    pub name: String,
    pub kinds: Vec<TargetKind>,
    /// Number of assets inside the mod's paks.
    pub assets: usize,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SortEntry {
    pub id: String,
    pub from: usize,
    pub to: usize,
    pub reason: String,
}

const FRAMEWORK_WORDS: [&str; 8] = [
    "ue4ss", "mod configuration menu", "mcm", "framework", "library", "loader", "api", "s2mm live-map",
];
const PATCH_WORDS: [&str; 9] = ["patch", "fix", "compat", "kompat", "addon", "add-on", "hotfix", "tweak for", "merge"];

fn normalize(s: &str) -> String {
    s.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn has_word(name: &str, words: &[&str]) -> bool {
    let n = format!(" {} ", normalize(name));
    words.iter().any(|w| n.contains(&format!(" {} ", normalize(w))) || (w.len() > 4 && n.contains(&normalize(w))))
}

/// Returns (rank, reason). Lower ranks load first (and lose conflicts).
fn classify(m: &SortInput) -> (u8, &'static str) {
    let paky = m.kinds.iter().any(|k| matches!(k, TargetKind::Pak | TargetKind::ZoneKit));
    if has_word(&m.name, &FRAMEWORK_WORDS) {
        return (0, "Grundlage/Framework");
    }
    if !paky && !m.kinds.is_empty() {
        return (1, "Skript-/DLL-Mod ohne Pak-Konflikte");
    }
    if has_word(&m.name, &PATCH_WORDS) {
        return (3, "Patch/Kompatibilität – muss gewinnen");
    }
    (2, "Pak-Mod")
}

pub fn sort(mods: &[SortInput]) -> Vec<SortEntry> {
    let mut items: Vec<(usize, &SortInput, u8, &'static str)> =
        mods.iter().enumerate().map(|(i, m)| { let (r, why) = classify(m); (i, m, r, why) }).collect();
    // Stable: rank, then bigger overhauls first within pak mods, then current order.
    items.sort_by(|a, b| {
        a.2.cmp(&b.2)
            .then_with(|| if a.2 == 2 { b.1.assets.cmp(&a.1.assets) } else { std::cmp::Ordering::Equal })
            .then(a.0.cmp(&b.0))
    });
    let mut order: Vec<(usize, &SortInput, String)> = items
        .into_iter()
        .map(|(i, m, r, why)| {
            let reason = if r == 2 && m.assets > 0 { format!("{why} · {} Assets", m.assets) } else { why.to_string() };
            (i, m, reason)
        })
        .collect();

    // A patch that names another mod goes directly after it.
    let names: Vec<(String, String)> = mods.iter().map(|m| (m.id.clone(), normalize(&m.name))).collect();
    let patches: Vec<String> = order.iter().filter(|(_, m, _)| classify(m).0 == 3).map(|(_, m, _)| m.id.clone()).collect();
    for pid in patches {
        let pname = normalize(&mods.iter().find(|m| m.id == pid).unwrap().name);
        let parent = names
            .iter()
            .filter(|(id, n)| *id != pid && n.len() >= 4 && pname.contains(n.as_str()))
            .max_by_key(|(_, n)| n.len())
            .map(|(id, _)| id.clone());
        if let Some(parent) = parent {
            let from = order.iter().position(|(_, m, _)| m.id == pid).unwrap();
            let entry = order.remove(from);
            let at = order.iter().position(|(_, m, _)| m.id == parent).unwrap() + 1;
            let parent_name = mods.iter().find(|m| m.id == parent).unwrap().name.clone();
            order.insert(at, (entry.0, entry.1, format!("Patch für „{parent_name}“ – direkt danach")));
        }
    }

    order
        .into_iter()
        .enumerate()
        .map(|(to, (from, m, reason))| SortEntry { id: m.id.clone(), from, to, reason })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, name: &str, kinds: &[TargetKind], assets: usize) -> SortInput {
        SortInput { id: id.into(), name: name.into(), kinds: kinds.to_vec(), assets }
    }

    #[test]
    fn sorts_frameworks_overhauls_patches() {
        let mods = vec![
            m("p", "Better Stamina - Compat Patch", &[TargetKind::Pak], 2),
            m("small", "Better Stamina", &[TargetKind::Pak], 3),
            m("big", "Gameplay Overhaul", &[TargetKind::Pak], 400),
            m("lua", "Lua Tweaks", &[TargetKind::Ue4ss], 0),
            m("u", "UE4SS", &[TargetKind::Binary], 0),
            m("fix", "Weight Fix", &[TargetKind::Pak], 1),
        ];
        let order: Vec<String> = sort(&mods).into_iter().map(|e| e.id).collect();
        assert_eq!(order, vec!["u", "lua", "big", "small", "p", "fix"]);
    }
}
