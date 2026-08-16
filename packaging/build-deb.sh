#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    printf 'usage: %s VERSION STAGE_DIR OUTPUT_DIR\n' "$0" >&2
    exit 2
fi

version=$1
stage_dir=$2
output_dir=$3
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/.." && pwd)

if [[ ! "$version" =~ ^[0-9][0-9A-Za-z.+~-]*$ ]]; then
    printf 'invalid Debian package version: %s\n' "$version" >&2
    exit 2
fi

if [[ ! -x "$stage_dir/usr/bin/removestaller" ]]; then
    printf 'staged Removestaller binary not found: %s\n' "$stage_dir/usr/bin/removestaller" >&2
    exit 1
fi

architecture=${DEB_HOST_ARCH:-$(dpkg-architecture -qDEB_HOST_ARCH)}
package_root=$(mktemp -d "${TMPDIR:-/tmp}/removestaller-deb.XXXXXX")
trap 'rm -rf "$package_root"' EXIT

mkdir -p "$output_dir"
cp -a "$stage_dir"/. "$package_root"/
install -d "$package_root/DEBIAN" "$package_root/usr/share/doc/removestaller"
install -m 0644 "$repo_root/packaging/copyright" "$package_root/usr/share/doc/removestaller/copyright"

cat > "$package_root/DEBIAN/control" <<EOF
Package: removestaller
Version: $version
Section: utils
Priority: optional
Architecture: $architecture
Maintainer: chloevpin <227690662+ChloeVPin@users.noreply.github.com>
Depends: libgtk-4-1 (>= 4.10), libadwaita-1-0 (>= 1.5), pkexec
Homepage: https://github.com/ChloeVPin/Removestaller
Description: Remove installed applications
 Removestaller removes installed applications from supported package formats
 through one GTK4 and libadwaita interface.
EOF

output="$output_dir/removestaller_${version}_${architecture}.deb"
dpkg-deb --build --root-owner-group "$package_root" "$output" >/dev/null
printf '%s\n' "$output"
