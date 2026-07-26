Windows Apps is a fast, private application catalog and launcher for Windows 10 and 11. It gathers Start Menu shortcuts, installed software, Store apps, Steam games, and portable executables into one searchable catalog.

## Highlights

- Stable, duplicate-free catalog - deduplication is now order-independent and settles to a fixed point, so merged cards no longer reappear after a background sync. Same-name-different-version apps stay separate, while 32-bit and 64-bit builds of one product merge correctly.
- Sharper categories - Steam source, publisher, install path, and the resolved target executable now drive classification, so games, VPN clients (Utilities), AI tools, and developer tools land in the right place and far fewer apps fall into Other.
- Cleaner listing - console/CLI executables, driver-staging payloads, and virtual-environment contents are recognized as auxiliary or excluded, and entries on disconnected drives are dropped instead of shown as dead cards.
- Faster to find and read - search is relevance-ranked so name matches come first, the grid is alphabetical with Favorites pinned to the top, and version labels display a single clean `v` prefix.

## Install

1. Download `Windows.Apps_0.2.7_x64-setup.exe`.
2. Run it. The installer is not Authenticode-signed, so SmartScreen may show **Windows protected your PC**; choose **More info -> Run anyway**.