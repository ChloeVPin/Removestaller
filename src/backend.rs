use std::error::Error;
use std::fmt;

use crate::model::App;

/// Whether a command line tool is present on PATH and runnable.
pub fn binary_available(bin: &str) -> bool {
    std::process::Command::new(bin)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Failure from a backend, carrying a human readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError(pub String);

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for BackendError {}

/// Turn a failed child process into an error that keeps useful diagnostics.
pub fn process_error(action: &str, output: &std::process::Output) -> BackendError {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        BackendError(format!("{action} failed"))
    } else {
        BackendError(format!("{action} failed: {detail}"))
    }
}

/// A backend that was available but could not be queried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendFailure {
    pub backend: &'static str,
    pub error: BackendError,
}

/// Results from scanning every available backend.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListReport {
    pub apps: Vec<App>,
    pub failures: Vec<BackendFailure>,
}

/// A source of installed applications for one package format.
pub trait Backend {
    /// Stable identifier for this backend, stored on every App it produces.
    fn id(&self) -> &'static str;

    /// Whether this backend's underlying tooling is present on the system.
    fn detect(&self) -> bool;

    /// List installed applications of this format.
    fn list(&self) -> Result<Vec<App>, BackendError>;

    /// Remove one installed application.
    fn remove(&self, app: &App) -> Result<(), BackendError>;
}

/// Holds every backend and queries them as a group.
pub struct Registry {
    backends: Vec<Box<dyn Backend + Send + Sync>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    pub fn add(&mut self, backend: Box<dyn Backend + Send + Sync>) {
        self.backends.push(backend);
    }

    /// Backends whose tooling is present on this system.
    pub fn available(&self) -> Vec<&(dyn Backend + Send + Sync)> {
        let mut out = Vec::new();
        for backend in &self.backends {
            if backend.detect() {
                out.push(backend.as_ref());
            }
        }
        out
    }

    /// All installed applications across every available backend.
    pub fn list_all(&self) -> ListReport {
        let mut apps = Vec::new();
        let mut failures = Vec::new();
        for backend in self.available() {
            match backend.list() {
                Ok(mut found) => apps.append(&mut found),
                // One broken backend must not hide results from the rest.
                Err(error) => failures.push(BackendFailure {
                    backend: backend.id(),
                    error,
                }),
            }
        }
        ListReport { apps, failures }
    }

    /// Remove an app by routing to the backend that owns it.
    pub fn remove(&self, app: &App) -> Result<(), BackendError> {
        for backend in &self.backends {
            if backend.id() == app.backend {
                return backend.remove(app);
            }
        }
        Err(BackendError(format!("no backend for '{}'", app.backend)))
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::App;

    struct FakeBackend {
        present: bool,
        apps: Vec<App>,
    }

    impl Backend for FakeBackend {
        fn id(&self) -> &'static str {
            "fake"
        }

        fn detect(&self) -> bool {
            self.present
        }

        fn list(&self) -> Result<Vec<App>, BackendError> {
            Ok(self.apps.clone())
        }

        fn remove(&self, _app: &App) -> Result<(), BackendError> {
            Ok(())
        }
    }

    #[test]
    fn registry_filters_unavailable_backends() {
        let mut registry = Registry::new();
        registry.add(Box::new(FakeBackend {
            present: false,
            apps: vec![App::new("hidden", "Hidden", "fake")],
        }));
        registry.add(Box::new(FakeBackend {
            present: true,
            apps: vec![App::new("shown", "Shown", "fake")],
        }));

        assert_eq!(registry.available().len(), 1);

        let report = registry.list_all();
        assert_eq!(report.apps.len(), 1);
        assert_eq!(report.apps[0].id, "shown");
        assert!(report.failures.is_empty());
    }

    #[test]
    fn remove_routes_to_owning_backend() {
        let mut registry = Registry::new();
        registry.add(Box::new(FakeBackend {
            present: true,
            apps: Vec::new(),
        }));

        let app = App::new("x", "X", "fake");
        assert!(registry.remove(&app).is_ok());

        let unknown = App::new("y", "Y", "missing");
        assert!(registry.remove(&unknown).is_err());
    }

    #[test]
    fn list_report_preserves_backend_failures() {
        struct FailingBackend;

        impl Backend for FailingBackend {
            fn id(&self) -> &'static str {
                "broken"
            }

            fn detect(&self) -> bool {
                true
            }

            fn list(&self) -> Result<Vec<App>, BackendError> {
                Err(BackendError("test failure".to_string()))
            }

            fn remove(&self, _app: &App) -> Result<(), BackendError> {
                Ok(())
            }
        }

        let mut registry = Registry::new();
        registry.add(Box::new(FailingBackend));

        let report = registry.list_all();
        assert_eq!(
            report.failures,
            vec![BackendFailure {
                backend: "broken",
                error: BackendError("test failure".to_string()),
            }]
        );
    }
}
