<div align="center">
  <img src="public/app-icon.png" width="96" height="96" alt="Windows Apps logo">

# Windows Apps

**A fast, private, and organized launcher for every application on your Windows PC.**

[![Version](https://img.shields.io/badge/version-0.1.0-5B8CFF?style=flat-square)](../../releases/latest)
![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?style=flat-square&logo=windows11&logoColor=white)
![Architecture](https://img.shields.io/badge/architecture-x64-334155?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![Privacy](https://img.shields.io/badge/catalog-local--first-16A34A?style=flat-square)

[**Download the latest Windows release**](../../releases/latest) · [Technical documentation](Documentation.md) · [Telegram](https://t.me/keskiyo)

</div>

---

## Why Windows Apps?

Windows accumulates applications across Start Menu shortcuts, executable registrations, Store packages, launchers, and system tools. Windows Apps turns those scattered sources into one clean, searchable catalog without uploading your application list anywhere.

It starts from a local cache, scans only when requested, removes maintenance noise, prefers useful shortcuts over duplicate executables, and keeps your organization choices between sessions.

## Features

|     | Capability                | What it provides                                                                                                                                                               |
| --- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| ◈   | **Unified catalog**       | Discovers Start Menu shortcuts, registered software, Windows apps, Steam libraries, and portable executables across fixed local drives.                                        |
| ◇   | **Smart cleanup**         | Filters installers, uninstallers, resource entries, broken names, and common duplicate records.                                                                                |
| ↕   | **Flexible organization** | Automatic categories, custom categories, renaming, and moving applications. Click a category name to open it or drag the same row to reorder it.                              |
| ★   | **Favorites and Hidden**  | Keep important apps close and move catalog noise into a reversible Hidden view.                                                                                                |
| ▶   | **Native launching**      | Opens `.lnk`, `.exe`, shell targets, and packaged applications through the correct Windows mechanism.                                                                          |
| ⓘ   | **Local metadata**        | Uses Windows, package, shortcut, and executable metadata. Missing information remains `Unknown` instead of being invented.                                                     |
| ⛨   | **Safe uninstall**        | Runs the registered quiet, standard, or MSIX uninstall route directly. If Windows exposes no safe route, uninstall stays disabled; program folders are never deleted manually. |
| ⌁   | **Background access**     | Closing the window keeps the app in the system tray. `Win+Shift+Q` or a tray click restores it.                                                                                |
| ◎   | **Controlled scanning**   | Shows progress, supports cancellation, ignores removable/network drives, and accepts custom include/exclude folders in Settings.                                                |

## Download and install

1. Open the [latest GitHub Release](../../releases/latest).
2. Download the Windows x64 setup file ending in `-setup.exe`.
3. Run the installer and start **Windows Apps**.
4. Select **Scan for apps** when you are ready to build the local catalog.

The scan checks permanent local drives regardless of their letters or folder names. USB and network drives are intentionally ignored.

> [!IMPORTANT]
> Version 0.1.0 is not code-signed. Microsoft Defender SmartScreen may show an "unrecognized app" warning for community builds. Always download the installer from this repository's Releases page and verify its published SHA-256 checksum.

## System requirements

| Requirement         | Supported value                                                                      |
| ------------------- | ------------------------------------------------------------------------------------ |
| Operating system    | Windows 10 or Windows 11                                                             |
| Architecture        | x64 (64-bit)                                                                         |
| Web runtime         | Microsoft Edge WebView2 Runtime                                                      |
| Internet connection | Not required for catalog use; the installer may download WebView2 when it is missing |

## Privacy by design

- Application discovery and categorization happen on your computer.
- Catalog data, icons, favorites, hidden entries, and custom categories are stored locally.
- Fixed-drive and Steam discovery run locally; no drive inventory is uploaded.
- Windows Apps does not send your software inventory to an external service.
- It does not fetch online descriptions or invent missing metadata.
- The only user-initiated web link in 0.1.0 is the Telegram contact in Settings.

## Keyboard and tray behavior

- `Win+Shift+Q` restores and focuses the application using the physical Q key, independent of the active English or Russian layout.
- Closing the main window hides it in the Windows notification area and keeps the shortcut active.
- Left-click the tray icon or select **Open Windows Apps** to restore the window.
- Select **Quit** from the tray menu to end the background process completely.
- **Launch when Windows starts** can be enabled from Settings for the current Windows account.

## Development

Install [Node.js](https://nodejs.org/), npm, stable Rust with the MSVC toolchain, and the official [Tauri prerequisites for Windows](https://v2.tauri.app/start/prerequisites/).

```powershell
npm install
npm run tauri dev
```

Run the verification suite:

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## Build from source

Create production Windows bundles on an x64 Windows machine:

```powershell
npm run tauri build
```

The downloadable NSIS setup executable is generated under:

```text
src-tauri/target/release/bundle/nsis/Windows Apps_0.1.0_x64-setup.exe
```

Upload this NSIS setup executable as the primary GitHub Release asset. The MSI
bundle is optional and is generated under `src-tauri/target/release/bundle/msi/`.

See [Documentation.md](Documentation.md) for architecture, troubleshooting, release verification, checksums, and the complete GitHub Release procedure.

## Documentation and support

- [Technical documentation](Documentation.md)
- [Latest release](../../releases/latest)
- [Telegram: @keskiyo](https://t.me/keskiyo)

<div align="center">
  <sub>Built with Tauri, Rust, React, TypeScript, and native Windows APIs.</sub>
</div>
