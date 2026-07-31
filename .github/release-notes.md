Windows Apps is a fast, private application catalog and launcher for Windows 10 and 11. It gathers Start Menu shortcuts, installed software, Store apps, Steam games, and portable executables into one searchable catalog.

## Highlights

- New App information dialog — file size and dates, CPU architecture, Authenticode signature status, and a one-click Open folder for any card.
- Three new categories — Office & Productivity, Security & Privacy, and File & Cloud — plus stronger auto-detection put far fewer apps in Other.
- Cleaner catalog: installer stubs, framework packages, folder shortcuts and bundled binaries move to Auxiliary, and duplicate Windows built-ins merge.
- Smarter search — relevance-ranked, typo-tolerant, and it still finds apps when you type in the wrong keyboard layout — and card names shown in your Windows language.

## Install

1. Download `Windows.Apps_0.2.8_x64-setup.exe`.
2. Run it. The installer is not Authenticode-signed, so SmartScreen may show **Windows protected your PC**; choose **More info -> Run anyway**.

## What's new

- **App information dialog** — open any card's details to see file size, creation and modification dates, CPU architecture (x86, x64, ARM64), whether the file is Authenticode-signed, and whether the install location still exists. Copy the path or a full report, or open the containing folder. Everything is read locally and resolved in Rust; the window never receives a path it could act on.
- **Sharper categories** — classification weighs the Steam source, publisher, install path, the resolved target executable, and PE product name, with three new general categories: Office & Productivity, Security & Privacy (antivirus, password managers, VPN clients), and File & Cloud (archivers, cloud sync). Games, AI, and developer tools land more accurately and far fewer apps fall into Other.
- **Cleaner listing** — more noise is recognized as auxiliary or hidden: console/CLI executables, driver-staging and virtual-environment payloads, MSIX framework packages, self-extractor and folder-opening shortcuts, architecture-stub (`32.exe`/`64.exe`) binaries, and bundled browser/toolchain binaries. Entries on disconnected drives are dropped instead of shown as dead cards.
- **Fewer duplicates** — deduplication is order-independent and settles to a fixed point, so merged cards no longer reappear after a background sync. Same-name-different-version apps stay separate while 32-bit and 64-bit builds merge, and duplicated Windows built-ins (the Run dialog, File Explorer, and other localized/English shortcut pairs) collapse into a single card.
- **Applications that read like installers are no longer hidden** — a name containing "Setup" or "Installer" is no longer enough to drop an app that Windows registered as a real product, so tools such as installer authoring software stay in the catalog. Shared runtimes and redistributables are matched separately and still stay out.
- **Command-line tools stay put** — a CLI executable moved to Auxiliary tools now stays there after a restart instead of reappearing in the main catalog, and a merge can no longer promote an SDK sample or a bundled component back into it.
- **Easier to organize** — an alphabetical grid with Favorites first, a category collapses by clicking anywhere on its header, Auxiliary tools are grouped by product, navigation counts always match the list they open, and Settings links straight to the Windows Installed apps page.
- **Faster with large catalogs** — category cards mount in batches as you scroll, the quick-launch palette ranks only what it shows, and the catalog counters are computed once per change instead of on every keystroke.
- **Keyboard and accessibility polish** — closing the quick-launch palette returns focus where you left it, the selected result meets AA contrast, and a single failure shows a single message.
- **More resilient** — the cache degrades gracefully on category or visibility values written by a newer build instead of forcing a full rescan, and long scans stay bounded and cancellable.