use std::process::Command;

use crate::backend::{binary_available, process_error, Backend, BackendError};
use crate::model::App;

fn pip_bin() -> Option<&'static str> {
    ["pip3", "pip"]
        .into_iter()
        .find(|bin| binary_available(bin))
}

pub struct PipBackend;

impl Backend for PipBackend {
    fn id(&self) -> &'static str {
        "pip"
    }

    fn detect(&self) -> bool {
        pip_bin().is_some()
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let bin = pip_bin().ok_or_else(|| BackendError("pip is not installed".to_string()))?;
        let output = Command::new(bin)
            .args(["list", "--user", "--format=freeze"])
            .output()
            .map_err(|e| BackendError(format!("failed to run {bin}: {e}")))?;

        if !output.status.success() {
            return Err(BackendError(format!(
                "{bin} list failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        Ok(parse_pip_freeze(&String::from_utf8_lossy(&output.stdout)))
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        let bin = pip_bin().ok_or_else(|| BackendError("pip is not installed".to_string()))?;
        let output = Command::new(bin)
            .args(["uninstall", "-y", &app.id])
            .output()
            .map_err(|e| BackendError(format!("failed to run {bin}: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(process_error(
                &format!("{bin} uninstall {}", app.id),
                &output,
            ))
        }
    }
}

fn parse_pip_freeze(stdout: &str) -> Vec<App> {
    let mut apps = Vec::new();
    for line in stdout.lines() {
        let Some((name, version)) = line.split_once("==") else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        apps.push(App::new(name, name, "pip").with_version(version.trim()));
    }
    apps
}

pub struct NpmBackend;

impl Backend for NpmBackend {
    fn id(&self) -> &'static str {
        "npm"
    }

    fn detect(&self) -> bool {
        binary_available("npm")
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let output = Command::new("npm")
            .args(["ls", "-g", "--depth=0", "--parseable"])
            .output()
            .map_err(|e| BackendError(format!("failed to run npm: {e}")))?;

        if !output.status.success() {
            return Err(BackendError(format!(
                "npm ls failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        Ok(parse_npm_ls(&String::from_utf8_lossy(&output.stdout)))
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        let output = Command::new("npm")
            .args(["uninstall", "-g", &app.id])
            .output()
            .map_err(|e| BackendError(format!("failed to run npm: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(process_error(&format!("npm uninstall {}", app.id), &output))
        }
    }
}

fn parse_npm_ls(stdout: &str) -> Vec<App> {
    let mut apps = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        let Some(idx) = line.rfind("node_modules/") else {
            continue;
        };
        let name = &line[idx + "node_modules/".len()..];
        if name.is_empty() {
            continue;
        }
        apps.push(App::new(name, name, "npm"));
    }
    apps
}

pub struct CargoBackend;

impl Backend for CargoBackend {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn detect(&self) -> bool {
        binary_available("cargo")
    }

    fn list(&self) -> Result<Vec<App>, BackendError> {
        let stdout = cargo_list()?;
        Ok(parse_cargo_list(&stdout))
    }

    fn remove(&self, app: &App) -> Result<(), BackendError> {
        // Use cargo's own uninstall so the crate is removed from cargo's
        // registry, not just its binaries, so it stops showing up on re-list.
        let output = Command::new("cargo")
            .args(["uninstall", &app.id])
            .output()
            .map_err(|e| BackendError(format!("failed to run cargo: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(process_error(
                &format!("cargo uninstall {}", app.id),
                &output,
            ))
        }
    }
}

fn cargo_list() -> Result<String, BackendError> {
    let output = Command::new("cargo")
        .args(["install", "--list"])
        .output()
        .map_err(|e| BackendError(format!("failed to run cargo: {e}")))?;

    if !output.status.success() {
        return Err(BackendError(format!(
            "cargo install --list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_cargo_list(stdout: &str) -> Vec<App> {
    let mut apps = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_suffix(':') else {
            continue;
        };
        let Some((name, version)) = rest.split_once(' ') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        apps.push(
            App::new(name, name, "cargo").with_version(version.trim().trim_start_matches('v')),
        );
    }
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pip_freeze() {
        let out = "foo==1.2.3\nbar==0.1\n-e vcs+file:///tmp/repo@abc#egg=baz\n";
        let apps = parse_pip_freeze(out);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].id, "foo");
        assert_eq!(apps[0].version.as_deref(), Some("1.2.3"));
        assert_eq!(apps[1].id, "bar");
    }

    #[test]
    fn parses_npm_ls() {
        let out = "/usr/lib\n/usr/lib/node_modules/typescript\n/usr/lib/node_modules/@scope/pkg\n";
        let apps = parse_npm_ls(out);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].id, "typescript");
        assert_eq!(apps[1].id, "@scope/pkg");
    }

    #[test]
    fn parses_cargo_list() {
        let out = "bat v0.24.0:\n    /home/user/.cargo/bin/bat\nexa v0.10.1:\n    /home/user/.cargo/bin/exa\n";
        let apps = parse_cargo_list(out);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].id, "bat");
        assert_eq!(apps[0].version.as_deref(), Some("0.24.0"));
        assert_eq!(apps[1].id, "exa");
    }
}
