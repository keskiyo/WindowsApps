# Windows Release Runbook

This file is the source of truth for publishing Windows releases. `Release.md` and `INFO.md` are local-only, ignored by Git, and must never be required by CI.

## Non-negotiable rules

- Release only from `master`. Do not create a release branch unless the user explicitly requests one.
- Never publish from an uncommitted or unpushed tree.
- Never move or recreate a tag after its release has been published. Use a new patch version for corrections.
- Do not publish manually while the release workflow is running.
- Do not modify updater keys, bundle identifiers, publisher metadata, installer targets, or GitHub secrets as part of a routine release.
- The installer is intentionally not Authenticode-signed. A SmartScreen warning is expected. The Tauri updater signature is still mandatory.

## 1. Prepare the version

1. Choose a new semantic version, for example `0.2.7`, and tag `v0.2.7`.
2. Set the same version in:
   - `package.json` and the package lockfile through npm;
   - `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` through Cargo;
   - `src-tauri/tauri.conf.json`.
3. Update tracked user documentation and changelog content that describes the release.
4. Confirm local planning files are ignored and untracked: `git check-ignore Release.md INFO.md` and `git ls-files Release.md INFO.md`.

## 2. Run the local release gate

Run every command and stop on the first failure:

```powershell
npm ci
npm run lint
npm test
npm run typecheck
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
powershell -NoProfile -File scripts/verify-release-version.ps1 -Tag "v<VERSION>"
```

Also confirm:

- `src-tauri/tauri.conf.json` uses only the `nsis` target and keeps `createUpdaterArtifacts: true`;
- the updater endpoint is `https://github.com/keskiyo/WindowsApps/releases/latest/download/latest.json`;
- `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` exist in GitHub Actions secrets;
- `git status --short` contains only intentional release changes;
- `master` is the only normal development branch required for the release.

## 3. Publish once

1. Commit the prepared release on `master` and push `master`.
2. Verify the pushed commit SHA matches local `HEAD`.
3. Create the tag on that exact commit: `git tag v<VERSION>`.
4. Push only that tag: `git push origin v<VERSION>`.
5. Wait for `.github/workflows/release.yml` to finish. Do not alter the tag while it is running.

The workflow creates a draft release, builds and signs the NSIS installer, generates GitHub release notes from commit history, creates and verifies `latest.json`, and publishes only after all checks pass.

## 4. Post-publish verification

Do not call the release complete until all checks pass:

- GitHub Actions run conclusion is `success`;
- the release is neither a draft nor a prerelease;
- assets contain exactly `latest.json`, `Windows.Apps_<VERSION>_x64-setup.exe`, and its `.sig`;
- `latest.json.version` equals `<VERSION>`;
- both Windows targets reference the published dotted asset name, not the local space-separated bundle name;
- `packageSize` equals the downloaded installer size;
- updater URLs return HTTP 200 and the signature is non-empty;
- the public release page contains generated notes;
- a machine with the previous published version detects, downloads, installs, and relaunches the new version through `Settings -> Check updates`;
- the update preserves the original installation scope and directory where supported by the existing installer mode.

## 5. Failure handling

- Read the first failed step and fix its cause on `master`; do not repeatedly retry unchanged code.
- If failure occurs before publication, remove only the failed draft when necessary, then rerun according to the repository workflow.
- If a release was already published with bad assets or manifest data, do not silently replace its tag. Prepare a new patch release.
- Verify actual GitHub asset names before writing manifest URLs. Tauri's local file is `Windows Apps_<VERSION>_x64-setup.exe`, while the published asset is `Windows.Apps_<VERSION>_x64-setup.exe`.
- After every workflow correction, reproduce the failed script locally with a representative fixture before pushing another tag.
