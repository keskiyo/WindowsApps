$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$wrapperPath = Join-Path $repoRoot "scripts/verify-updater-signature.ps1"
$missingInstaller = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-missing-installer-$([Guid]::NewGuid().ToString('N')).exe"
$missingSignature = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-missing-signature-$([Guid]::NewGuid().ToString('N')).sig"

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
  $output = & powershell -NoProfile -File $wrapperPath -InstallerPath $missingInstaller -SignaturePath $missingSignature 2>&1 | Out-String
  $exitCode = $LASTEXITCODE
} finally {
  $ErrorActionPreference = $previousErrorActionPreference
}

if ($exitCode -eq 0) {
  throw "Missing release assets unexpectedly verified"
}

if ($output -notmatch "Updater signature verification failed") {
  throw "Missing release assets did not return the static verification error"
}

if ($output.Contains($missingInstaller) -or $output.Contains($missingSignature)) {
  throw "Missing release assets leaked a caller-controlled path"
}

Write-Output "Verified updater signature wrapper failure handling"
