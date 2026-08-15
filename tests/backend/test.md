# Backend test map

The Rust backend has no external integration-test crate. Tests live next to the
modules they exercise in `#[cfg(test)]` blocks so they can validate private and
`pub(crate)` behaviour without widening production APIs.

Snapshot: `v0.3.6`.

- Rust: **607 test entries** in **77 source files**; one developer-only timing
  test is ignored in the normal run, so a green suite reports 606 passed.
- Frontend: **600 Vitest tests** in **79 files**; it is documented here only to
  distinguish the two suites.
- Backend tests must be deterministic. Filesystem/cache tests use
  `tempfile::tempdir()` and never inspect real user directories, registry data,
  network state, or installed applications.

## Run

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

```powershell
cargo test --manifest-path src-tauri/Cargo.toml catalog::dedup::tests
```

The second form runs one module's tests. Add `--no-fail-fast` to report every
failure. The release gate also runs `cargo fmt --check` and Clippy with
`-D warnings`.

## Catalog

The catalog suite protects source discovery, installer/document separation,
classification, conservative visibility, duplicate merging, trusted target
resolution, cache migration, bounded scans, source-snapshot retention, icon
hydration and deterministic catalog output. `catalog/mod.rs` keeps the model and
assembly; Start Menu traversal, close-scope classification and stable identity
moved to `catalog/start_menu.rs`, `catalog/close.rs` and `catalog/identity.rs`,
and their tests moved with them. Cache tests also cover refusing to overwrite a
document written by a newer schema version.

| Module                                                                                           |     Tests |
| ------------------------------------------------------------------------------------------------ | --------: |
| [`catalog/mod.rs`](../../src-tauri/src/catalog/mod.rs)                                           |        64 |
| [`catalog/artifact/documentation.rs`](../../src-tauri/src/catalog/artifact/documentation.rs)     |         7 |
| [`catalog/artifact/installer.rs`](../../src-tauri/src/catalog/artifact/installer.rs)             |        18 |
| [`catalog/classify/mod.rs`](../../src-tauri/src/catalog/classify/mod.rs)                         |        15 |
| [`catalog/classify/tables.rs`](../../src-tauri/src/catalog/classify/tables.rs)                   |         5 |
| [`catalog/close.rs`](../../src-tauri/src/catalog/close.rs)                                       |         6 |
| [`catalog/dedup/merge.rs`](../../src-tauri/src/catalog/dedup/merge.rs)                           |         4 |
| [`catalog/dedup/mod.rs`](../../src-tauri/src/catalog/dedup/mod.rs)                               |        72 |
| [`catalog/details/cache.rs`](../../src-tauri/src/catalog/details/cache.rs)                       |         1 |
| [`catalog/details/read.rs`](../../src-tauri/src/catalog/details/read.rs)                         |         4 |
| [`catalog/details/target.rs`](../../src-tauri/src/catalog/details/target.rs)                     |         6 |
| [`catalog/filters.rs`](../../src-tauri/src/catalog/filters.rs)                                   |         2 |
| [`catalog/golden/mod.rs`](../../src-tauri/src/catalog/golden/mod.rs)                             |         5 |
| [`catalog/golden/properties.rs`](../../src-tauri/src/catalog/golden/properties.rs)               |         7 |
| [`catalog/golden/timings.rs`](../../src-tauri/src/catalog/golden/timings.rs)                     | 1 ignored |
| [`catalog/identity.rs`](../../src-tauri/src/catalog/identity.rs)                                 |         4 |
| [`catalog/machine.rs`](../../src-tauri/src/catalog/machine.rs)                                   |         2 |
| [`catalog/model.rs`](../../src-tauri/src/catalog/model.rs)                                       |         1 |
| [`catalog/place.rs`](../../src-tauri/src/catalog/place.rs)                                       |         4 |
| [`catalog/scan/coordinator.rs`](../../src-tauri/src/catalog/scan/coordinator.rs)                 |         5 |
| [`catalog/scan/hydration.rs`](../../src-tauri/src/catalog/scan/hydration.rs)                     |         7 |
| [`catalog/scan/incremental.rs`](../../src-tauri/src/catalog/scan/incremental.rs)                 |        15 |
| [`catalog/scan/settings.rs`](../../src-tauri/src/catalog/scan/settings.rs)                       |         2 |
| [`catalog/sources/installer_cache.rs`](../../src-tauri/src/catalog/sources/installer_cache.rs)   |         3 |
| [`catalog/sources/portable.rs`](../../src-tauri/src/catalog/sources/portable.rs)                 |         8 |
| [`catalog/sources/registry.rs`](../../src-tauri/src/catalog/sources/registry.rs)                 |        10 |
| [`catalog/sources/source.rs`](../../src-tauri/src/catalog/sources/source.rs)                     |         4 |
| [`catalog/sources/start_apps.rs`](../../src-tauri/src/catalog/sources/start_apps.rs)             |        15 |
| [`catalog/sources/steam.rs`](../../src-tauri/src/catalog/sources/steam.rs)                       |         7 |
| [`catalog/start_menu.rs`](../../src-tauri/src/catalog/start_menu.rs)                             |         7 |
| [`catalog/storage/cache.rs`](../../src-tauri/src/catalog/storage/cache.rs)                       |        18 |
| [`catalog/storage/icon_cache.rs`](../../src-tauri/src/catalog/storage/icon_cache.rs)             |        13 |
| [`catalog/sync/document.rs`](../../src-tauri/src/catalog/sync/document.rs)                       |         5 |
| [`catalog/sync/hydration.rs`](../../src-tauri/src/catalog/sync/hydration.rs)                     |         1 |
| [`catalog/sync/mod.rs`](../../src-tauri/src/catalog/sync/mod.rs)                                 |        12 |
| [`catalog/sync/portable.rs`](../../src-tauri/src/catalog/sync/portable.rs)                       |         6 |
| [`catalog/sync/scan_control.rs`](../../src-tauri/src/catalog/sync/scan_control.rs)               |         5 |
| [`catalog/target_availability.rs`](../../src-tauri/src/catalog/target_availability.rs)           |        12 |
| [`catalog/tree.rs`](../../src-tauri/src/catalog/tree.rs)                                         |         8 |
| [`catalog/visibility/markers/rules.rs`](../../src-tauri/src/catalog/visibility/markers/rules.rs) |         5 |
| [`catalog/visibility/mod.rs`](../../src-tauri/src/catalog/visibility/mod.rs)                     |        38 |
| [`catalog/visibility/report.rs`](../../src-tauri/src/catalog/visibility/report.rs)               |         1 |

Fixture corpora in `src-tauri/tests/fixtures/` anchor category, visibility and
foreign-machine decisions. They are regression data, not claims of real-world
coverage.

## Windows platform boundary

These tests validate the only layer that calls Windows APIs: executable and
folder target validation, launch/close process identity, PE metadata,
signatures, icons, registry, startup, drive discovery, global shortcut,
uninstall and watcher lifecycle.

| Module                                                                                                                       | Tests |
| ---------------------------------------------------------------------------------------------------------------------------- | ----: |
| [`platform/windows/autostart.rs`](../../src-tauri/src/platform/windows/autostart.rs)                                         |     1 |
| [`platform/windows/change_watcher.rs`](../../src-tauri/src/platform/windows/change_watcher.rs)                               |     3 |
| [`platform/windows/drives.rs`](../../src-tauri/src/platform/windows/drives.rs)                                               |     2 |
| [`platform/windows/execution/closer/frames.rs`](../../src-tauri/src/platform/windows/execution/closer/frames.rs)             |     2 |
| [`platform/windows/execution/closer/identity.rs`](../../src-tauri/src/platform/windows/execution/closer/identity.rs)         |    14 |
| [`platform/windows/execution/closer/mod.rs`](../../src-tauri/src/platform/windows/execution/closer/mod.rs)                   |    11 |
| [`platform/windows/execution/closer/processes.rs`](../../src-tauri/src/platform/windows/execution/closer/processes.rs)       |     1 |
| [`platform/windows/execution/exec_target.rs`](../../src-tauri/src/platform/windows/execution/exec_target.rs)                 |     6 |
| [`platform/windows/execution/executable_metadata.rs`](../../src-tauri/src/platform/windows/execution/executable_metadata.rs) |     7 |
| [`platform/windows/execution/folder.rs`](../../src-tauri/src/platform/windows/execution/folder.rs)                           |     3 |
| [`platform/windows/execution/launcher.rs`](../../src-tauri/src/platform/windows/execution/launcher.rs)                       |     7 |
| [`platform/windows/execution/pe.rs`](../../src-tauri/src/platform/windows/execution/pe.rs)                                   |     3 |
| [`platform/windows/execution/protected.rs`](../../src-tauri/src/platform/windows/execution/protected.rs)                     |     8 |
| [`platform/windows/execution/signature.rs`](../../src-tauri/src/platform/windows/execution/signature.rs)                     |     1 |
| [`platform/windows/icon_extractor.rs`](../../src-tauri/src/platform/windows/icon_extractor.rs)                               |    14 |
| [`platform/windows/known_folders.rs`](../../src-tauri/src/platform/windows/known_folders.rs)                                 |     1 |
| [`platform/windows/locale.rs`](../../src-tauri/src/platform/windows/locale.rs)                                               |     1 |
| [`platform/windows/registry/install_registry.rs`](../../src-tauri/src/platform/windows/registry/install_registry.rs)         |     5 |
| [`platform/windows/registry/registered_targets.rs`](../../src-tauri/src/platform/windows/registry/registered_targets.rs)     |     1 |
| [`platform/windows/registry/uninstall_registry.rs`](../../src-tauri/src/platform/windows/registry/uninstall_registry.rs)     |     2 |
| [`platform/windows/shortcuts/global_shortcut.rs`](../../src-tauri/src/platform/windows/shortcuts/global_shortcut.rs)         |     3 |
| [`platform/windows/uninstall/uninstall_history.rs`](../../src-tauri/src/platform/windows/uninstall/uninstall_history.rs)     |     3 |
| [`platform/windows/uninstall/uninstaller.rs`](../../src-tauri/src/platform/windows/uninstall/uninstaller.rs)                 |    18 |

## Core, IPC and lifecycle

The core suite checks that webview requests remain ID-only, blocking work leaves
the IPC caller thread, errors never expose local internals, interface-failure
reports stay bounded and free of control characters, window close hides to tray,
autostart hides only after tray setup succeeds, and an exact `--autostart`
argument is required.

| Module                                                                   | Tests |
| ------------------------------------------------------------------------ | ----: |
| [`app_state.rs`](../../src-tauri/src/app_state.rs)                       |    12 |
| [`commands/catalog.rs`](../../src-tauri/src/commands/catalog.rs)         |     5 |
| [`commands/close.rs`](../../src-tauri/src/commands/close.rs)             |     6 |
| [`commands/details.rs`](../../src-tauri/src/commands/details.rs)         |     1 |
| [`commands/diagnostics.rs`](../../src-tauri/src/commands/diagnostics.rs) |     3 |
| [`commands/launch.rs`](../../src-tauri/src/commands/launch.rs)       |     5 |
| [`commands/mod.rs`](../../src-tauri/src/commands/mod.rs)             |     1 |
| [`commands/settings.rs`](../../src-tauri/src/commands/settings.rs)   |     3 |
| [`commands/uninstall.rs`](../../src-tauri/src/commands/uninstall.rs) |     4 |
| [`error.rs`](../../src-tauri/src/error.rs)                           |     6 |
| [`lifecycle/mod.rs`](../../src-tauri/src/lifecycle/mod.rs)           |     6 |

## Refreshing this map

Run this after backend test changes, then update the affected module count and
the snapshot totals:

```powershell
$files = rg -l '^\s*#\[test\]' src-tauri/src -g '*.rs'
$files | ForEach-Object {
  $count = (Get-Content $_ | Select-String -Pattern '^\s*#\[test\]').Count
  "{0}: {1}" -f $_, $count
}
"Files: $($files.Count)"
"Tests: $((rg '^\s*#\[test\]' src-tauri/src -g '*.rs' | Measure-Object).Count)"
```

Use `cargo test --manifest-path src-tauri/Cargo.toml` as the execution source
of truth. Run `npm test` separately for the frontend suite.
