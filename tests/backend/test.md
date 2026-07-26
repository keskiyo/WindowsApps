# Backend test map

The Rust backend has **no separate test tree**. Every backend test lives **colocated** in a
`#[cfg(test)] mod tests` block at the bottom of the module it exercises. This file is the index of
where those backend tests are and what they cover.

- Total: **272 tests** across **34 files** (all plain `#[test]`; no integration `tests/` crate).
- Convention: [`src-tauri/AGENTS.md`](../../src-tauri/AGENTS.md) §10 — colocated, `tempfile::tempdir()`
  for any filesystem/cache test, never touching real user dirs, registry, network, or installed apps;
  deterministic and order-independent; every fixed bug gets a failing-before-fix regression test.

## Run

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml catalog::dedup::tests
```

Second form runs one module's tests (swap the path). Add `--no-fail-fast` to see every failure.

## Catalog — scanning, dedup, classification, cache (`src-tauri/src/catalog/`)

| File                                                                       | Tests | What it verifies                                                                                                                                                                                                                                                                                                                                                                        |
| -------------------------------------------------------------------------- | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`dedup/mod.rs`](../../src-tauri/src/catalog/dedup/mod.rs)                 | 48    | Evidence-based merging: identity matching, `should_merge` thresholds, publisher/version/install-root vetoes (same-name-different-version stays split; x86/x64 variants merge). Deterministic `canonical_order_key` + fixed-point loop (no duplicate oscillation after background sync), differential harness (`resolve_apps` vs reference) and scale invariant over generated catalogs. |
| [`mod.rs`](../../src-tauri/src/catalog/mod.rs)                             | 55    | `classify_app` category signals (Steam source, publisher, install-path markers, resolved-exe keywords → Games/AI/Development/Utilities-VPN/Editors/Media), legacy `classify(name,path)`, `sanitize`/`sanitize_reported`, phantom-drop (`target_is_present`), console demotion, and the built-in category set.                                                                           |
| [`visibility/mod.rs`](../../src-tauri/src/catalog/visibility/mod.rs)       | 22    | `classify_visibility` (Primary/Auxiliary/Rejected) with reason codes: command environments (VS prompts, IDLE, MySQL CLI, safe-mode, interpreter host+args), product components, console applications, runtime/helper/service demotion, and sticky reasons across a merge.                                                                                                               |
| [`visibility/report.rs`](../../src-tauri/src/catalog/visibility/report.rs) | 1     | Dev-only visibility report: user-profile path redaction (`<USERPROFILE>` prefix).                                                                                                                                                                                                                                                                                                       |
| [`registry.rs`](../../src-tauri/src/catalog/registry.rs)                   | 9     | Uninstall-key parsing into candidates: user/machine hives, missing/denied keys, display-name/publisher extraction, filtering of system components.                                                                                                                                                                                                                                      |
| [`start_apps.rs`](../../src-tauri/src/catalog/start_apps.rs)               | 8     | Start Menu / AUMID shortcut discovery, target resolution, and identity fields used downstream.                                                                                                                                                                                                                                                                                          |
| [`steam.rs`](../../src-tauri/src/catalog/steam.rs)                         | 6     | Steam library / `steamapps` manifest parsing into game candidates with the Steam source kind.                                                                                                                                                                                                                                                                                           |
| [`portable.rs`](../../src-tauri/src/catalog/portable.rs)                   | 8     | Portable-executable discovery and exclusions (`.venv`/`venv`/`site-packages`/`chipset_software`/`issetupprerequisites`, driver-staging), product-metadata coherence gating.                                                                                                                                                                                                             |
| [`incremental.rs`](../../src-tauri/src/catalog/incremental.rs)             | 7     | Incremental scan reuse: unchanged directories skipped, only-changed re-read, bounded work.                                                                                                                                                                                                                                                                                              |
| [`cache.rs`](../../src-tauri/src/catalog/cache.rs)                         | 9     | `apps-cache.json` load/store: exact-version load, older-version upgrade, pre-schema array, `#[serde(default)]` new fields, atomic write + `.bak` recovery on corrupt/malformed data.                                                                                                                                                                                                    |
| [`icon_cache.rs`](../../src-tauri/src/catalog/icon_cache.rs)               | 8     | Content-addressed icon storage, cache key stability, and clear/rebuild paths.                                                                                                                                                                                                                                                                                                           |
| [`hydration.rs`](../../src-tauri/src/catalog/hydration.rs)                 | 6     | Lazy batched icon hydration: bounded patch batches, visible-first priority.                                                                                                                                                                                                                                                                                                             |
| [`scan_coordinator.rs`](../../src-tauri/src/catalog/scan_coordinator.rs)   | 3     | Single scan entry point, coalescing overlapping scans, cancellation.                                                                                                                                                                                                                                                                                                                    |
| [`sync.rs`](../../src-tauri/src/catalog/sync.rs)                           | 2     | Post-scan pipeline: `sanitize_reported`, phantom-drop `retain`, console demotion ordering.                                                                                                                                                                                                                                                                                              |
| [`source.rs`](../../src-tauri/src/catalog/source.rs)                       | 2     | `catalog::source` plug-in seam for app sources.                                                                                                                                                                                                                                                                                                                                         |
| [`scan_settings.rs`](../../src-tauri/src/catalog/scan_settings.rs)         | 2     | Scan-path normalization (absolute, case-insensitive dedupe), include/exclude handling.                                                                                                                                                                                                                                                                                                  |

## Platform / Windows boundary (`src-tauri/src/platform/windows/`)

| File                                                                                    | Tests | What it verifies                                                                                                                                        |
| --------------------------------------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`uninstaller.rs`](../../src-tauri/src/platform/windows/uninstaller.rs)                 | 17    | Uninstall mechanism selection (vendor/MSI/MSIX), UNC/network rejection, no recursive folder deletion, safe removal-mechanism labels.                    |
| [`executable_metadata.rs`](../../src-tauri/src/platform/windows/executable_metadata.rs) | 8     | PE header parsing: `is_console_subsystem` (DOS→PE→optional-header Subsystem, CUI vs GUI), ProductName/OriginalFilename extraction, malformed-PE safety. |
| [`exec_target.rs`](../../src-tauri/src/platform/windows/exec_target.rs)                 | 6     | Launch target building from a fixed exe + argument vector; refusal of shell strings, interpreters, and UNC targets.                                     |
| [`launcher.rs`](../../src-tauri/src/platform/windows/launcher.rs)                       | 5     | Native launch mechanisms (shortcut/exe/shell/packaged/Steam) dispatch.                                                                                  |
| [`install_registry.rs`](../../src-tauri/src/platform/windows/install_registry.rs)       | 5     | Installed-product registry reads across hives/views.                                                                                                    |
| [`icon_extractor.rs`](../../src-tauri/src/platform/windows/icon_extractor.rs)           | 5     | HICON extraction with RAII guards, missing-icon handling.                                                                                               |
| [`change_watcher.rs`](../../src-tauri/src/platform/windows/change_watcher.rs)           | 3     | Filesystem change watcher lifecycle owned by `AppState`.                                                                                                |
| [`uninstall_history.rs`](../../src-tauri/src/platform/windows/uninstall_history.rs)     | 3     | History ring (newest 100) storing only name/publisher/mechanism/result — never command/path/args/error.                                                 |
| [`global_shortcut.rs`](../../src-tauri/src/platform/windows/global_shortcut.rs)         | 2     | `Win+Shift+Q` physical-key (layout-independent) binding.                                                                                                |
| [`drives.rs`](../../src-tauri/src/platform/windows/drives.rs)                           | 2     | Fixed-drive enumeration; removable/optical/network excluded.                                                                                            |
| [`autostart.rs`](../../src-tauri/src/platform/windows/autostart.rs)                     | 1     | Launch-at-sign-in registration toggle.                                                                                                                  |

## Core — state, IPC transport, errors, lifecycle (`src-tauri/src/`)

| File                                                                         | Tests | What it verifies                                                                                                                                                           |
| ---------------------------------------------------------------------------- | ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`app_state.rs`](../../src-tauri/src/app_state.rs)                           | 5     | `AppState` id→target resolution, uninstall-history recording, failed-uninstall returns original error.                                                                     |
| [`catalog_sync/document.rs`](../../src-tauri/src/catalog_sync/document.rs)   | 4     | Sanitized-cache load, stale-uninstall reset, generation-guarded write-back (never overwrites a newer catalog).                                                             |
| [`catalog_sync/hydration.rs`](../../src-tauri/src/catalog_sync/hydration.rs) | 1     | Hydration patch merge persists new icons without erasing existing ones.                                                                                                    |
| [`error.rs`](../../src-tauri/src/error.rs)                                   | 4     | `AppError` code ↔ `safe_message` mapping (static, no path/command/registry/username), `From<String>` → `Other`, distinct cancel/coalesce/unavailable/validation/not-found. |
| [`lifecycle/mod.rs`](../../src-tauri/src/lifecycle/mod.rs)                   | 3     | App lifecycle wiring, tray/window hide-to-tray, shutdown ownership.                                                                                                        |
| [`commands/mod.rs`](../../src-tauri/src/commands/mod.rs)                     | 1     | `run_blocking` transport adapter runs work off the calling thread.                                                                                                         |
| [`commands/settings.rs`](../../src-tauri/src/commands/settings.rs)           | 1     | Scan-path normalization (absolute, case-insensitive dedupe) at the command boundary.                                                                                       |

## Notes

- Counts are `#[test]` functions per file. Modules without a `#[cfg(test)]` block (e.g.
  `platform/windows/shortcut.rs`, `uninstall_registry.rs`) are not listed. Update counts when adding tests.
- When you change dedup, visibility, cache schema, or an IPC contract, extend the fixture-backed tests
  in the matching file above — do not add a parallel test tree.
