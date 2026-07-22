# v0.2.6 Release Audit

Date: 2026-07-22

## Decision

`v0.2.6` is ready for release-candidate review. Publication remains blocked on its
manual production gate: update an installed `0.2.5` copy to `0.2.6` through Settings
-> Check updates. No tag, push, GitHub release, or published `latest.json` has been
created by this audit.

## Implemented Update Flow

1. `src-tauri/tauri.conf.json` uses `plugins.updater.windows.installMode: "quiet"`.
2. The updater keeps the existing signed Tauri + NSIS + GitHub Releases architecture.
3. The app UI shows download progress, verification, `Finishing update...`, restart,
   retry, and a permission-specific install error.
4. Current-user writable installations can update without a separate NSIS progress window.
   Protected or system-wide locations require elevation, which quiet mode cannot request.
5. Application data stays under `app_data_dir`, outside the install directory.

## Release Contract

1. Version is `0.2.6` in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`,
   `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
2. The release workflow validates the tag, runs frontend and Rust quality gates, creates
   a signed draft release, normalizes `latest.json`, verifies assets, then publishes.
3. GitHub Actions has the `TAURI_SIGNING_PRIVATE_KEY` repository secret. The private key
   is not stored locally or in the repository.
4. The actual Tauri bundle name is `Windows Apps_0.2.6_x64-setup.exe`. Release scripts,
   README, and Release.md now use this exact name; the previous `Windows.Apps_...` script
   expectation was corrected during this audit.

## Fresh Automated Evidence

| Check | Result |
| --- | --- |
| `scripts/verify-release-version.ps1 -Tag v0.2.6` | passed |
| `scripts/verify-release-notes.ps1 -Path Release.md -Tag v0.2.6` | passed |
| `npm run lint` | passed with 0 errors and 0 warnings |
| `npm run typecheck` | passed |
| `npm test -- --run` | 22 files, 147 tests passed |
| `npm run build` | passed |
| `cargo test --manifest-path src-tauri/Cargo.toml --no-fail-fast` | 191 tests passed |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | passed |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | passed |
| Local `npx tauri build` | built `Windows Apps_0.2.6_x64-setup.exe`; signing stops locally because the key remains in GitHub Actions |

## Documentation Updated

1. `README.md` now links to `v0.2.6`, uses the correct installer name, and documents quiet updates.
2. `Documentation.md` now identifies `0.2.6` and documents quiet updater behavior and its writable-location requirement.
3. `Release.md` is the public `v0.2.6` release draft with four updater-safe highlights.
4. `RELEASE_CHECKLIST.md` is reset for `v0.2.6`; it does not inherit a production updater pass from `v0.2.5`.

## Required Manual Gate

1. Install public `v0.2.5` in a writable test folder, for example `D:\Apps\Windows Apps`.
2. Create and push tag `v0.2.6`; wait for GitHub Actions to publish a signed release.
3. Start the installed `0.2.5` copy and select Settings -> Check updates.
4. Select Update & restart.
5. Confirm no NSIS progress window appears, the app restarts as `0.2.6`, the selected
   install folder is retained, application data is retained, and Windows Apps has one
   installed-app entry.
6. Download published assets and run `scripts/verify-release-assets.ps1` against them.
7. Mark the matching `RELEASE_CHECKLIST.md` gates only after those results are observed.

## Non-Blocking Follow-Up

An explicit `Download installer` recovery button can be added after release. It is not
needed for `v0.2.6`: the current dialog already offers Retry and a link to release notes.

## Release Operator Procedure

Before publishing, use the Production Updater Gate in `RELEASE_CHECKLIST.md` as the source
of truth. The essential sequence is:

1. Install public `v0.2.5` into a writable test folder outside `C:\Program Files` and create
   recognizable user state: a category, favorite, hidden app, and included scan folder.
2. Confirm the reviewed release commit has no uncommitted changes and every automated check
   is green. Create and push annotated tag `v0.2.6` exactly once.
3. Wait for GitHub Actions to create the draft release. Verify the release contains only the
   NSIS setup executable, matching Tauri updater signature, and `latest.json`; download them
   and run `scripts/verify-release-assets.ps1` locally.
4. From the installed `0.2.5` copy, use Settings -> Check updates. Confirm the update dialog
   offers `0.2.6`, displays four highlights, and links to the matching GitHub release.
5. Select Update & restart. Confirm progress is readable, no extra NSIS window appears, and
   the restarted app is `0.2.6` in the original selected install folder with user data intact.
6. Confirm one Installed apps entry remains, normal launch/search/scan/uninstall works, a
   subsequent update check reports current, and offline retry does not expose raw errors.
7. Record the workflow URL and screenshots, then mark the manual checklist. Publish only
   after every manual item passes.

## Signing Policy

Windows Apps will not use Authenticode code signing. A SmartScreen warning is expected and
is not a release blocker. Do not add a certificate, publisher reputation workflow, or signing
secret for SmartScreen. The existing Tauri updater signature is still mandatory and must never
be removed or bypassed: it verifies the downloaded update package, but does not change
SmartScreen reputation.
