Windows Apps is a fast, private application catalog and launcher for Windows 10 and 11. It gathers Start Menu shortcuts, installed software, Store apps, Steam games, and portable executables into one searchable catalog.

## Highlights

- Three new categories — Office & Productivity, Security & Privacy, and File & Cloud — plus stronger auto-detection put far fewer apps in Other.
- Cleaner catalog: installer stubs, framework packages, folder shortcuts and bundled binaries move to Auxiliary, and duplicate Windows built-ins merge.
- Smarter search — relevance-ranked, typo-tolerant, and it still finds apps when you type in the wrong keyboard layout — and card names shown in your Windows language.
- Easier to organize: an alphabetical grid with Favorites first, collapse a category by clicking its header, Auxiliary tools grouped by product, and an Installed-apps link.

## Install

1. Download `Windows.Apps_0.2.8_x64-setup.exe`.
2. Run it. The installer is not Authenticode-signed, so SmartScreen may show **Windows protected your PC**; choose **More info -> Run anyway**.

## What's new

- **Sharper categories** — classification weighs the Steam source, publisher, install path, the resolved target executable, and PE product name, with three new general categories: Office & Productivity, Security & Privacy (antivirus, password managers, VPN clients), and File & Cloud (archivers, cloud sync). Games, AI, and developer tools land more accurately and far fewer apps fall into Other.
- **Cleaner listing** — more noise is recognized as auxiliary or hidden: console/CLI executables, driver-staging and virtual-environment payloads, MSIX framework packages, self-extractor and folder-opening shortcuts, architecture-stub (`32.exe`/`64.exe`) binaries, and bundled browser/toolchain binaries. Entries on disconnected drives are dropped instead of shown as dead cards.
- **Fewer duplicates** — deduplication is order-independent and settles to a fixed point, so merged cards no longer reappear after a background sync. Same-name-different-version apps stay separate while 32-bit and 64-bit builds merge, and duplicated Windows built-ins (the Run dialog, File Explorer, and other localized/English shortcut pairs) collapse into a single card.
- **More resilient** — the cache degrades gracefully on category or visibility values written by a newer build instead of forcing a full rescan.
