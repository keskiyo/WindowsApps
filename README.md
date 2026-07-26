<div align="center">
  <img src="public/app-icon.png" width="96" height="96" alt="Windows Apps logo">

# Windows Apps

**A fast, private application catalog and launcher for Windows 10 and Windows 11.**

[![Version](https://img.shields.io/badge/version-0.2.7-7C3AED?style=flat-square)](https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.7)
![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?style=flat-square&logo=windows11&logoColor=white)
![Architecture](https://img.shields.io/badge/architecture-x64-334155?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![Local first](https://img.shields.io/badge/catalog-local--first-16A34A?style=flat-square)

[Download Windows Apps 0.2.7](https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.7) ·
[Documentation](Documentation.md) ·
[Telegram](https://t.me/keskiyo)

</div>

---

## Overview

Windows Apps collects applications scattered across Start Menu shortcuts, Windows registrations, Store packages, Steam libraries, and fixed local drives into one searchable catalog.

The catalog is stored locally. On startup, cached names appear immediately while icons and metadata are loaded in the background. Incremental scanning checks changed locations instead of repeatedly scanning every file.

## Features

Most launchers just re-list what Windows already knows. Windows Apps earns its place by what it does _with_ that list — deciding what deserves to be a launch card, keeping your choices attached to the app rather than to a fragile internal ID, and resolving every native action inside Rust.

### An intelligent catalog

- **Evidence-based deduplication** — the same product found as a Start Menu shortcut, a registry entry, a Store package, and a Steam game collapses into one card. Merging weighs resolved targets, product families, publishers, and install roots; a useful shortcut wins, and a genuine app is kept when the evidence is ambiguous rather than guessed away.
- **Explainable classification, not filename guessing** — every entry is scored `primary` / `auxiliary` / `rejected` from Windows registration, shortcut target, PE metadata, location, and role, each with a stable reason code. `ServiceDesk` is not mistaken for a service; a renamed `notification_helper.exe` is not mistaken for an app.
- **Auxiliary tools instead of deletion** — runtime components, language servers, and maintenance executables stay out of your categories and search but remain inspectable, and any of them can be restored in one click.
- **Choices that survive a rescan** — Favorites, Hidden items, and restored tools are tracked by a normalized application identity, so they follow the app across a source change, a full rescan, or a cache reset instead of vanishing when its internal ID changes.

### Built to feel instant

- **Cache-first startup** — names render from a lightweight versioned cache before any scan runs; icons and metadata stream in behind them.
- **Incremental by default** — unchanged directories reuse prior results; only what actually changed on disk is re-read.
- **Bounded, cancellable full scans** — depth, entry-count, and time limits with live progress. Reparse points, network, and removable drives are skipped, so a scan can neither loop nor run away.
- **Batched icon hydration** — visible cards are prioritized and icons arrive in batches, so streaming metadata never re-renders the whole grid.

### Secure by construction

- **The webview can only pass an ID** — every launch and uninstall is resolved from a Rust-owned catalog map, never from a path supplied by the frontend.
- **No shell strings** — processes are built from a fixed executable plus an argument vector; UNC/network targets, script interpreters, and unvalidated uninstall commands are refused.
- **Local and quiet** — the catalog is processed and stored on your machine; no inventory, drive list, or telemetry leaves it, and missing metadata is left unknown rather than invented.
- **Privacy-limited uninstall history** — the newest 100 attempts, storing only name, publisher, mechanism, and result — never a command, path, argument, or error text.

### Finding and launching

- **Full-text search** — matches name, publisher, description, and install path, each word independently, cached so typing stays smooth over a large catalog.
- **Quick-launch palette (Ctrl+K)** — keyboard-first find-and-launch; `Ctrl+F` or `/` jumps to search.
- **Honest launch feedback** — the card dims with a spinner and a top activity bar until the app's window is actually ready, cleared early by the backend or by a short ceiling.
- **Native launch mechanisms** — shortcuts, executables, shell targets, packaged apps, and Steam entries each launch the way Windows intends; Steam games start through Steam, so overlay, cloud saves, and playtime keep working.

### Organization and desktop fit

- **Flexible organization** — automatic plus custom categories with reordering and drag-to-move, Favorites surfaced first within a category, and reversible Hidden items.
- **Registered, signed, reversible** — vendor / MSI / MSIX uninstall from Windows' own registration; signed NSIS-only auto-updates with full notes, byte progress, and verification; nothing forced.
- **Lives in the tray** — closing hides to the notification area; `Win+Shift+Q` (physical key, layout-independent) brings it back; optional launch at sign-in.
- **Accessible and responsive** — focus-trapped dialogs, arrow-key menus, `aria-current` navigation, reduced-motion support, and a persistent sidebar that becomes an overlay drawer on narrow windows.
- **Catalog maintenance** — Settings exposes scan diagnostics, force full scan, and cache reset.

## Installation

1. Open [Windows Apps 0.2.7](https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.7).
2. Download `Windows.Apps_0.2.7_x64-setup.exe`.
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
- **Repair missing icons** to retry extraction only for applications without an icon;
- **Clear icon cache** to rebuild icons from the existing catalog without rescanning drives;
- **Reset catalog cache** to remove generated catalog/icon caches and perform a clean scan.

Favorites, Hidden items, promoted auxiliary tools, custom categories, and category assignments are preserved when the catalog cache is reset.

Catalog visibility is conservative. Start Menu/AUMID applications, Steam games, registered products, and portable executables with coherent product metadata receive strong launcher evidence. Runtime paths and helper/service/language-server metadata reduce visibility. Ambiguous entries are placed in **Auxiliary tools** rather than deleted, and **Restore to catalog** creates a persistent user override.

PE `ProductName` and `OriginalFilename` metadata are retained separately. They help identify renamed installers and helpers without replacing the user-facing name. Shortcut arguments affect identity only for a small set of user-facing modes such as browser profiles and PWA app launches; ordinary service flags do not create duplicate cards.

## Privacy and safety

- The application catalog is processed and stored locally.
- No software inventory or drive list is uploaded.
- No telemetry service is configured.
- Missing descriptions are left unknown; Windows Apps does not invent or download metadata.
- Launch commands are resolved from Rust-owned catalog IDs, not arbitrary frontend paths.
- Uninstall targets come from Windows registration data. The preview exposes only application identity and a safe removal-mechanism label; command details remain backend-only.
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
npm run lint
npm run typecheck
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Production build:

```powershell
npm run tauri build
```

Primary local artifact:

```text
src-tauri/target/release/bundle/nsis/Windows Apps_0.2.7_x64-setup.exe
```

## Support

- [Technical documentation](Documentation.md)
- [GitHub Releases](https://github.com/keskiyo/WindowsApps/releases)
- [Telegram: @keskiyo](https://t.me/keskiyo)

<div align="center">
  <sub>Built with Tauri, Rust, React, TypeScript, Vite, and native Windows APIs.</sub>
</div>
