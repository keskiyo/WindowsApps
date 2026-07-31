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

$storeImports = Find-InTree -Path "src/store" -Pattern "lib/tauri"
if ($storeImports.Count -gt 0) {
  throw "Frontend store depends on the concrete Tauri client:`n$($storeImports -join "`n")"
}

$singleton = Find-InTree -Path "src" -Pattern "export const appStore"
if ($singleton.Count -gt 0) {
  throw "Runtime store must be created by the composition root:`n$($singleton -join "`n")"
}

Write-Output "Verified frontend dependency boundaries"
