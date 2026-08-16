//! Shared helpers for parsing freedesktop .desktop files.

/// The display name and icon of a desktop entry.
pub struct DesktopEntry {
    pub name: String,
    pub icon: Option<String>,
}

/// Whether a .desktop file describes a visible, launchable application.
pub fn is_visible_app(content: &str) -> bool {
    let mut in_entry = false;
    let mut is_application = true;
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
        match key.trim() {
            "NoDisplay" | "Hidden" if value.trim().eq_ignore_ascii_case("true") => return false,
            "Type" => is_application = value.trim().eq_ignore_ascii_case("Application"),
            _ => {}
        }
    }
    is_application
}

/// Parse the display Name and Icon of a desktop entry, returning None when the
/// entry is not a visible application.
pub fn parse_entry(content: &str) -> Option<DesktopEntry> {
    if !is_visible_app(content) {
        return None;
    }
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
            "Name" if name.is_none() && !value.is_empty() => name = Some(value.to_string()),
            "Icon" if icon.is_none() && !value.is_empty() => icon = Some(value.to_string()),
            _ => {}
        }
    }
    Some(DesktopEntry {
        name: name.unwrap_or_default(),
        icon,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_visible_apps() {
        let app = "[Desktop Entry]\nType=Application\nName=Foo\n";
        assert!(is_visible_app(app));

        let no_display = "[Desktop Entry]\nType=Application\nNoDisplay=true\n";
        assert!(!is_visible_app(no_display));

        let hidden = "[Desktop Entry]\nType=Application\nHidden=true\n";
        assert!(!is_visible_app(hidden));

        let link = "[Desktop Entry]\nType=Link\n";
        assert!(!is_visible_app(link));
    }

    #[test]
    fn parses_name_and_icon() {
        let content = "[Desktop Entry]\nType=Application\nName=Firefox\nIcon=firefox\n";
        let entry = parse_entry(content).unwrap();
        assert_eq!(entry.name, "Firefox");
        assert_eq!(entry.icon.as_deref(), Some("firefox"));
    }

    #[test]
    fn prefers_unlocalized_name_and_icon() {
        let content = "[Desktop Entry]\nType=Application\nName=Files\nName[fr]=Fichiers\nIcon=org.gnome.Nautilus\n";
        let entry = parse_entry(content).unwrap();
        assert_eq!(entry.name, "Files");
        assert_eq!(entry.icon.as_deref(), Some("org.gnome.Nautilus"));
    }

    #[test]
    fn rejects_invisible_entries() {
        let no_display = "[Desktop Entry]\nType=Application\nName=X\nNoDisplay=true\n";
        assert!(parse_entry(no_display).is_none());
    }

    #[test]
    fn empty_name_when_absent() {
        let entry = parse_entry("[Desktop Entry]\nType=Application\n").unwrap();
        assert_eq!(entry.name, "");
        assert_eq!(entry.icon, None);
    }
}
