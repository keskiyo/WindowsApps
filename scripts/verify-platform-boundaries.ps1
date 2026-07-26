$ErrorActionPreference = "Stop"

$matches = & rg -n -P '(?<!platform::)(?<!os::)\bwindows::|\bwinreg::' src-tauri/src
$searchExit = $LASTEXITCODE
if ($searchExit -gt 1) {
  throw "Could not scan Rust sources for Windows API imports"
}

$violations = @($matches | Where-Object {
  ($_ -replace '\\', '/') -notmatch '^src-tauri/src/platform/windows/'
})

if ($violations.Count -gt 0) {
  throw "Windows API escaped platform/windows:`n$($violations -join "`n")"
}

Write-Output "Verified Windows API ownership boundary"
