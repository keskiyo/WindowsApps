$ErrorActionPreference = "Stop"

# Uses Select-String rather than an external grep so the check runs on any Windows PowerShell
# host and on a clean CI runner, neither of which is guaranteed to have ripgrep installed.
function Find-InTree {
  param([string]$Path, [string]$Pattern)

  if (-not (Test-Path -LiteralPath $Path)) {
    throw "Boundary scan path does not exist: $Path"
  }
  return @(
    Get-ChildItem -LiteralPath $Path -Recurse -File |
      Select-String -Pattern $Pattern -CaseSensitive |
      ForEach-Object { "{0}:{1}:{2}" -f $_.Path, $_.LineNumber, $_.Line.Trim() }
  )
}

$storeImports = Find-InTree -Path "src/app/store" -Pattern "api/appsClient|api/systemClient|api/tauri/client"
if ($storeImports.Count -gt 0) {
  throw "Frontend store depends on the concrete Tauri client:`n$($storeImports -join "`n")"
}

$singleton = Find-InTree -Path "src" -Pattern "export const appStore"
if ($singleton.Count -gt 0) {
  throw "Runtime store must be created by the composition root:`n$($singleton -join "`n")"
}

# The concrete IPC clients are composition-time wiring. Anything else that reaches for them is
# building a second runtime path around the AppsClient / SystemClient seam the tests substitute.
$clientImports = @(
  Find-InTree -Path "src" -Pattern "entities/(app/api/appsClient|system/api/systemClient)'" |
    Where-Object { $_ -notmatch "src[\\/]app[\\/]main\.tsx" }
)
if ($clientImports.Count -gt 0) {
  throw "Only the composition root may import a concrete IPC client:`n$($clientImports -join "`n")"
}

# --- Layer and slice boundaries -------------------------------------------------------------
#
# app -> pages -> widgets -> features -> entities -> shared. An import may only point to the
# same rank or lower, and a slice is entered through its root `index.ts`, never through
# `ui/`, `model/`, `lib/` or `api/`. Sibling slices of one layer do not import each other.

$rank = @{ "app" = 5; "pages" = 4; "widgets" = 3; "features" = 2; "entities" = 1; "shared" = 0 }
# Layers whose top-level directories are slices with a public API. `shared` and `app` are not
# sliced: `shared` is segment-only, `app` is the single composition root.
$sliced = @("pages", "widgets", "features", "entities")

# The one deliberate upward edge. The store is a single root store, so a per-card launching
# subscription has to reach it; splitting the store into per-domain slices is the fix, and
# until then this stays a named exception rather than a silent one. A second entry belongs
# here only after that split, never as a shortcut around the layering.
$allowedUpward = @{
  "src/features/launch-app/model/useIsLaunching.ts" = "src/app/store/appStoreContext"
}

function Resolve-Import {
  param([string]$FromDir, [string]$Spec)

  $parts = New-Object System.Collections.Generic.List[string]
  foreach ($p in ($FromDir -split "/")) { if ($p -ne "") { [void]$parts.Add($p) } }
  foreach ($p in ($Spec -split "/")) {
    if ($p -eq "" -or $p -eq ".") { continue }
    if ($p -eq "..") { if ($parts.Count -gt 0) { $parts.RemoveAt($parts.Count - 1) } }
    else { [void]$parts.Add($p) }
  }
  return ($parts -join "/")
}

$violations = New-Object System.Collections.Generic.List[string]
$root = (Get-Location).Path

foreach ($file in (Get-ChildItem -LiteralPath "src" -Recurse -File -Include *.ts, *.tsx)) {
  $from = $file.FullName.Substring($root.Length + 1).Replace("\", "/")
  $fromParts = $from -split "/"
  $fromLayer = $fromParts[1]
  if (-not $rank.ContainsKey($fromLayer)) { continue }
  $fromSlice = if ($sliced -contains $fromLayer) { $fromParts[2] } else { "" }
  $fromDir = ($from -replace "/[^/]+$", "")

  foreach ($match in ([regex]::Matches([System.IO.File]::ReadAllText($file.FullName), "from\s+'(\.[^']+)'"))) {
    $target = Resolve-Import $fromDir $match.Groups[1].Value
    $toParts = $target -split "/"
    if ($toParts.Count -lt 2 -or $toParts[0] -ne "src") { continue }
    $toLayer = $toParts[1]
    if (-not $rank.ContainsKey($toLayer)) { continue }

    if ($allowedUpward[$from] -eq $target) { continue }

    if ($rank[$toLayer] -gt $rank[$fromLayer]) {
      $violations.Add("${from}: imports upward into '$toLayer' ($target)")
      continue
    }
    if (-not ($sliced -contains $toLayer)) { continue }

    $toSlice = $toParts[2]
    if ($fromLayer -eq $toLayer -and $fromSlice -eq $toSlice) { continue }
    if ($fromLayer -eq $toLayer) {
      # An App has a Category and the uninstall history quotes an App mechanism, so the entity
      # layer is the one place a sibling reference is real. It still goes through the public API.
      if ($fromLayer -eq "entities" -and $toParts.Count -eq 3) { continue }
      $violations.Add("${from}: imports sibling slice '$toLayer/$toSlice' ($target)")
      continue
    }
    # The composition root is the only place that may wire a concrete entity client; the check
    # above already pins which files those are.
    if ($from -eq "src/app/main.tsx" -and $toParts[3] -eq "api") { continue }
    if ($toParts.Count -gt 3) {
      $violations.Add("${from}: reaches into '$toLayer/$toSlice' internals ($target)")
    }
  }
}

if ($violations.Count -gt 0) {
  throw "Frontend layer/slice boundary violations:`n$($violations -join "`n")"
}

Write-Output "Verified frontend dependency boundaries"
