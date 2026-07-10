<div align="center">
  <img src="public/app-icon.png" width="96" height="96" alt="Windows Apps logo">

# Windows Apps

**A fast, private application catalog and launcher for Windows 10 and Windows 11.**

[![Version](https://img.shields.io/badge/version-0.2.1-7C3AED?style=flat-square)](https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.1)
![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?style=flat-square&logo=windows11&logoColor=white)
![Architecture](https://img.shields.io/badge/architecture-x64-334155?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![Local first](https://img.shields.io/badge/catalog-local--first-16A34A?style=flat-square)

[Download Windows Apps 0.2.1](https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.1) ·
[Documentation](Documentation.md) ·
[Telegram](https://t.me/keskiyo)

</div>

---

## Overview

Windows Apps collects applications scattered across Start Menu shortcuts, Windows registrations, Store packages, Steam libraries, and fixed local drives into one searchable catalog.

The catalog is stored locally. On startup, cached names appear immediately while icons and metadata are loaded in the background. Incremental scanning checks changed locations instead of repeatedly scanning every file.

## Features

- **Unified catalog** — Start Menu shortcuts, registered desktop software, packaged Windows apps, Steam games, and portable executables.
- **Fast startup** — lightweight versioned cache is rendered before background synchronization finishes.
- **Incremental scanning** — unchanged directories reuse previous results.
- **Controlled full scans** — progress reporting, cancellation, depth, entry-count, and time limits.
- **Smart deduplication** — evidence-based resolution merges the same product found across sources, with useful shortcuts preferred and legitimate apps kept when identity is ambiguous.
- **Noise filtering** — installers, uninstall helpers, updaters, documentation shortcuts, resource entries, and broken names are filtered.
- **Full-text search** — matches application name, publisher, description, and install path; each word is matched independently.
- **Quick launch (Ctrl+K)** — keyboard-first command palette to find and launch any app; `Ctrl+F` or `/` jumps to search.
- **Launch feedback** — the card shows a launching state (dimmed icon + spinner) and a top activity bar, cleared when the app window is ready or after a short ceiling.
- **Background icon loading** — visible application cards receive priority without creating duplicate hydration work.
- **Organization** — automatic and custom categories, category reordering, application moves, Favorites (surfaced first within a category), and reversible Hidden items.
- **Automatic updates** — the app checks GitHub Releases on startup and offers a signed update with one click; you choose when to install.
- **Responsive navigation** — persistent sidebar from `1024px`; overlay drawer on smaller windows.
- **Keyboard & accessibility** — focus traps in dialogs and menus, `aria-current` navigation, arrow-key menus, and reduced-motion support.
- **Native launching** — shortcuts, executables, shell targets, Steam entries, and packaged applications use their appropriate Windows launch mechanism.
- **Registered uninstall** — Windows Apps uses vendor, MSI, or MSIX uninstall information registered with Windows.
- **Uninstall history** — keeps a local privacy-limited history of the latest 100 attempts.
- **System tray** — closing the window keeps the launcher available in the notification area.
- **Global shortcut** — `Win+Shift+Q` restores the window using the physical Q key regardless of keyboard layout.
- **Windows startup** — optional launch after sign-in for the current Windows account.
- **Catalog maintenance** — Force full scan and Reset catalog cache are available in Settings.

## Installation

1. Open [Windows Apps 0.2.1](https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.1).
2. Download `Windows.Apps_0.2.1_x64-setup.exe`.
3. Run the installer.
4. Start **Windows Apps** and select **Scan for apps** when prompted.

The installer is not Authenticode-signed, so Microsoft Defender SmartScreen may show an unrecognized-app warning. Download builds only from this repository's official Releases. Automatic updates are cryptographically signed and verified by the app before installation.

### System requirements

| Requirement      | Value                                                |
| ---------------- | ---------------------------------------------------- |
| Operating system | Windows 10 or Windows 11                             |
| Architecture     | x64                                                  |
| Runtime          | Microsoft Edge WebView2                              |
| Internet         | Not required after installation for catalog features |

The NSIS installer can download the WebView2 bootstrapper when the runtime is missing.

## How scanning works

Windows Apps scans permanent local drives reported by Windows as fixed drives. Removable USB, optical, and network drives are excluded from automatic scanning.

Each fixed-drive scan is limited to:

- 16 directory levels;
- 500,000 filesystem entries;
- three minutes per scan root.

Symbolic links, junctions, and other reparse-point directories are skipped. These limits prevent loops and excessive disk activity while retaining already discovered applications.

Use:

- **Refresh** for a normal incremental update;
- **Force full scan** to rebuild the filesystem index;
- **Reset catalog cache** to remove generated catalog/icon caches and perform a clean scan.

Favorites, Hidden items, custom categories, and category assignments are preserved when the catalog cache is reset.

## Privacy and safety

- The application catalog is processed and stored locally.
- No software inventory or drive list is uploaded.
- No telemetry service is configured.
- Missing descriptions are left unknown; Windows Apps does not invent or download metadata.
- Launch commands are resolved from Rust-owned catalog IDs, not arbitrary frontend paths.
- Uninstall commands come from Windows registration data and are previewed before execution.
- UNC/network uninstall executables are rejected.
- Program folders are never recursively deleted as an uninstall method.
- Uninstall history excludes commands, paths, arguments, package IDs, usernames, and detailed error text.

## Development

Prerequisites:

- Node.js 22 and npm;
- stable Rust with the MSVC toolchain;
- Microsoft C++ Build Tools and Windows SDK;
- WebView2 Runtime;
- [Tauri prerequisites for Windows](https://v2.tauri.app/start/prerequisites/).

Install and run:

```powershell
npm install
npm run tauri dev
```

Verification:

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

Production build:

```powershell
npm run tauri build
```

Primary local artifact:

```text
src-tauri/target/release/bundle/nsis/Windows Apps_0.2.1_x64-setup.exe
```

## Release process

Pushing a `v*` tag runs `.github/workflows/release.yml`. The workflow:

1. validates that the tag matches all version manifests;
2. installs npm dependencies;
3. runs frontend and Rust tests;
4. uses `tauri-apps/tauri-action` to build the Windows x64 installer, sign the update
   artifacts with the `TAURI_SIGNING_PRIVATE_KEY` secret, and generate `latest.json`;
5. creates the GitHub Release for the tag and uploads the installer, `latest.json`, and
   signature so existing installs can update automatically.

See [Documentation.md](Documentation.md) for architecture, native commands, scanning behavior, troubleshooting, and the release checklist.

## Support

- [Technical documentation](Documentation.md)
- [GitHub Releases](https://github.com/keskiyo/WindowsApps/releases)
- [Telegram: @keskiyo](https://t.me/keskiyo)

<div align="center">
  <sub>Built with Tauri, Rust, React, TypeScript, Vite, and native Windows APIs.</sub>
</div>
