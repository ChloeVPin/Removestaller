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
    printf 'invalid RPM package version: %s\n' "$version" >&2
    exit 2
fi

if [[ ! -x "$stage_dir/usr/bin/removestaller" ]]; then
    printf 'staged Removestaller binary not found: %s\n' "$stage_dir/usr/bin/removestaller" >&2
    exit 1
fi

mkdir -p "$output_dir"
top_dir=$(mktemp -d "${TMPDIR:-/tmp}/removestaller-rpm.XXXXXX")
trap 'rm -rf "$top_dir"' EXIT
mkdir -p "$top_dir"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
install -d "$top_dir/SOURCES/stage"
cp -a "$stage_dir"/. "$top_dir/SOURCES/stage"/
install -d "$top_dir/SOURCES/stage/usr/share/doc/removestaller"
install -m 0644 "$repo_root/packaging/copyright" "$top_dir/SOURCES/stage/usr/share/doc/removestaller/copyright"

cat > "$top_dir/SPECS/removestaller.spec" <<EOF
Name:           removestaller
Version:        $version
Release:        1%{?dist}
Summary:        Remove installed applications
License:        GPL-3.0-or-later
URL:            https://github.com/ChloeVPin/Removestaller

Requires:       gtk4 >= 4.10
Requires:       libadwaita >= 1.5
Requires:       polkit

%description
Removestaller removes installed applications from supported package formats
through one GTK4 and libadwaita interface.

%install
rm -rf %{buildroot}
cp -a %{_topdir}/SOURCES/stage/. %{buildroot}/

%files
%license %{_docdir}/removestaller/copyright
%{_bindir}/removestaller
%{_datadir}/applications/io.github.chloevpin.Removestaller.desktop
%{_datadir}/icons/hicolor/scalable/apps/io.github.chloevpin.Removestaller.svg
%{_datadir}/metainfo/io.github.chloevpin.Removestaller.metainfo.xml

%changelog
* $(date -u '+%a %b %d %Y') chloevpin <227690662+ChloeVPin@users.noreply.github.com> - $version-1
- Initial Removestaller package.
EOF

rpmbuild --define "_topdir $top_dir" --define "_build_id_links none" -bb "$top_dir/SPECS/removestaller.spec" >/dev/null
rpm_path=$(find "$top_dir/RPMS" -type f -name '*.rpm' -print -quit)
if [[ -z "$rpm_path" ]]; then
    printf 'rpmbuild did not produce an RPM\n' >&2
    exit 1
fi

output="$output_dir/$(basename "$rpm_path")"
cp "$rpm_path" "$output"
printf '%s\n' "$output"
