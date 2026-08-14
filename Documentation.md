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

Marks — favorites, hidden, promoted and installer placements — and category
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

Classification decisions are explainable in the interface, not only in source:
the application information dialog reports the discovery source, where the
record is shown and why, the recorded launch-target check where one applies,
and the signal that chose its category.

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
- Signature checks do not show Windows UI or fetch network data. Logs and
  diagnostics exclude credentials, file contents and unnecessary personal paths.
- Unsafe Windows code is confined to `platform/windows`; every unsafe block has
  an adjacent `// SAFETY:` rationale.
- The installer is not Authenticode-signed and can show SmartScreen; updater
  package integrity is protected by its separate signature.

## 14. Repository workflow

Read the nearest `AGENTS.md` before modifying a layer. Follow existing seams;
do not add dependencies, broad package updates, comments in production source,
path aliases, global utility folders, raw Tauri imports outside the approved
integration modules, or relaxed checks without explicit approval.

New frontend behavior receives a lowest-level regression test. Frontend tests
use typed complete client fakes and query observable behavior. Rust unit tests
remain colocated where private crate contracts require them. Performance tests
assert bounded semantic work rather than wall-clock thresholds.

Development commands are defined by `package.json`:

```powershell
npm run dev
npm run tauri dev
npm run lint
npm run typecheck
npm test
npm run build
```

## 16. Verification and releases

For frontend production changes run lint, typecheck, relevant tests, the full
suite for shared behavior, and production build. For backend changes run format,
Clippy with warnings denied, and relevant/full Rust tests:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

`verify.yml` runs on pull requests and `master`:

- frontend: `npm ci`, lint, typecheck, coverage test run and production build;
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
