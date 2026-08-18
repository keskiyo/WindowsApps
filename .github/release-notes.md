Windows Apps is a fast, private application catalog and launcher for Windows 10 and 11. It gathers Start Menu shortcuts, installed software, Store apps, Steam games, and portable executables into one searchable catalog.

## Highlights

- **A scan starts no interpreter** — Start Menu entries, Store packages and packaged-app removal go through Windows directly, so nothing launches `powershell.exe` behind the catalog.
- **More software is recognised** — vendor install trees, localized names and Store package families now answer for records whose own name says nothing, including 1C:Enterprise, Cheat Engine, Equalizer APO and Office's advertised shortcuts.
- **Programs start in their own folder** — a launched application receives its install directory, so files it writes beside itself no longer land in the catalog's folder.
- **Focus returns where you left it** — every dialog hands the keyboard back to the control that opened it, including the application information dialog.

## Install

1. Download `Windows.Apps_0.3.7_x64-setup.exe`.
2. Run it. The installer is not Authenticode-signed, so SmartScreen may show **Windows protected your PC**; choose **More info -> Run anyway**.