$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$soakPath = Join-Path $repoRoot "scripts/run-release-soak.ps1"

if (-not (Test-Path -LiteralPath $soakPath -PathType Leaf)) {
  throw "Release soak harness not found"
}

$source = [IO.File]::ReadAllText($soakPath, [Text.Encoding]::UTF8)
foreach ($requiredPattern in @(
  '\[int\]\$ProcessId',
  '\[string\]\$ProcessPath',
  '\[int\]\$RefreshCancelCycles = 100',
  '\[int\]\$IdleMinutes = 120',
  '\$logsDirectory = Join-Path \$env:LOCALAPPDATA "dev\.neiroslop\.windowsapps\\logs"'
)) {
  if ($source -notmatch $requiredPattern) {
    throw "Release soak harness is missing $requiredPattern"
  }
}

if ($source -match 'Stop-Process|taskkill|Remove-Item') {
  throw "Release soak harness must not terminate processes or delete files"
}

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
  $output = & powershell -NoProfile -File $soakPath `
    -ProcessId 0 `
    -ProcessPath (Join-Path $repoRoot "missing.exe") `
    -RefreshCancelCycles 1 `
    -IdleMinutes 0 2>&1 | Out-String
  $exitCode = $LASTEXITCODE
} finally {
  $ErrorActionPreference = $previousErrorActionPreference
}

if ($exitCode -eq 0) {
  throw "Release soak harness accepted a missing target"
}
if ($output -notmatch "Release soak target unavailable") {
  throw "Release soak harness did not return the static target error"
}

Write-Output "Verified release soak harness safety contract"
