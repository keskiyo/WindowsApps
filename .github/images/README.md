# README screenshots

The five images the root `README.md` references. Until a file exists, its slot renders as a broken
image on GitHub — capture them before publishing the release, or remove the matching tag.

They live here rather than in `public/` because `public/` is copied into the application bundle by
Vite, and rather than in `docs/`, which is git-ignored.

## Referenced by the README right now

Both files must exist or the README shows a broken image. Keep these exact names — no spaces, since
a space has to be percent-encoded in a Markdown image URL and silently breaks when it is not.

| File                   | What it is                                                                                     |
| ---------------------- | ---------------------------------------------------------------------------------------------- |
| `catalog.png`          | Hero shot: main catalog, wide window, sidebar visible, populated categories with real icons.     |
| `auxiliary-tools.png`  | The Auxiliary tools view with the count tiles across the top.                                    |

## Worth adding later

Not referenced yet — add the file, then add the `<img>` tag to the README in the same change.

| File               | What to capture                                                                                                          |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `app-info.png`     | The App information dialog with everything populated — size, dates, architecture, a **Verified** signature. This is the v0.2.8 headline feature and currently has no picture anywhere. |
| `search.png`       | The `Ctrl+K` quick-launch palette open, with a query typed and ranked results.                                             |
| `settings.png`     | Settings showing scan diagnostics and the update section.                                                                  |

## Capture guidance

- **Window size:** capture at a wide desktop width so the sidebar is visible, not the narrow drawer
  layout. The two-column table pairs images side by side, so keep all four non-hero shots the same
  aspect ratio.
- **Format:** PNG. Keep each file under roughly 500 KB — resize to about 1600 px wide before saving
  rather than shipping a raw 4K capture.
- **Content:** these are published publicly. Do not capture a window that shows a personal folder
  path, a machine name, a user name, or software you would rather not disclose. Favor common
  applications. The App information dialog in particular shows a full file path — pick an app
  installed under `C:\Program Files`.
- **Theme:** use one theme consistently across all five.

## Known asset issue

`public/app-icon.png` is ~1.9 MB and the README renders it at 96×96. It is also the in-app icon, so
it is not purely a documentation concern — but a downscaled copy for the README header, or a smaller
source asset, would remove almost two megabytes from every page load.
