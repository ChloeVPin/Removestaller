# Removestaller project brief

## Product

Removestaller is a native Linux desktop application for finding and removing installed applications from one place. It brings packages from the system package manager and common developer or desktop runtimes into one focused interface.

It supports available APT, DNF, Zypper, Pacman, Flatpak, Snap, AppImage, Python, Node.js, and Rust sources. Each application shows its name, version, package identifier, and source before removal. Removal requires an explicit confirmation step.

The product is not a software store and does not recommend, install, rank, or advertise applications. Its purpose is simpler: show what is installed and make removal understandable and deliberate.

## Positioning

**One calm place to remove what no longer belongs on your Linux system.**

Removestaller should feel precise, quiet, and capable. It is for people who want a clearer view of the software already on their computer, including applications installed from different sources.

## Facts

- Name: Removestaller
- Developer: chloevpin
- Current version: 1.0.2
- Platform: Linux
- Built with: Rust, GTK4, libadwaita
- License: GPL-3.0-or-later
- Source and releases: https://github.com/ChloeVPin/Removestaller
- Packages: Debian `.deb` and Fedora 44 `.rpm`

## Core capabilities

- Collect installed applications from supported package sources.
- Group applications by their source.
- Search by application name or package identifier.
- Show package details before removal.
- Ask for confirmation before removing anything.
- Clearly show when a source could not be checked.

## Website copy starter

### Hero

**Remove software with a clearer view.**

Removestaller brings applications from the package systems already on your Linux computer into one focused list. Find what you no longer need, review its details, and remove it with confidence.

Primary action: `Download for Linux`

Secondary action: `View source`

### How it works

**See the full picture**

Browse installed applications grouped by the source that manages them.

**Check before you change**

Open an application to review its version, package identifier, and source.

**Remove deliberately**

Every removal is confirmed before it runs.

### Supported sources

APT, DNF, Zypper, Pacman, Flatpak, Snap, AppImage, Python, Node.js, and Rust, when available on the system.

### Trust note

Removestaller works with package tools already present on the computer. It does not host software, add a marketplace, or collect accounts.

## Visual direction

The website and product identity should be minimal but not generic. Avoid the default startup look: oversized gradient blobs, floating glass cards, stock 3D illustrations, dense feature grids, neon effects, generic dashboard charts, and decorative icon rows.

Use a restrained editorial layout with generous negative space, one strong blue drawn from the logo, warm off-white surfaces, charcoal text, and a small number of carefully considered rules or dividers. Let the product screenshots do the visual work.

Typography should be distinctive and highly legible. Do not use Inter, Poppins, Montserrat, or a generic geometric sans just because they are familiar. Select a typeface with a little character, excellent small-size rendering, and a proper license for web use. Use one type family with a limited weight range. The interface itself should continue using the system font supplied by GTK and libadwaita.

Do not introduce a generic icon library as decoration. Use the existing Removestaller logo where an identity mark is needed. If an icon is necessary for navigation or a control, use a small, consistent custom line treatment or the operating system’s native symbolic icon language.

Keep motion subtle and functional. Use quick fades or position shifts only to explain state changes. Do not use scroll-jacking, looping hero animations, or parallax.

## Screenshot source paths

The original user-supplied screenshots are in:

`/home/user/Pictures/Screenshots`

Use these files for website work:

| Original file | Purpose | Repository copy |
| --- | --- | --- |
| `/home/user/Pictures/Screenshots/1.png` | Installed application list | `data/screenshots/overview.png` |
| `/home/user/Pictures/Screenshots/3.png` | Application details | `data/screenshots/application-details.png` |
| `/home/user/Pictures/Screenshots/4.png` | Removal confirmation | `data/screenshots/removal-confirmation.png` |
| `/home/user/Pictures/Screenshots/5.png` | About dialog | `data/screenshots/about.png` |

Do not use `2.png` as a primary website screenshot. It is a partial continuation of the list view. All selected screenshots are 910 by 750 PNG files and are already included in the AppStream metadata.
