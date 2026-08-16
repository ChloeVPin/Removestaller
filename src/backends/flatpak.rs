use std::process::Command;

use crate::backend::{binary_available, process_error, Backend, BackendError};
use crate::model::App;

pub struct FlatpakBackend;

impl Backend for FlatpakBackend {
    fn id(&self) -> &'static str {
        "flatpak"
    }

    fn detect(&self) -> bool {
        binary_available("flatpak")
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let output = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,name,version"])
            .output()
            .map_err(|e| BackendError(format!("failed to run flatpak: {e}")))?;

        if !output.status.success() {
            return Err(BackendError(format!(
                "flatpak list failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut apps = Vec::new();

        for line in stdout.lines().skip(1) {
            let cols: Vec<&str> = line.split('\t').map(str::trim).collect();
            if cols.len() < 2 || cols[0].is_empty() {
                continue;
            }
            let mut app = App::new(cols[0], cols[1], self.id());
            if cols.len() >= 3 && !cols[2].is_empty() {
                app = app.with_version(cols[2]);
            }
            apps.push(app);
        }

        Ok(apps)
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        let output = Command::new("flatpak")
            .args(["uninstall", "-y", &app.id])
            .output()
            .map_err(|e| BackendError(format!("failed to run flatpak: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(process_error(
                &format!("flatpak uninstall {}", app.id),
                &output,
            ))
        }
    }
}
