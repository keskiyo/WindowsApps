param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerPath,

  [Parameter(Mandatory = $true)]
  [string]$SignaturePath
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-updater-signature-$([Guid]::NewGuid().ToString('N'))"

function ConvertFrom-OuterBase64 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$EncodedValue
  )

  $trimmed = $EncodedValue.Trim()
  if ($trimmed.Length -eq 0 -or ($trimmed.Length % 4) -ne 0 -or $trimmed -notmatch '^[A-Za-z0-9+/]*={0,2}$') {
    throw "Invalid outer signature encoding"
  }

  $bytes = [Convert]::FromBase64String($trimmed)
  return [Text.UTF8Encoding]::new($false, $true).GetString($bytes)
}

try {
  $configPath = Join-Path $repoRoot "src-tauri/tauri.conf.json"
  if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf) -or -not (Test-Path -LiteralPath $SignaturePath -PathType Leaf) -or -not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
    throw "Missing verifier input"
  }

  $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
  $publicKey = $config.plugins.updater.pubkey
  if ($publicKey -isnot [string]) {
    throw "Missing updater public key"
  }

  New-Item -ItemType Directory -Path $tempRoot | Out-Null
  $innerSignaturePath = Join-Path $tempRoot "installer.sig"
  $publicKeyPath = Join-Path $tempRoot "public.key"
  $utf8 = [Text.UTF8Encoding]::new($false)
  [IO.File]::WriteAllText($innerSignaturePath, (ConvertFrom-OuterBase64 ([IO.File]::ReadAllText($SignaturePath, [Text.Encoding]::UTF8))), $utf8)
  [IO.File]::WriteAllText($publicKeyPath, (ConvertFrom-OuterBase64 $publicKey), $utf8)

  & cargo run --quiet --locked --manifest-path (Join-Path $repoRoot "src-tauri/Cargo.toml") --example verify_updater_signature -- $InstallerPath $innerSignaturePath $publicKeyPath
  if ($LASTEXITCODE -ne 0) {
    throw "Invalid updater signature"
  }
} catch {
  throw "Updater signature verification failed"
} finally {
  if (Test-Path -LiteralPath $tempRoot) {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force
  }
}
