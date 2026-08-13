Windows Apps is a fast, private application catalog and launcher for Windows 10 and 11. It gathers Start Menu shortcuts, installed software, Store apps, Steam games, and portable executables into one searchable catalog.

## Highlights

- **Private native actions** — catalog data sent to the interface no longer contains execution paths, launch arguments or uninstall commands.
- **Resilient catalog recovery** — failed or incomplete sources retain their last good records, and catalog cache writes retain a recoverable backup.
- **Recoverable Scenarios** — unavailable application entries keep their saved identity and can be identified and removed instead of forcing deletion of the whole Scenario.
- **Safer settings import** — preference imports report failed persistence instead of claiming that unsaved settings were restored.
- **Verified updates** — release automation cryptographically verifies the generated installer against the configured Tauri updater key before publishing its manifest.

## Install

1. Download `Windows.Apps_0.3.4_x64-setup.exe`.
2. Run it. The installer is not Authenticode-signed, so SmartScreen may show **Windows protected your PC**; choose **More info -> Run anyway**.