/// A single installed application, owned by one backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    /// Stable identifier used for removal (package name, Flatpak ref, path).
    pub id: String,
    /// Human readable display name.
    pub name: String,
    /// Version string, when the backend reports one.
    pub version: Option<String>,
    /// Identifier of the backend that owns this app.
    pub backend: &'static str,
    /// Icon theme name or absolute path to the app's icon, when known.
    pub icon: Option<String>,
}

impl App {
    pub fn new(id: impl Into<String>, name: impl Into<String>, backend: &'static str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: None,
            backend,
            icon: None,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}
