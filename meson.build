project(
  'removestaller',
  version: '1.0.2',
  meson_version: '>= 1.0.0',
)

app_id = 'io.github.chloevpin.Removestaller'
binary_name = 'removestaller'
cargo_profile = get_option('buildtype') == 'release' ? 'release' : 'debug'

gnome = import('gnome')

dependency('gtk4', version: '>= 4.10')
dependency('libadwaita-1', version: '>= 1.5')

# Build the Rust binary with cargo and expose it as a meson target.
cargo_bin = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: binary_name,
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    find_program('sh'),
    meson.project_source_root() / 'build-aux' / 'cargo-build.sh',
    meson.project_source_root(),
    meson.project_build_root() / 'target',
    binary_name,
    '@OUTPUT@',
    cargo_profile,
  ],
)

install_data(
  join_paths('data', app_id + '.desktop'),
  install_dir: get_option('datadir') / 'applications',
)

install_data(
  join_paths('data', app_id + '.metainfo.xml'),
  install_dir: get_option('datadir') / 'metainfo',
)

install_data(
  join_paths('data', 'icons', 'hicolor', 'scalable', 'apps', app_id + '.svg'),
  install_dir: get_option('datadir') / 'icons' / 'hicolor' / 'scalable' / 'apps',
)

gnome.post_install(
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
