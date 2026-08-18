# Windows Apps Technical Documentation

Technical reference for maintainers. [README](README.md) is the user-facing
overview; source and tests are the detailed implementation reference.

## 1. Product scope and environment

Windows Apps is a local Windows catalog, launcher, and organization layer. It
discovers applications, sanitizes and deduplicates results, persists a compact
cache, and exposes launch and registered uninstall through a React desktop UI.
It updates only itself from signed GitHub Releases; it never updates cataloged
third-party applications.

Out of scope: cloud sync, telemetry, metadata uploads, online enrichment,
arbitrary frontend command execution, VPN control, and direct deletion of
program directories.

| Area            | Supported implementation                              |
| --------------- | ----------------------------------------------------- |
| OS              | Windows 10 and Windows 11, x64                        |
| Desktop runtime | Tauri 2 and Microsoft Edge WebView2                   |
| Frontend        | React 18, TypeScript, Vite 6, Tailwind CSS 4, Zustand |
| Backend         | Rust 2021 and Windows APIs                            |
| Package         | NSIS setup executable                                 |
| Tests           | Vitest/Testing Library and Rust unit tests            |

## 3. Architecture and ownership

Frontend dependencies follow `app → pages → widgets → features → entities →
shared`. Slices are entered through their root `index.ts`; sibling slices do
not import one another except entity public APIs. `shared` has no catalog,
settings, or update knowledge. `scripts/verify-frontend-boundaries.ps1`
enforces the frontend boundary contract.

| Owner                | Responsibility                                                                                      |
| -------------------- | --------------------------------------------------------------------------------------------------- |
| React                | Presentation, navigation, search, dialogs, feedback, and user preferences in the root Zustand store |
| `entities/*` clients | Typed seams between UI and Tauri IPC                                                                |
| Rust commands        | Validate transport input, resolve trusted targets, delegate, and map safe errors                    |
| `catalog/*`          | Discovery, classification, deduplication, cache and incremental synchronization                     |
| `platform/windows/*` | Registry, filesystem, shell, COM, Windows handles, launch, uninstall, shortcut and autostart APIs   |
| `AppState`           | Process-wide trusted catalog targets, lifecycle and watcher ownership                               |

Runtime path:

```text
UI → model/store → entity client → Tauri IPC → command → catalog → platform/windows
```

The webview sends catalog IDs, never executable paths, registry keys, shell
commands, or uninstall commands. Rust resolves each ID from `AppState` before
any native action.

Main source areas:

| Path                                          | Owns                                                          |
| --------------------------------------------- | ------------------------------------------------------------- |
| `src/app/`                                    | Composition root, shell and root store                        |
| `src/pages/`, `src/widgets/`, `src/features/` | Screens, interface areas and user scenarios                   |
| `src/entities/`                               | App, category, scenario and system contracts/clients          |
| `src/shared/`                                 | Domain-independent UI, hooks and Tauri transport helpers      |
| `src-tauri/src/catalog/`                      | Catalog model, scanning, sources, storage, sync and decisions |
| `src-tauri/src/commands/`                     | Tauri transport adapters                                      |
| `src-tauri/src/platform/windows/`             | Windows-native boundary                                       |
| `tests/frontend/`                             | Frontend tests mirroring source ownership                     |

Two shared pieces own behaviour that used to be copied per call site, so a
change to either is a change everywhere it applies:

| Owner                                                        | Owns                                                                                                                                                                                               |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/shared/hooks/useModalDialog.ts`                         | Modal lifecycle: body-scroll lock, focus trap, initial focus, focus restoration, and the optional Escape listener. Composes `useBodyScrollLock` and `useFocusTrap` rather than reimplementing them |
| `src/pages/settings/ui/components/SettingsSectionHeader.tsx` | The icon tile, heading and description shared by every settings card                                                                                                                               |

`useModalDialog` captures the opening element once, on mount, so a dialog that
re-renders mid-flight — an installer that starts, an update that begins
downloading — still returns focus to the control the reader came from. A dialog
passes `onDismiss` only when the shared hook should own Escape; the command
palette, app picker and scenario-run dialog keep Escape in their own key
handler because it belongs to the same arrow-key navigation contract, and
`ConfirmDialog` deliberately has no Escape at all: a destructive confirmation
dismisses only through a control the reader aimed at.

The backend groups each large module by reason to change rather than by file
size. `catalog/sync/` splits source scanning (`scan_sources.rs`), per-source
health (`health.rs`), the catalog delta (`delta.rs`) and cache assembly
(`assemble.rs`), leaving `synchronize` as orchestration. `catalog/scan/`
separates the filesystem walk from the index model and executable fingerprints,
and hydration from icon extraction. `app_state/` separates catalog memory,
launch-wait limiting and uninstall history. `platform/windows/icon_extractor/`
separates image decoding, GDI bitmap encoding, shell icons and AppUserModelId
lookups, so each `unsafe` block sits next to the ownership rules it depends on.
`platform/windows/uninstall/validate.rs` holds the whole uninstall-argument
validation surface, which is the boundary that keeps a registry-supplied
command from becoming an arbitrary process.

## 4. IPC, events, and errors

Command families cover catalog reads and scans, icon hydration, launch/close,
details/folder, uninstall/history, system settings, settings backup export,
project links, update release links, stale-copy handling, and bounded
interface-failure reporting. Commands return
`Result<T, AppError>`; errors expose stable `SCREAMING_SNAKE` codes and static
safe messages. Internal paths, commands, registry values and upstream errors
never reach the webview.

Scenario close actions accept catalog IDs only. The trusted catalog classifies
close targets; only `Safe` targets may be added or executed. Critical Windows
processes and session components are counted as blocked rather than terminated.

A close asks every matching window to shut down, waits five seconds, then ends
whatever stayed open; unsaved work in those processes is lost. The wait is
reported to the interface as coarse stages over `close://progress` — asking,
a per-second countdown, then terminating — so the pause reads as deliberate
rather than as a hang. A scenario run ends in a single summary notice counting
launches, failures, closures and refusals; nothing is discarded silently.

An uninstall runs the registered uninstaller and waits for it. Exit codes are
read rather than assumed: `0`, `1641` and `3010` are completions, `1602` and `2`
are the user closing the wizard, and everything else is a failure. A cancelled
uninstall returns `UNINSTALL_CANCELLED`, is reported as information rather than
an error, and is not written to the uninstall history, because nothing was
uninstalled.

The native logger starts before application setup and retains `Info`-level
production diagnostics in Tauri's platform log directory. A root React error
boundary replaces render failures with a static recovery screen; exception
details are not displayed in the webview. The boundary reports the failure kind
and a truncated stack to the native log through `log_client_error`, which strips
control characters and bounds both fields. Dialogs sit behind their own boundary
so a failing panel closes instead of replacing the whole interface.

Catalog reads, refreshes and catalog-update events use a display DTO. The DTO
excludes uninstall targets and arguments, launch arguments, resolved execution
targets and shortcut icon paths. Rust retains those values only in the catalog
cache and trusted `AppState`; every native action still resolves the catalog ID
there. Display paths can be shown to the user but never return as action input.

IPC changes update all of these together:

1. Rust request/response type and `#[serde(rename_all = "camelCase")]`;
2. command registration in `src-tauri/src/lib.rs`;
3. owning entity TypeScript type, public API and client method;
4. every complete client fake used by tests;
5. event listener teardown and stale-generation behavior where applicable.

Events use `namespace://name`. Catalog synchronization emits full updates,
deltas, change counts, hydration patches, diagnostics and coarse scan progress.
Progress is coalesced; icon and metadata patches are emitted in bounded batches.
The synchronization lock is released before any event leaves the backend, so a
listener that calls back into the catalog cannot meet a writer still holding it.

## 5. Persisted data

Two stores contain user data:

| Store         | Owner                          | Rules                                                                                                   |
| ------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Catalog cache | `catalog/storage/cache.rs`     | Versioned, atomic, cache-first, backup-aware; corrupt primary data falls back safely.                   |
| Preferences   | `src/app/store/preferences.ts` | Versioned `localStorage` document for categories, marks, scenarios, first-seen data and unknown fields. |

Persisted-format changes must bump the appropriate version, upgrade every
supported version, default new fields, preserve unknown data, safely handle
malformed input, and test migration paths.

Preferences preserve unknown root fields, which is also how a field this version
stopped reading survives: scenario run history is no longer collected or parsed,
and the records an earlier version wrote are carried through the document
untouched rather than dropped. Invalid primary data falls back to a
one-step backup. A document written by a newer preference schema is never
overwritten. Import and local-backup restore reject unsupported/newer documents
and also refuse replacement when the installed app is older than the current
local schema. Export contains preference-backed data only: never the catalog
cache, executable paths, catalog icons, or scan folders. Each Scenario also
retains a bounded 32 KiB name/icon snapshot per app identity so unavailable
entries remain identifiable and removable; it is presentation data, never a
launch target.

Filing an application into Installers & Docs by hand records which half it
belongs to. The category holds one bucket per artifact kind, so a placement that
only said "Installers & Docs" had to pick installer, and a reference document
filed by hand landed beside setup programs with no way back. The menu therefore
opens a third level under that row — Installers or Docs — and the choice is
persisted as its own placement. The scanner's own verdict is unchanged: an entry
it already recognised as an installer or a document is not offered a move, and
upgrading a document written before this split leaves every existing placement an
installer.

Marks — favorites, hidden, promoted and manual artifact placements — and category
overrides are reconciled against the catalog on every full replacement, not only
at startup. The initial load, a refresh, a forced scan, an `apps://updated`
snapshot, a `catalog://delta` and a preferences import all run the same
reconciliation. A mark matches a record by catalog ID **or** by its durable
identity, so a rescan that reassigns IDs cannot silently clear favorites or
reveal hidden applications. Reconciliation persists only when a set actually
changed, and an empty catalog leaves the stored sets untouched instead of
treating absence as removal. First-seen timestamps follow the same path, so an
application discovered by a background delta appears in Recently added without
waiting for the next startup.

Deleting a user category removes that category's overrides rather than
rewriting them to another category. The affected applications return to the
category the classifier detected for them.

Catalog writes retain the previous known-good cache as `apps-cache.json.bak` after
an atomic replacement. In-memory catalog state updates only after the replacement
write succeeds. A cache file written by a newer schema version is never
overwritten by an older build: the catalog treats it as absent, scans into
memory, and skips the write so the file survives intact for the newer build.
This mirrors the equivalent preference rule above.
Cache/index and generated icons are separate; clearing icons does not remove the
catalog, and resetting the catalog does not remove user preferences.

## 6. Catalog operation

Sources are Start Menu shortcuts, uninstall registry entries, Start Apps and
packaged applications, Steam libraries, configured fixed-drive portable scans,
and watcher-triggered refreshes. Each source reports health independently;
failed or stale sources retain their last valid snapshot where safe.

Normal startup is cache-first. Background validation and incremental scans keep
the UI usable while source work runs. A force scan explicitly bypasses the
previous filesystem index. Scan work is cancellable, generation-aware and
bounded; no stale result may overwrite a newer generation.

Scanning starts no interpreter. Start Apps and packaged applications are read
through the shell itself: `platform/windows/apps_folder.rs` enumerates
`shell:AppsFolder` over `IShellItem`/`IEnumShellItems` and reads each entry's
display name, parsing name, `System.Link.TargetParsingPath` and — for packaged
entries — `System.AppUserModel.PackageFullName` and `PackageInstallPath`. The
executable behind a packaged entry comes from the read-only application map in
`platform/windows/registry/package_registry.rs`. Nothing about this path spawns
`powershell.exe`, so a scan no longer looks like a process launching a hidden
interpreter — behaviour that reputation-based antivirus scores against unsigned
binaries. The package map is best-effort: when it cannot be read, packaged
entries keep their name, publisher, version, install location and uninstall
target and lose only the resolved executable.

| Stage          | Invariant                                                                                                  |
| -------------- | ---------------------------------------------------------------------------------------------------------- |
| Traversal      | Fixed/local configured roots only; depth, entry, time and cancellation bounds; no reparse-point recursion. |
| Classification | Artifact, visibility and category decisions are deterministic and explainable.                             |
| Deduplication  | Canonical identity and launch evidence prevent unrelated same-name applications from merging.              |
| Cache          | Source-aware generation document; invalid data degrades safely.                                            |
| Hydration      | Icons/details are lazy, size-limited, trusted-ID-only and processed in bounded batches.                    |
| Search         | Current view/category only; literal matches rank above corrected, transliterated and fuzzy matches.        |

A query token expands into variants before matching: the literal token, the
token remapped between the English and Russian keyboard layouts, and a
Cyrillic-to-Latin transliteration. Ranking keeps the literal variant above the
rest, so a transliterated hit never displaces an exact one. Transliteration is
letter-for-letter and does not resolve loanwords whose spelling diverges.

Search stays inside the active view, and a query that also matches records
outside it reports those counts for Tools, Hidden and Installers & docs with a
direct link to the owning view, rather than leaving the matches invisible.
Typing into search from Settings, More or Scenarios switches to the catalog.
Empty user categories are not rendered while a query is active. The command
palette opened with an empty query lists favorites first, then recently added
applications.

Before the first scan the catalog shows what will be scanned, that nothing runs
automatically at startup, and that the data stays on the device, with the scan
action and a link to folder settings on the same card.

A scenario list is filled from a modal picker that searches the catalog and
switches on several applications at once, each row naming the category the
application sits in and carrying its switch on the right. The dialog is portalled
to `document.body` at a fixed size, so a transformed ancestor cannot become its
containing block and push it off the window. It states which list is being
filled — launch or close — and for which scenario, since both lists open the
same dialog. It offers exactly the applications the All apps
view shows: hidden records, installers and auxiliary tools stay out, while an
entry already stored in a scenario still resolves against the whole catalog so
it keeps its name and icon. Candidates are ordered by name, case-insensitively
and with digits compared as numbers, until a query replaces that order with
relevance ranking; a query that names a category also brings in the applications
of that category, after the entries the query matched by name. Rows arrive a
batch at a time — revealed by scrolling to the end of the list or by a control
that names how many are left — and skip layout and paint off-screen through the
same `content-visibility` mechanism the catalog cards use; the footer counts the
whole result set, not the rendered batch.
Applications the list
already holds are not offered, and an application held by the opposite list of
the same scenario is shown locked with the reason rather than accepted and
rejected afterwards. Confirming adds the whole set in one step; every close
target that carries a risk warning still costs its own separate confirmation,
and declining one leaves the rest of the set intact. Deleting a scenario is
confirmed in its own dialog, since the delete control sits beside rename and the
configuration it removes cannot be recovered.

Every destructive confirmation — uninstall, delete category, delete scenario —
is the one `shared/ui/ConfirmDialog`: same layout, same wording positions, same
Cancel and named danger action, painted from tokens and portalled to
`document.body`. It dismisses only through Cancel or its close control: neither
Escape nor a click on the backdrop discards it, so a confirmation cannot be lost
to a stray keystroke while it is being read. Cancel takes focus on open, which
keeps the keyboard exit one keystroke away, and focus returns to the control that
opened the dialog. Optional detail — the uninstall route, for instance — renders
in a block between the description and the actions.

Returning focus is not specific to confirmations: every dialog hands it back to
the control that opened it, so closing one never drops the keyboard at the top
of the catalog. The application information dialog used to be the exception and
left focus on `<body>`; it now goes through the same shared modal lifecycle as
the rest, and a regression test opens it from a control and asserts the control
has focus again after it closes. The navigation drawer restores focus to its
menu button explicitly, because the burger outlives the panel.

The More page previews every scenario while they all fit its card and spends the
last slot on a "View all" row only once a scenario is left out of the preview.

Classification decisions are explainable in the interface, not only in source:
the application information dialog reports the discovery source, where the
record is shown and why, the recorded launch-target check where one applies,
and the signal that chose its category.

An entry that removes software is never an installation artifact, however
generic its target: a shortcut is read as an uninstall action when its first
word says so (`Uninstall …`, `Удалить …`, `Деинсталляция …`) or when its launch
arguments carry an uninstall switch (`/x{…}`, `--uninstall`, `REMOVE=ALL`).
Both checks live in `catalog::filters` beside the uninstall-target path rule,
so the artifact classifier and the visibility rules read the same definition.
Product names built from the same words — Revo Uninstaller, IObit Uninstaller —
stay applications, because only the first word counts.

Windows built-in tools are recognised by their whole name rather than by a
substring, so a vendor product can never inherit the category by containing the
word. A trailing qualifier does not defeat that rule: the comparison also runs
against the name with its parenthetical suffixes removed, which is what files
`Windows PowerShell (x86)` and `Источники данных ODBC (64-разрядная версия)`
with the tools they are variants of. A name that consists of nothing but a
qualifier matches no rule at all.

Characters that imitate a Latin letter are folded before any signal is read, so
the micro sign in `µTorrent` is compared as `u` and the record reaches the same
rule as its ASCII spelling.

Shell binaries are not category evidence. A Start Menu folder shortcut resolves
to `explorer.exe`, so the executable of a Windows feature is read from the tools
that are only ever themselves; `Проводник` is recognised by its name instead.

An entry generated for a `file://` target is documentation whenever the target
is a document, so a registered `…/doc/index.html` is filed with the other
documentation rather than as an application of the product it documents.

Category rules are applied twice, and the second pass is the one that matters.
A source records a first guess from the name and path it has; assembly re-runs
`classify_app` over the finished record, where the publisher, product name,
description and resolved executable are available and outweigh the name. A rule
that needs vendor evidence therefore belongs on those fields, and measuring a
source's output in isolation understates the result: on a 432-record scan of
three sources, 51 entries carried no category, while the assembled catalog of
217 left 5.

A component shipped as part of Windows is recognised by its publisher, not by
where it is installed. Store packages live under `Program Files\WindowsApps`
whether Microsoft ships them with the operating system or sells them alongside
everyone else's, so treating that tree as evidence swept Xbox, the Game Bar, Dev
Home, To Do and Power Automate into Windows Features. The package publisher id
`cw5n1h2txyewy` is the identity of `CN=Microsoft Windows` and belongs only to
genuine shell components; first-party applications that are still part of a
Windows installation, such as Calculator or Maps, stay on the explicit
package-name list beside it.

### Signals that do not depend on knowing the product

A table of product names can only recognise software it already lists, so three
signals carry records the table has never seen:

- **What the system registered.** `catalog::machine::Associations` reads the file
  types and URL protocols an executable claims — `Applications\<exe>\SupportedTypes`
  and the `Capabilities` of every entry in `RegisteredApplications`. An
  application that owns `.flac` is a player and one that owns `mailto:` is a mail
  client, whatever it calls itself. Extensions are a closed, standardised set,
  unlike product names. The map is a machine fact read once per scan and is never
  persisted, so no cached record can go stale against it.
- **What the vendor wrote about itself.** `catalog::classify::vocabulary` holds
  plain purpose words in Russian and English — `media player`, `графический
редактор`, `terminal emulator` — matched against the description and the
  ProductName of the binary. Its weight sits between the threshold and a
  product-name match: a description alone leaves `Other`, but never outranks a
  named product. Two independent generic matches do.
- **Where the entry lives.** Start Menu groups and vendor folders (`\Игры\`,
  `\Development\`) score at path weight, the weakest evidence of the three.

`WindowsFeatures` is deliberately excluded from the vocabulary: it is recognised
by whole values only, so a third-party shortcut described as «Проводник» cannot
inherit it. Its one substring field is the install path, which carries the two
machine facts that need no product name: a component living under
`\Windows\System32`, `\SysWOW64`, `\Windows\Speech` or `\Windows\SystemApps\`, and
a Store package whose family starts `\WindowsApps\Microsoft.`. A Store package
from any other publisher is untouched by that rule.

Two kinds of reported metadata are treated as no evidence at all rather than as
weak evidence, because scoring them is worse than ignoring them:

- **Localized resource stubs.** Windows reports `MSPAINT.EXE.MUI` as the original
  file name, and dropping only the last extension leaves `mspaint.exe`, which
  matches no executable rule. The `.mui` suffix is removed before the stem is
  taken.
- **Packaging-toolkit metadata.** An InstallShield shortcut reports publisher
  `Acresso Software Inc.`, product and description `InstallShield`, and
  `_IsIcoRes.exe` as the binary — all describing the installer, not the product.
  Those values are blanked, so the product name in the shortcut's own title is
  what the rules read. The same applies to Inno Setup, NSIS and Nullsoft
  defaults.

One more install path carries a product without naming it. An MSI-advertised
shortcut reports the Windows Installer cache — `C:\Windows\Installer\{GUID}` —
instead of a program folder, so neither the name nor the path says what the
product is. Microsoft registers Office under a fixed product-code family, so
`\Installer\{90120000`, `{90140000`, `{90150000` and `{90160000` file the whole
suite at once: `Access 2016` and `Publisher 2016` carry no other evidence and
would otherwise stay unclassified for the same reason as their telemetry and
language companions.

Records that still match nothing are listed under Settings → Advanced with every
signal the classifier read, so a machine with unfamiliar software shows what the
tables are missing rather than a silent pile in `Other`. One action copies the
whole list — signals, source, artifact, visibility and the recorded reason — as
plain text, so an unfamiliar machine can be reported without retyping it.

That report is how the rules above were derived rather than guessed. A copied
list of 60 unrecognised records from an unfamiliar Windows install was replayed
through the classifier: 13 were reachable from machine facts alone (Windows
paths, Store package families, the `.mui` and packaging-metadata fixes), and the
remaining product names were added only where the record itself carried an
unambiguous signal — a vendor's own product family, a driver standard, a shared
install root. 48 of the 60 now classify and four more are recognised as
components; the rest stay in `Other` on purpose, because a rule that fitted them
would fit only that one machine. Every one of those records is a fixture in
`catalog_categories.json` or `catalog_visibility.json`, including guards that
must **not** match: a Store package from a non-Microsoft publisher is not a
Windows feature, and a product named after a maintenance verb is still a
product.

A second report, 29 records from an unrelated Windows install, was replayed the
same way. Five needed nothing: four Microsoft Store packages were already
answered by the package-family path, and the catalog recognised itself by
publisher. The other 24 fell into three shapes. A versioned vendor tree names a
family its executables never do — `1cestart`, `1cv8`, `1cv8c` and `1cv8s` say
nothing, while `\1cv8\` and the publisher spelled in both Cyrillic and Latin say
1C:Enterprise. A component names its own install root instead of itself:
`Unload kernel module` and `Peace` are only a verb and an author until the
`Cheat Engine` and `EqualizerAPO` trees they sit in answer for them. And a
localized name can drop every product word it had — `Кнопки сервисов Яндекса на
панели задач` leaves only `YandexPin.exe` to read, and it must not be read as
the browser whose folder it shares.

That last shape is why a shared install root scores below a vendor. A dongle
driver installer ships inside the 1C tree, so the tree would file it as business
software; its own publisher and executable outrank the path and keep it with the
maintenance tools. The reverse guard already existed for Windows features, and
this is the same rule seen from the other side: a path answers only for a record
that has nothing else to say.

A query that names a category also returns the applications filed under it,
after the entries matched by name. The catalog search and the scenario picker
share one implementation so both answer the same question the same way.

The golden catalog harness under `src-tauri/src/catalog/golden/` protects
identity, launch descriptor, category, visibility and dedup contracts with
fixtures and deterministic generated properties. Recording a new baseline is
deliberate:

```powershell
$env:WINDOWSAPPS_GOLDEN_UPDATE = "1"; cargo test --manifest-path src-tauri/Cargo.toml golden
```

## 10. Desktop operations

### Launch and close

Launch uses a trusted catalog target through the Windows shell. Launch feedback
clears from a backend input-idle signal when available, otherwise from a bounded
client fallback. A launch action does not expose arbitrary command execution.

Close operates on trusted process identities, not visible windows alone. It
first requests normal close, then performs a bounded recheck before terminating
survivors. The application process, Windows-critical images and unsafe process
groups are excluded. Store/URI entries without a safe executable close target
are reported unavailable. Steam closes by its exact `steam.exe` target, never
by the Steam installation directory, so games remain explicit Scenario entries.

### Uninstall

Uninstall previews expose only application identity, publisher, source and safe
removal mechanism. Execution requires confirmation and uses a validated,
Rust-owned target. History excludes paths, command lines and internal errors.

Removing a packaged application goes through
`Windows.Management.Deployment.PackageManager` in
`platform/windows/uninstall/msix.rs`, not through an interpreter. Removal is
silent, as before. The call runs on a thread the module spawns for it and joins:
that thread enters a multithreaded apartment, because the blocking wait on the
deployment operation pumps no messages and would deadlock a completion
marshalled into the process's single-threaded apartment. Failures are reported
as the deployment error code alone, without a package name or path.

The product no longer starts `powershell.exe` anywhere. `cargo clippy` enforces
it: removing the last call site left `exec_target::system_powershell` unused and
failed the build under `-D warnings`, so the helper is gone and a new caller
would have to reintroduce it deliberately.

### Windows integration and updates

- Tray, startup, global shortcut and window lifecycle are backend-owned.
- An enabled Windows startup entry launches the installed application with an
  internal exact `--autostart` argument. That launch hides the main window only
  after the tray is ready; the tray's **Open Windows Apps** action restores it.
  A normal launch remains visible, and a tray initialization failure keeps the
  window visible.
- WebView2 uses Tauri's silent bootstrapper when missing.
- The updater checks the signed release manifest on startup. An available
  version is announced by a dismissible banner in the shell notice area beside
  the stale-copy and preference-write notices; it never opens a dialog by
  itself. The update dialog opens only from the banner's action, and download,
  verification, installation and restart remain modal from that point.
- Updater signatures are verified with the public key in `tauri.conf.json`.
  The private key exists only in CI secrets.
- Download progress reports real bytes/percentage; verification, installation
  and restart are indeterminate stages. Update failures retain a safe retry UI.
- Update checks are silent offline, when current, and outside the desktop
  runtime used by browser development/tests.

## 13. Privacy and security

- Catalog discovery, classification and preferences are local; no telemetry,
  catalog upload or online metadata lookup is configured.
- CSP and Tauri capabilities are least-privilege. Do not widen capabilities,
  CSP, updater keys/endpoints, bundle identifier or publisher without explicit
  approval.
- Inbound IDs and payload sizes are validated before lookup, filesystem work or
  memory allocation. The backend resolves display data separately from command
  targets.
- Folder actions accept only existing trusted local folders. UNC, device,
  relative and packaged-app paths are refused.
- Native process execution uses a fixed executable plus argument vector, never
  a shell string. Registry, shortcuts, filesystem entries and IPC payloads are
  untrusted.
- A launched executable starts in its own directory, the way Explorer starts it.
  Leaving the shell's working directory unset handed the child this process's
  own directory, so anything it wrote relative to it landed in the catalog's
  folder — Rufus dropped its `rufus.com` console companion there. Shortcuts keep
  the working directory recorded in the `.lnk`; AppUserModelIds and `steam://`
  URIs have no directory of their own and pass none.
- Signature checks do not show Windows UI or fetch network data. Logs and
  diagnostics exclude credentials, file contents and unnecessary personal paths.
- Unsafe Windows code is confined to `platform/windows`; every unsafe block has
  an adjacent `// SAFETY:` rationale.
- The installer is not Authenticode-signed and can show SmartScreen; updater
  package integrity is protected by its separate signature.
- Because the binary is unsigned and every release starts at zero reputation,
  behaviour that reputation-based antivirus scores heavily is avoided on
  purpose: no interpreter is started, and `mainBinaryName` ships the executable
  as `WindowsApps.exe` rather than the Cargo package's generic `app.exe`. The
  NSIS template records `MainBinaryName` in the uninstall key and deletes the
  previously installed binary when the name changes, so an update from a build
  that shipped `app.exe` leaves nothing behind. The Windows startup value is
  rewritten to the running executable on every start of an installed copy, so a
  renamed, moved or updated copy never leaves a startup entry pointing at a
  path that no longer exists.

## 14. Repository workflow

Follow existing seams; do not add dependencies, broad package updates, comments
in production source, path aliases, global utility folders, raw Tauri imports
outside the approved integration modules, or relaxed checks without explicit
approval. §3 above states which layer owns what, and the boundary scripts in
`scripts/` fail the `contracts` job when a change crosses one — those two are
the reference a clone actually carries.

New frontend behavior receives a lowest-level regression test. Frontend tests
use typed complete client fakes and query observable behavior. Rust unit tests
remain colocated where private crate contracts require them. Performance tests
assert bounded semantic work rather than wall-clock thresholds.

Development commands are defined by `package.json`:

```powershell
npm run dev
npm run tauri dev
npm run lint
npm run format:check
npm run typecheck
npm test
npm run build
```

Formatting is owned by Prettier through `prettier.config.mjs` and `.prettierignore`.
`endOfLine` is `auto` because `core.autocrlf` is enabled on Windows checkouts while
the hosted runners are not; pinning it to `lf` would make `format:check` disagree
with itself across platforms. Generated files — `package-lock.json`, the golden
catalog baselines re-recorded by `serde_json`, `THIRD_PARTY_NOTICES.md`,
`.github/release-notes.md` and the SHA-pinned workflows — stay ignored so their
generators remain the only writers.

## 16. Verification and releases

For frontend production changes run lint, formatting check, typecheck, relevant
tests, the full suite for shared behavior, and production build. For backend
changes run format,
Clippy with warnings denied, and relevant/full Rust tests:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

`verify.yml` runs on pull requests and `master`:

- frontend: `npm ci`, lint, formatting check, typecheck, coverage test run and
  production build;
- backend: tests on `windows-latest` and `windows-2022`, plus format and Clippy;
- MSRV: compile at the `rust-version` declared in `src-tauri/Cargo.toml`;
- contracts: frontend/platform boundaries, release-script tests and dependency
  audit gates, including updater-signature fixtures.

The signed updater-signature fixtures under `src-tauri/tests/fixtures/` are
byte streams, not text. `.gitattributes` marks that directory `binary` so a
checkout with `core.autocrlf` enabled cannot rewrite line endings and invalidate
the detached signature; `test-verify-updater-signature.ps1` also compares each
fixture against its size in the index and names that failure explicitly.

Node.js `22.22.2` and Rust `1.96.0` are pinned in `.node-version` and
`rust-toolchain.toml`. Cargo verification uses `--locked`; the separate MSRV
job builds with Rust `1.88.0`.

Release preparation also provides `scripts/run-release-soak.ps1`.
It binds to one exact application PID and executable path, records operator-confirmed
Refresh/Cancel outcomes, samples memory during the idle/watcher window, and writes
local evidence under `.1localDocuments`. It never controls or terminates the target
process. This record is reference-machine evidence, not a replacement for CI or the
clean Windows acceptance matrix.

Runtime `npm audit --omit=dev --audit-level=high` admits no exceptions. High or
critical development-only advisories require dated entries in
`.github/npm-audit-exceptions.json`; stale or undocumented exceptions fail CI.

Release is tag-only: a `v*` tag on the exact `master` SHA triggers
`release.yml`. Version values must agree across npm/Cargo manifests, lockfiles
and `tauri.conf.json`. The release workflow reruns critical gates, builds and
signs the NSIS bundle, verifies its detached updater signature against the
configured public key, creates/verifies `latest.json`, then publishes the draft.
Published tags are immutable; corrections use a new patch version. The project
source is MIT-licensed; third-party notices are recorded in
`THIRD_PARTY_NOTICES.md`.

## 17. Troubleshooting

| Problem                       | First action                                                                                             |
| ----------------------------- | -------------------------------------------------------------------------------------------------------- |
| Catalog empty                 | Use **Scan for apps**; the first complete scan is explicit.                                              |
| Duplicate or stale entries    | Refresh; then use **Settings → Advanced → Catalog maintenance → Reset catalog cache**.                   |
| Missing application           | Check permanent local drive/exclusions; add a folder in **Settings → Advanced → Application discovery**. |
| Old version or icon           | Refresh; missing icons can be repaired from catalog maintenance without losing preferences.              |
| Shortcut/startup fails        | Re-enable it in Settings; Windows policy or another process can block registration.                      |
| Uninstall unavailable         | The catalog record has no trusted, parseable uninstall target.                                           |
| Catalog stays on placeholders | The event connection failed; use **Retry** in the notice. Refresh and launch keep working without it.    |
| A panel closes by itself      | That dialog failed to render; the failure is in the application log and the catalog is unaffected.       |
| Search finds nothing here     | Check the counts under the results; a match may live in Tools, Hidden or Installers & docs.              |
| Update/download failure       | Retry from the update dialog or use the linked GitHub release.                                           |
| SmartScreen warning           | Expected for the unsigned NSIS installer; verify the release source and updater signature.               |
