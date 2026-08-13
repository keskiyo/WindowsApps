Windows Apps is a fast, private application catalog and launcher for Windows 10 and 11. It gathers Start Menu shortcuts, installed software, Store apps, Steam games, and portable executables into one searchable catalog.

## Highlights

- **Resilient catalog recovery** — failed or incomplete sources retain their last good records, and catalog cache writes retain a recoverable backup. A cache reset that fails now leaves the visible catalog untouched instead of emptying it.
- **Recoverable Scenarios** — unavailable application entries keep their saved identity and can be identified and removed instead of forcing deletion of the whole Scenario.
- **Interface recovery** — a failed event connection no longer leaves the catalog on a loading placeholder, and offers a retry. A failing dialog closes on its own instead of replacing the whole window, and the failure is written to the local application log.

## Install

1. Download `Windows.Apps_0.3.4_x64-setup.exe`.
2. Run it. The installer is not Authenticode-signed, so SmartScreen may show **Windows protected your PC**; choose **More info -> Run anyway**.
