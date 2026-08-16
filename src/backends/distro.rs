use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use crate::backend::{binary_available, process_error, Backend, BackendError};
use crate::backends::desktop::{parse_entry, DesktopEntry};
use crate::model::App;

/// Packages that must never be offered for removal, even if they ship a
/// visible desktop file, because removing them can break the system.
const CRITICAL_PACKAGES: &[&str] = &[
    "apt",
    "bash",
    "base-files",
    "coreutils",
    "dbus",
    "dpkg",
    "gdm3",
    "gnome-session",
    "gnome-shell",
    "grub-common",
    "grub-efi-amd64",
    "grub-pc",
    "libc6",
    "lightdm",
    "login",
    "network-manager",
    "passwd",
    "snapd",
    "sudo",
    "systemd",
    "systemd-sysv",
    "ubuntu-desktop",
    "ubuntu-desktop-minimal",
    "ubuntu-session",
    "xorg",
    "xserver-xorg",
    "xserver-xorg-core",
    "xwayland",
];

const CRITICAL_PREFIXES: &[&str] = &[
    "linux-image",
    "linux-headers",
    "linux-modules",
    "linux-generic",
    "linux-tools",
];

fn is_critical(name: &str) -> bool {
    CRITICAL_PACKAGES.contains(&name) || CRITICAL_PREFIXES.iter().any(|p| name.starts_with(p))
}

fn run_capture(program: &str, args: &[&str]) -> Result<String, BackendError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| BackendError(format!("failed to run {program}: {e}")))?;

    if !output.status.success() {
        return Err(BackendError(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Like run_capture, but exit code 1 (no matching files) is treated as an
/// empty result rather than an error.
fn run_capture_allow_missing(program: &str, args: &[&str]) -> Result<String, BackendError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| BackendError(format!("failed to run {program}: {e}")))?;

    if !output.status.success() && output.status.code() != Some(1) {
        return Err(BackendError(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a root command through pkexec, which shows the polkit auth prompt.
fn run_privileged(args: &[&str]) -> Result<(), BackendError> {
    let output = Command::new("pkexec")
        .args(args)
        .output()
        .map_err(|e| BackendError(format!("failed to run pkexec: {e}")))?;

    if output.status.success() {
        Ok(())
    } else if matches!(output.status.code(), Some(126) | Some(127)) {
        Err(BackendError("removal failed or was cancelled".to_string()))
    } else {
        Err(process_error("privileged package removal", &output))
    }
}

/// Every visible desktop file in the XDG application directories, paired with
/// its parsed display Name and Icon.
fn visible_desktop_entries() -> Vec<(String, DesktopEntry)> {
    let mut entries = Vec::new();
    for directory in application_dirs() {
        let Ok(dir) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(info) = parse_entry(&content) {
                        entries.push((path.to_string_lossy().to_string(), info));
                    }
                }
            }
        }
    }
    entries
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| PathBuf::from(home).join(".local/share"));
        dirs.push(data_home.join("applications"));
    }

    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });
    dirs.extend(
        data_dirs
            .into_iter()
            .filter(|path| path.is_absolute())
            .map(|path| path.join("applications")),
    );

    dirs
}

/// Parse `dpkg -S` output into a desktop file path -> package map.
fn parse_dpkg_owners(out: &str) -> HashMap<String, String> {
    let mut owners = HashMap::new();
    for line in out.lines() {
        let Some((owner, path)) = line.split_once(": ") else {
            continue;
        };
        let Some(pkg) = owner.split(':').next() else {
            continue;
        };
        owners.insert(path.trim().to_string(), pkg.to_string());
    }
    owners
}

/// Map desktop file path -> owning package using `dpkg -S`, which reports the
/// owner per path so unowned files cannot shift the alignment.
fn dpkg_owners(files: &[&str]) -> HashMap<String, String> {
    if files.is_empty() {
        return HashMap::new();
    }
    let mut argv: Vec<&str> = vec!["-S"];
    argv.extend_from_slice(files);
    let Ok(out) = run_capture_allow_missing("dpkg", &argv) else {
        return HashMap::new();
    };
    parse_dpkg_owners(&out)
}

/// Owning package for a single file on rpm systems.
fn rpm_owner(file: &str) -> Option<String> {
    let output = Command::new("rpm")
        .args(["-qf", "--qf", "%{NAME}", file])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let owner = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if owner.is_empty() {
        None
    } else {
        Some(owner)
    }
}

/// Owning package for a single file on pacman systems.
fn pacman_owner(file: &str) -> Option<String> {
    let output = Command::new("pacman").args(["-Qqo", file]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let owner = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if owner.is_empty() {
        None
    } else {
        Some(owner)
    }
}

/// Build Apps from desktop entries whose owning package is in `owners`,
/// using each entry's real Name and Icon instead of the raw package name.
fn build_apps(
    versions: &HashMap<String, String>,
    backend: &'static str,
    entries: &[(String, DesktopEntry)],
    owners: &HashMap<String, String>,
) -> Vec<App> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for (path, info) in entries {
        let Some(pkg) = owners.get(path) else {
            continue;
        };
        if is_critical(pkg) || !seen.insert(pkg.clone()) {
            continue;
        }
        let name = if info.name.is_empty() {
            pkg.clone()
        } else {
            info.name.clone()
        };
        let mut app = App::new(pkg.clone(), name, backend);
        if let Some(v) = versions.get(pkg) {
            app = app.with_version(v.clone());
        }
        if let Some(icon) = &info.icon {
            app = app.with_icon(icon.clone());
        }
        apps.push(app);
    }
    apps
}

pub struct AptBackend;

impl Backend for AptBackend {
    fn id(&self) -> &'static str {
        "apt"
    }

    fn detect(&self) -> bool {
        binary_available("apt-get") && binary_available("dpkg-query")
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let out = run_capture("dpkg-query", &["-W", "-f=${binary:Package}\t${Version}\n"])?;
        let versions = versions_from_dpkg(&out);

        let entries = visible_desktop_entries();
        let files: Vec<&str> = entries.iter().map(|(p, _)| p.as_str()).collect();
        let owners = dpkg_owners(&files);

        Ok(build_apps(&versions, "apt", &entries, &owners))
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        run_privileged(&["apt-get", "remove", "-y", app.id.as_str()])
    }
}

pub struct DnfBackend;

impl Backend for DnfBackend {
    fn id(&self) -> &'static str {
        "dnf"
    }

    fn detect(&self) -> bool {
        binary_available("dnf") && binary_available("rpm")
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let out = run_capture("rpm", &["-qa", "--qf", "%{NAME}\t%{VERSION}-%{RELEASE}\n"])?;
        let versions = versions_from_rpm(&out);

        let entries = visible_desktop_entries();
        let owners: HashMap<String, String> = entries
            .iter()
            .filter_map(|(p, _)| rpm_owner(p).map(|o| (p.clone(), o)))
            .collect();

        Ok(build_apps(&versions, "dnf", &entries, &owners))
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        run_privileged(&["dnf", "remove", "-y", app.id.as_str()])
    }
}

pub struct ZypperBackend;

impl Backend for ZypperBackend {
    fn id(&self) -> &'static str {
        "zypper"
    }

    fn detect(&self) -> bool {
        binary_available("zypper") && binary_available("rpm")
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let out = run_capture("rpm", &["-qa", "--qf", "%{NAME}\t%{VERSION}-%{RELEASE}\n"])?;
        let versions = versions_from_rpm(&out);

        let entries = visible_desktop_entries();
        let owners: HashMap<String, String> = entries
            .iter()
            .filter_map(|(p, _)| rpm_owner(p).map(|o| (p.clone(), o)))
            .collect();

        Ok(build_apps(&versions, "zypper", &entries, &owners))
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        run_privileged(&["zypper", "remove", "-y", app.id.as_str()])
    }
}

pub struct PacmanBackend;

impl Backend for PacmanBackend {
    fn id(&self) -> &'static str {
        "pacman"
    }

    fn detect(&self) -> bool {
        binary_available("pacman")
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let out = run_capture("pacman", &["-Q"])?;
        let versions = versions_from_pacman(&out);

        let entries = visible_desktop_entries();
        let owners: HashMap<String, String> = entries
            .iter()
            .filter_map(|(p, _)| pacman_owner(p).map(|o| (p.clone(), o)))
            .collect();

        Ok(build_apps(&versions, "pacman", &entries, &owners))
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        run_privileged(&["pacman", "-Rns", "--noconfirm", app.id.as_str()])
    }
}

fn versions_from_dpkg(out: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in out.lines() {
        if let Some((name, version)) = line.split_once('\t') {
            if !name.is_empty() {
                map.insert(name.to_string(), version.to_string());
            }
        }
    }
    map
}

fn versions_from_rpm(out: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in out.lines() {
        if let Some((name, version)) = line.split_once('\t') {
            if !name.is_empty() {
                map.insert(name.to_string(), version.to_string());
            }
        }
    }
    map
}

fn versions_from_pacman(out: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in out.lines() {
        if let Some((name, version)) = line.split_once(' ') {
            if !name.is_empty() {
                map.insert(name.to_string(), version.to_string());
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dpkg_versions() {
        let out = "3cpio\t0.14.0-1ubuntu1\nadduser\t3.153ubuntu1\n";
        let versions = versions_from_dpkg(out);
        assert_eq!(versions["3cpio"], "0.14.0-1ubuntu1");
        assert_eq!(versions["adduser"], "3.153ubuntu1");
    }

    #[test]
    fn parses_rpm_versions() {
        let out = "firefox\t126.0-1.fc40\nbash\t5.2.26-3.fc40\n";
        let versions = versions_from_rpm(out);
        assert_eq!(versions["firefox"], "126.0-1.fc40");
        assert_eq!(versions["bash"], "5.2.26-3.fc40");
    }

    #[test]
    fn parses_pacman_versions() {
        let out = "firefox 126.0-1\nbash 5.2.026-1\n";
        let versions = versions_from_pacman(out);
        assert_eq!(versions["firefox"], "126.0-1");
        assert_eq!(versions["bash"], "5.2.026-1");
    }

    #[test]
    fn parses_dpkg_owners() {
        let out = "nautilus: /usr/share/applications/org.gnome.Nautilus.desktop\n\
                   libfoo:amd64: /usr/share/applications/foo.desktop\n";
        let owners = parse_dpkg_owners(out);
        assert_eq!(
            owners["/usr/share/applications/org.gnome.Nautilus.desktop"],
            "nautilus"
        );
        assert_eq!(owners["/usr/share/applications/foo.desktop"], "libfoo");
    }

    #[test]
    fn flags_critical_packages() {
        assert!(is_critical("systemd"));
        assert!(is_critical("linux-image-6.8.0-31-generic"));
        assert!(is_critical("xwayland"));
        assert!(!is_critical("firefox"));
        assert!(!is_critical("nautilus"));
    }

    #[test]
    fn builds_apps_with_real_names_and_icons() {
        let versions = HashMap::from([
            ("nautilus".to_string(), "1:47.0".to_string()),
            ("firefox".to_string(), "1:140.0".to_string()),
        ]);
        let entries = vec![
            (
                "/usr/share/applications/org.gnome.Nautilus.desktop".to_string(),
                DesktopEntry {
                    name: "Files".to_string(),
                    icon: Some("org.gnome.Nautilus".to_string()),
                },
            ),
            (
                "/usr/share/applications/firefox.desktop".to_string(),
                DesktopEntry {
                    name: "Firefox".to_string(),
                    icon: Some("firefox".to_string()),
                },
            ),
        ];
        let owners = HashMap::from([
            (
                "/usr/share/applications/org.gnome.Nautilus.desktop".to_string(),
                "nautilus".to_string(),
            ),
            (
                "/usr/share/applications/firefox.desktop".to_string(),
                "firefox".to_string(),
            ),
        ]);

        let apps = build_apps(&versions, "apt", &entries, &owners);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].id, "nautilus");
        assert_eq!(apps[0].name, "Files");
        assert_eq!(apps[0].icon.as_deref(), Some("org.gnome.Nautilus"));
        assert_eq!(apps[1].id, "firefox");
        assert_eq!(apps[1].name, "Firefox");
        assert_eq!(apps[1].version.as_deref(), Some("1:140.0"));
    }
}
