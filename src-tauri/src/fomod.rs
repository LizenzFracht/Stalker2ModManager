//! FOMOD installer support (fomod/ModuleConfig.xml): install steps, option groups,
//! plugin images/descriptions and flag-based conditional installs.

use crate::tr;

use crate::model::{Conditional, FileMap, GroupKind, ModLayout, ModOption, OptionGroup};
use anyhow::{Context, Result};
use roxmltree::{Document, Node};
use std::path::Path;

pub struct Fomod {
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub layout: ModLayout,
}

fn decode(bytes: &[u8]) -> String {
    let text = if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        let le = bytes[0] == 0xFF;
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) })
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        let b = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
        String::from_utf8_lossy(b).into_owned()
    };
    // roxmltree only takes UTF-8 text; drop the declaration that may claim UTF-16.
    match (text.find("<?xml"), text.find("?>")) {
        (Some(0), Some(end)) => text[end + 2..].to_string(),
        _ => text.trim_start_matches('\u{feff}').to_string(),
    }
}

fn is(n: &Node, name: &str) -> bool {
    n.is_element() && n.tag_name().name().eq_ignore_ascii_case(name)
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| is(c, name))
}

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(move |c| is(c, name))
}

fn attr(n: &Node, name: &str) -> Option<String> {
    n.attributes()
        .find(|a| a.name().eq_ignore_ascii_case(name))
        .map(|a| a.value().to_string())
}

fn norm(p: &str) -> String {
    p.replace('\\', "/").trim_matches('/').to_string()
}

fn file_list(node: Option<Node>) -> Vec<FileMap> {
    let Some(node) = node else { return Vec::new() };
    let mut out = Vec::new();
    for n in node.children().filter(|c| c.is_element()) {
        let src = norm(&attr(&n, "source").unwrap_or_default());
        let priority = attr(&n, "priority").and_then(|p| p.parse().ok()).unwrap_or(0);
        if is(&n, "file") {
            let dest = match attr(&n, "destination") {
                None => src.clone(),
                Some(d) if d.trim().is_empty() => src.rsplit('/').next().unwrap_or(&src).to_string(),
                Some(d) if d.ends_with('/') || d.ends_with('\\') => {
                    format!("{}/{}", norm(&d), src.rsplit('/').next().unwrap_or(&src))
                }
                Some(d) => norm(&d),
            };
            out.push(FileMap { src, dest, priority });
        } else if is(&n, "folder") {
            let dest = attr(&n, "destination").map(|d| norm(&d)).unwrap_or_default();
            out.push(FileMap { src, dest, priority });
        }
    }
    out
}

fn group_kind(s: &str) -> GroupKind {
    match s {
        "SelectExactlyOne" => GroupKind::ExactlyOne,
        "SelectAtMostOne" => GroupKind::AtMostOne,
        "SelectAtLeastOne" => GroupKind::AtLeastOne,
        "SelectAll" => GroupKind::All,
        _ => GroupKind::Any,
    }
}

fn flags(node: Option<Node>) -> Vec<(String, String)> {
    let Some(node) = node else { return Vec::new() };
    children(node, "flag")
        .map(|f| (attr(&f, "name").unwrap_or_default(), f.text().unwrap_or("").trim().to_string()))
        .collect()
}

fn plugin_type(plugin: Node) -> String {
    let Some(td) = child(plugin, "typeDescriptor") else { return "Optional".into() };
    if let Some(t) = child(td, "type") {
        return attr(&t, "name").unwrap_or_default();
    }
    child(td, "dependencyType")
        .and_then(|d| child(d, "defaultType"))
        .and_then(|t| attr(&t, "name"))
        .unwrap_or_else(|| "Optional".into())
}

pub fn parse(xml_path: &Path) -> Result<Fomod> {
    let text = decode(&std::fs::read(xml_path)?);
    let doc = Document::parse_with_options(
        &text,
        roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() },
    )
    .context(tr!("FOMOD ModuleConfig.xml ist ungültig", "FOMOD ModuleConfig.xml is invalid"))?;
    let root = doc.root_element();

    let mut layout = ModLayout { fomod: true, ..Default::default() };
    layout.base = file_list(child(root, "requiredInstallFiles"));

    if let Some(steps) = child(root, "installSteps") {
        for (si, step) in children(steps, "installStep").enumerate() {
            let step_name = attr(&step, "name");
            let Some(groups) = child(step, "optionalFileGroups") else { continue };
            for (gi, group) in children(groups, "group").enumerate() {
                let gid = format!("s{si}g{gi}");
                let kind = group_kind(&attr(&group, "type").unwrap_or_default());
                let mut options = Vec::new();
                if let Some(plugins) = child(group, "plugins") {
                    for (pi, plugin) in children(plugins, "plugin").enumerate() {
                        let ty = plugin_type(plugin);
                        if ty == "NotUsable" {
                            continue;
                        }
                        options.push(ModOption {
                            id: format!("{gid}o{pi}"),
                            name: attr(&plugin, "name").unwrap_or_else(|| format!("Option {}", pi + 1)),
                            description: child(plugin, "description")
                                .and_then(|d| d.text())
                                .unwrap_or("")
                                .trim()
                                .to_string(),
                            image: child(plugin, "image").and_then(|i| attr(&i, "path")).map(|p| norm(&p)),
                            files: file_list(child(plugin, "files")),
                            default_selected: matches!(ty.as_str(), "Required" | "Recommended"),
                            flags: flags(child(plugin, "conditionFlags")),
                        });
                    }
                }
                if !options.is_empty() {
                    layout.groups.push(OptionGroup {
                        id: gid,
                        name: attr(&group, "name").unwrap_or_else(|| "Optionen".into()),
                        step: step_name.clone(),
                        kind,
                        options,
                    });
                }
            }
        }
    }

    if let Some(cond) = child(root, "conditionalFileInstalls").and_then(|c| child(c, "patterns")) {
        for pattern in children(cond, "pattern") {
            let deps = child(pattern, "dependencies");
            let all = deps.and_then(|d| attr(&d, "operator")).map(|o| o != "Or").unwrap_or(true);
            let flags = deps
                .map(|d| {
                    children(d, "flagDependency")
                        .map(|f| (attr(&f, "flag").unwrap_or_default(), attr(&f, "value").unwrap_or_default()))
                        .collect()
                })
                .unwrap_or_default();
            layout.conditionals.push(Conditional { all, flags, files: file_list(child(pattern, "files")) });
        }
    }

    // info.xml next to ModuleConfig.xml carries the display metadata.
    let mut info = (None, None, None);
    if let Some(dir) = xml_path.parent() {
        if let Some(p) = crate::util::resolve_ci(dir, "info.xml") {
            if let Ok(bytes) = std::fs::read(p) {
                let t = decode(&bytes);
                if let Ok(d) = Document::parse(&t) {
                    let r = d.root_element();
                    let get = |n: &str| child(r, n).and_then(|c| c.text()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
                    info = (get("Name"), get("Author"), get("Version"));
                }
            }
        }
    }
    let module_name = child(root, "moduleName").and_then(|n| n.text()).map(|s| s.trim().to_string());

    Ok(Fomod {
        name: info.0.or(module_name).filter(|s| !s.is_empty()),
        author: info.1,
        version: info.2,
        layout,
    })
}
