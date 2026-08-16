pub mod appimage;
pub mod desktop;
pub mod distro;
pub mod flatpak;
pub mod snap;
pub mod userspace;

use crate::backend::Registry;

/// Build the registry with every supported backend.
pub fn build_registry() -> Registry {
    let mut registry = Registry::new();
    registry.add(Box::new(distro::AptBackend));
    registry.add(Box::new(distro::DnfBackend));
    registry.add(Box::new(distro::ZypperBackend));
    registry.add(Box::new(distro::PacmanBackend));
    registry.add(Box::new(flatpak::FlatpakBackend));
    registry.add(Box::new(snap::SnapBackend));
    registry.add(Box::new(appimage::AppImageBackend));
    registry.add(Box::new(userspace::PipBackend));
    registry.add(Box::new(userspace::NpmBackend));
    registry.add(Box::new(userspace::CargoBackend));
    registry
}
