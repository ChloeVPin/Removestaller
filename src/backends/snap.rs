use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::backend::{binary_available, process_error, Backend, BackendError};
use crate::backends::desktop::is_visible_app;
use crate::model::App;

pub struct SnapBackend;

impl Backend for SnapBackend {
    fn id(&self) -> &'static str {
        "snap"
    }

    fn detect(&self) -> bool {
        binary_available("snap")
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let versions = snap_versions()?;
        Ok(snap_apps(&versions))
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        let output = Command::new("snap")
            .args(["remove", &app.id])
            .output()
            .map_err(|e| BackendError(format!("failed to run snap: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(process_error(&format!("snap remove {}", app.id), &output))
        }
    }
}

/// Snap name -> version, parsed from `snap list`.
fn snap_versions() -> Result<HashMap<String, String>, BackendError> {
    let output = Command::new("snap")
        .arg("list")
        .output()
        .map_err(|e| BackendError(format!("failed to run snap: {e}")))?;

    if !output.status.success() {
        return Err(BackendError(format!(
            "snap list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let mut map = HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 2 {
            map.insert(cols[0].to_string(), cols[1].to_string());
        }
    }
    Ok(map)
}

/// Visible snap desktop entries, deduplicated per snap, preferring the
/// canonical entry whose desktop file stem matches the snap name.
fn snap_desktop_entries() -> Vec<(String, String, Option<String>)> {
    let mut entries: Vec<(String, String, Option<String>, bool)> = Vec::new();

    let Ok(dir) = std::fs::read_dir("/var/lib/snapd/desktop/applications") else {
        return Vec::new();
    };

    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "desktop").unwrap_or(false) {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Some((snap, name, icon)) = parse_snap_desktop(&content) else {
                continue;
            };
            let canonical = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .map(|stem| {
                    stem.split_once('_')
                        .map(|(_, rest)| rest == snap)
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            entries.push((snap, name, icon, canonical));
        }
    }

    // Canonical entries first so the first entry seen per snap is the main app.
    entries.sort_by_key(|(_, _, _, canonical)| !canonical);
    entries
        .into_iter()
        .map(|(snap, name, icon, _)| (snap, name, icon))
        .collect()
}

/// Extract the snap instance name, display Name, and Icon from an exported
/// snap desktop file.
fn parse_snap_desktop(content: &str) -> Option<(String, String, Option<String>)> {
    if !is_visible_app(content) {
        return None;
    }
    let mut snap: Option<String> = None;
    let mut name: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut in_entry = false;
    for line in content.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            in_entry = true;
            continue;
        }
        if line.starts_with('[') {
            in_entry = false;
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "X-SnapInstanceName" if snap.is_none() && !value.is_empty() => {
                snap = Some(value.to_string())
            }
            "Name" if name.is_none() && !value.is_empty() => name = Some(value.to_string()),
            "Icon" if icon.is_none() && !value.is_empty() => icon = Some(value.to_string()),
            _ => {}
        }
    }
    let snap = snap?;
    Some((snap, name.unwrap_or_default(), icon))
}

/// Build the App list from visible snap desktop entries, dropping snaps with
/// no visible entry (bases, runtimes, and CLI-only snaps).
fn snap_apps(versions: &HashMap<String, String>) -> Vec<App> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for (snap, name, icon) in snap_desktop_entries() {
        if !versions.contains_key(&snap) || !seen.insert(snap.clone()) {
            continue;
        }
        let name = if name.is_empty() { snap.clone() } else { name };
        let mut app = App::new(snap.clone(), name, "snap");
        if let Some(v) = versions.get(&snap) {
            app = app.with_version(v.clone());
        }
        if let Some(icon) = icon {
            app = app.with_icon(icon);
        }
        apps.push(app);
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_snap_desktop() {
        let content = "[Desktop Entry]\nX-SnapInstanceName=snap-store\nType=Application\n\
                       Name=App Center\nIcon=/snap/snap-store/current/meta/gui/icon.png\n";
        let (snap, name, icon) = parse_snap_desktop(content).unwrap();
        assert_eq!(snap, "snap-store");
        assert_eq!(name, "App Center");
        assert_eq!(
            icon.as_deref(),
            Some("/snap/snap-store/current/meta/gui/icon.png")
        );
    }

    #[test]
    fn skips_hidden_snap_entries() {
        let content = "[Desktop Entry]\nX-SnapInstanceName=prompting-client\nType=Application\n\
                       NoDisplay=true\nName=Prompting Client\n";
        assert!(parse_snap_desktop(content).is_none());
    }
}
