<div align="center">
  <img src="public/app-icon.png" width="96" height="96" alt="Windows Apps logo">

# Windows Apps

**Every application on your PC, in one searchable place.**

Start Menu shortcuts, installed programs, Store apps, Steam games and portable executables — collected into a single catalog that opens instantly and stays on your machine.

[![Version](https://img.shields.io/badge/version-0.3.2-7C3AED?style=flat-square)](https://github.com/keskiyo/WindowsApps/releases/tag/v0.3.2)
![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?style=flat-square&logo=windows11&logoColor=white)
![Architecture](https://img.shields.io/badge/architecture-x64-334155?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![Local first](https://img.shields.io/badge/catalog-local--first-16A34A?style=flat-square)

### [⬇ Download for Windows](https://github.com/keskiyo/WindowsApps/releases/latest)

[Documentation](Documentation.md) · [Release notes](https://github.com/keskiyo/WindowsApps/releases/tag/v0.3.2) · [Telegram](https://t.me/keskiyo)

<img src=".github/images/catalog.png" alt="Windows Apps catalog" width="880">

</div>

---

## Contents

[Why](#why) · [Screenshots](#screenshots) · [Install](#install) · [Features](#features) · [Privacy](#privacy) · [Development](#development)

## Why

Windows spreads your software across four places that never agree with each other: the Start Menu, Installed apps, the Microsoft Store, and whatever you unzipped to a folder. Windows Apps reads all of them and produces one list.

The interesting part is what happens next. The same program found as a shortcut, a registry entry, a Store package and a Steam game becomes **one card**. Runtime components, updaters and language servers step aside into **Auxiliary tools** instead of cluttering your categories. And your favorites and custom categories follow the application itself, so a rescan or a version bump does not scatter them.

Everything runs locally. Nothing about your machine is uploaded.

## Screenshots

<div align="center">
  <img src=".github/images/more.png" alt="More: auxiliary tools, scenarios, hidden apps and installers" width="880">
</div>

**More** holds what is kept out of the main list — Auxiliary tools, Scenarios, Hidden apps and
Installers & Docs. Each card carries the count of the view it opens and previews its newest entries.

<div align="center">
  <img src=".github/images/scenarios.png" alt="Scenarios with launch and close lists" width="880">
</div>

**Scenarios** pair a launch list with a close list. One click starts everything in the first and
fully closes everything in the second.

<div align="center">
  <img src=".github/images/app-info.png" alt="App information dialog" width="880">
</div>

**App information** — size, dates, architecture, Authenticode signature and whether the install
location still exists. Copy the path, copy a full report, or open the containing folder.

<div align="center">
  <img src=".github/images/settings.png" alt="Settings and catalog maintenance" width="880">
</div>

**Settings** — scan folders, autostart, the global shortcut, updates, preference export/import
and a one-step local backup, plus catalog maintenance whose force scan and cache reset preserve
Favorites, Hidden apps and categories.

## Install

1. Download **`Windows.Apps_0.3.2_x64-setup.exe`** from the [latest release](https://github.com/keskiyo/WindowsApps/releases/latest).
2. Run it.
3. Start **Windows Apps** and choose **Scan for apps**.

> The installer is not Authenticode-signed, so SmartScreen may show _"Windows protected your PC"_. Choose **More info → Run anyway**. Download only from this repository's Releases. Automatic updates are cryptographically signed and verified by the app before installing.

| Requirement  | Value                      |
| ------------ | -------------------------- |
| OS           | Windows 10 or 11           |
| Architecture | x64                        |
| Runtime      | Microsoft Edge WebView2    |
| Internet     | Not required after install |

## Features

- **One app, one card** — shortcuts, registry entries, Store packages, Steam games and discovered executables are merged into a single application when they point to the same product.

- **Typo and layout tolerant search** — finds apps by name, publisher, description and path even with misspellings or the wrong keyboard layout.

- **Native launching** — Steam games launch through Steam, packaged apps through Windows, shortcuts as shortcuts, and executables directly. No generic launcher path that breaks overlays, cloud saves or playtime tracking.

- **Real launch status** — an app is considered launched when its window is ready, not when a process merely appears.

- **Scenarios** — save a set of apps to start and another set to fully close, then run both as one action. A close list will not take a process Windows cannot survive losing, and asks before it takes one that ends the desktop shell.

- **Non-destructive filtering** — updaters, crash handlers, runtimes, documentation and other secondary entries are moved to **Auxiliary tools** instead of silently disappearing.

- **Stable app identity** — Favorites, Hidden state, categories and scenarios survive executable path changes, version updates, rescans and cache resets. Scenarios can be starred too, and Favorites shows them beside the applications.

- **Conservative detection** — ambiguous applications stay visible instead of being incorrectly merged or removed. Text matches alone cannot permanently discard an entry. A card is removed only when Windows confirms its file is gone; a check that was denied or failed keeps the application and says so.

- **Decisions you can read** — the application dialog reports which signal chose the category, why an entry is Auxiliary, and what the launch-target check found. Settings adds the health of each catalog source.

- **Local app inspection** — architecture, Authenticode status, file metadata and install state are resolved locally from the catalog entry.

- **Bounded scanning** — filesystem discovery has hard depth, entry and time limits; junctions and symbolic links are never followed.

## Privacy

- Catalog and metadata stay on the PC.
- No telemetry or application inventory is uploaded.
- Launch and uninstall targets are resolved inside Rust from application IDs.
- Shell command strings are not used.
- Unvalidated uninstall commands and network targets are rejected.

See [Documentation.md](Documentation.md#13-privacy-and-security) for implementation details.

## Development

Prerequisites: Node.js 22, Rust 1.88+ with the MSVC toolchain, Microsoft C++ Build Tools and Windows SDK, WebView2 Runtime, and the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/).

```powershell
npm install
npm run tauri dev
```

Verification — all of these must pass:

```powershell
npm run lint
npm run typecheck
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Coverage is reported, not gated (`npm run test:coverage`). The MSRV declared in `src-tauri/Cargo.toml` is a compatibility promise and is compiled by the MSRV job in `.github/workflows/verify.yml`.

Production build:

```powershell
npm run tauri build
```

Local artifact: `src-tauri/target/release/bundle/nsis/Windows Apps_0.3.2_x64-setup.exe`

## Support

[Technical documentation](Documentation.md) · [Releases](https://github.com/keskiyo/WindowsApps/releases) · [Telegram: @keskiyo](https://t.me/keskiyo)

<div align="center">
  <sub>Built with Tauri, Rust, React, TypeScript, Vite and native Windows APIs.</sub>
</div>
