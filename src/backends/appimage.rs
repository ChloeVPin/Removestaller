use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::backend::{Backend, BackendError};
use crate::model::App;

/// AppImages have no dedicated tooling: this backend works directly on the
/// filesystem, so it is always available.
pub struct AppImageBackend;

impl Backend for AppImageBackend {
    fn id(&self) -> &'static str {
        "appimage"
    }

    fn detect(&self) -> bool {
        true
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        Ok(scan_appimages())
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        fs::remove_file(&app.id)
            .map_err(|e| BackendError(format!("failed to delete {}: {e}", app.id)))
    }
}

fn scan_appimages() -> Vec<App> {
    let mut apps = Vec::new();

    let Some(home) = env::var_os("HOME") else {
        return apps;
    };
    let home = PathBuf::from(home);

    for dir in ["Applications", ".local/bin", "bin", "Desktop", "Downloads"] {
        let full = home.join(dir);
        let Ok(entries) = fs::read_dir(&full) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_appimage(&path) {
                if let Some(app) = app_from_path(&path) {
                    apps.push(app);
                }
            }
        }
    }

    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

fn is_appimage(path: &Path) -> bool {
    path.is_file() && has_appimage_extension(path)
}

fn has_appimage_extension(path: &Path) -> bool {
    path.extension()
        .map(|e| e.eq_ignore_ascii_case("AppImage"))
        .unwrap_or(false)
}

fn app_from_path(path: &Path) -> Option<App> {
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let id = path.to_string_lossy().to_string();
    Some(App::new(id, stem, "appimage"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_appimage_extension() {
        assert!(has_appimage_extension(Path::new("Foo.AppImage")));
        assert!(has_appimage_extension(Path::new("Foo.appimage")));
        assert!(!has_appimage_extension(Path::new("Foo.png")));
    }

    #[test]
    fn builds_app_from_path() {
        let app = app_from_path(Path::new("/home/user/Applications/MyTool.AppImage")).unwrap();
        assert_eq!(app.id, "/home/user/Applications/MyTool.AppImage");
        assert_eq!(app.name, "MyTool");
        assert_eq!(app.backend, "appimage");
    }
}
