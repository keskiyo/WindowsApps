Windows Apps is a fast, private application catalog and launcher for Windows 10 and 11. It gathers Start Menu shortcuts, installed software, Store apps, Steam games, and portable executables into one searchable catalog.

## Highlights

- Cleaner catalog - source and metadata signals separate applications from runtimes, helpers, services, language servers, documentation, and installers.
- Stable user choices - auxiliary tools can be restored permanently, hidden, or moved back without losing their canonical identity.
- More reliable maintenance - overlapping scans and updater actions are coalesced, cache backups recover interrupted writes, and icon maintenance stays separate from full scanning.
- Quiet updates - signed Windows updates finish without a separate NSIS progress window for user-writable installations.

## Install

1. Download `Windows Apps_0.2.6_x64-setup.exe`.
2. Run it. The installer is not Authenticode-signed, so SmartScreen may show **Windows protected your PC**; choose **More info -> Run anyway**.
3. Existing v0.2.5 installations can detect v0.2.6 through the signed Tauri updater manifest.

## Known Limitations

- SmartScreen publisher trust is not provided because the installer has no paid Authenticode certificate.
- Tauri updater signatures protect update integrity but do not replace Authenticode reputation.
- A production self-update cannot be considered verified until v0.2.6 is installed from public v0.2.5 through the update dialog.
