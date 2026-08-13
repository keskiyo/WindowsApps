$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$fixtureRoot = Join-Path $repoRoot "src-tauri/tests/fixtures/updater-signature"
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-updater-signature-test-$([Guid]::NewGuid().ToString('N'))"

function Decode-OuterBase64 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$SourcePath,

    [Parameter(Mandatory = $true)]
    [string]$DestinationPath
  )

  $outer = [IO.File]::ReadAllText($SourcePath, [Text.Encoding]::UTF8).Trim()
  [IO.File]::WriteAllBytes($DestinationPath, [Convert]::FromBase64String($outer))
}

function Get-IndexBlobSize {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RelativePath
  )

  $previousPreference = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    $size = & git -C $repoRoot cat-file -s ":$RelativePath"
  } catch {
    $size = $null
  } finally {
    $ErrorActionPreference = $previousPreference
  }
  if ($LASTEXITCODE -ne 0 -or "$size" -notmatch '^\s*(\d+)\s*$') {
    return $null
  }
  return [int]$Matches[1]
}

function Assert-FixtureBytesIntact {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RelativePath
  )

  $indexSize = Get-IndexBlobSize -RelativePath $RelativePath
  if ($null -eq $indexSize) {
    return
  }
  $workingSize = (Get-Item -LiteralPath (Join-Path $repoRoot $RelativePath)).Length
  if ($workingSize -ne $indexSize) {
    throw "$RelativePath is $workingSize bytes in the working tree and $indexSize bytes in git; end-of-line conversion altered a signed fixture. Keep src-tauri/tests/fixtures/updater-signature marked binary in .gitattributes"
  }
}

function Assert-Verification {
  param(
    [Parameter(Mandatory = $true)]
    [string]$PayloadPath,

    [Parameter(Mandatory = $true)]
    [string]$SignaturePath,

    [Parameter(Mandatory = $true)]
    [string]$PublicKeyPath,

    [Parameter(Mandatory = $true)]
    [bool]$ShouldSucceed
  )

  & cargo run --quiet --locked --manifest-path (Join-Path $repoRoot "src-tauri/Cargo.toml") --example verify_updater_signature -- $PayloadPath $SignaturePath $PublicKeyPath
  if (($LASTEXITCODE -eq 0) -ne $ShouldSucceed) {
    throw "Unexpected verifier result for $([IO.Path]::GetFileName($PayloadPath))"
  }
}

try {
  foreach ($fixture in @("payload.bin", "payload.bin.sig", "public.key", "wrong-public.key")) {
    Assert-FixtureBytesIntact -RelativePath "src-tauri/tests/fixtures/updater-signature/$fixture"
  }
  New-Item -ItemType Directory -Path $tempRoot | Out-Null
  $signaturePath = Join-Path $tempRoot "payload.sig"
  $publicKeyPath = Join-Path $tempRoot "public.key"
  $wrongPublicKeyPath = Join-Path $tempRoot "wrong-public.key"
  $mutatedPayloadPath = Join-Path $tempRoot "payload-mutated.bin"
  $malformedSignaturePath = Join-Path $tempRoot "payload-malformed.sig"
  $emptySignaturePath = Join-Path $tempRoot "payload-empty.sig"

  Decode-OuterBase64 (Join-Path $fixtureRoot "payload.bin.sig") $signaturePath
  Decode-OuterBase64 (Join-Path $fixtureRoot "public.key") $publicKeyPath
  Decode-OuterBase64 (Join-Path $fixtureRoot "wrong-public.key") $wrongPublicKeyPath
  [IO.File]::Copy((Join-Path $fixtureRoot "payload.bin"), $mutatedPayloadPath)
  $mutated = [IO.File]::ReadAllBytes($mutatedPayloadPath)
  $mutated[0] = $mutated[0] -bxor 1
  [IO.File]::WriteAllBytes($mutatedPayloadPath, $mutated)
  [IO.File]::WriteAllText($malformedSignaturePath, "not-base64", [Text.UTF8Encoding]::new($false))
  [IO.File]::WriteAllText($emptySignaturePath, "", [Text.UTF8Encoding]::new($false))

  Assert-Verification (Join-Path $fixtureRoot "payload.bin") $signaturePath $publicKeyPath $true
  Assert-Verification $mutatedPayloadPath $signaturePath $publicKeyPath $false
  Assert-Verification (Join-Path $fixtureRoot "payload.bin") $signaturePath $wrongPublicKeyPath $false
  Assert-Verification (Join-Path $fixtureRoot "payload.bin") $malformedSignaturePath $publicKeyPath $false
  Assert-Verification (Join-Path $fixtureRoot "payload.bin") $emptySignaturePath $publicKeyPath $false
  Assert-Verification (Join-Path $fixtureRoot "missing.bin") $signaturePath $publicKeyPath $false

  Write-Output "Verified updater signature positive and negative fixtures"
} finally {
  if (Test-Path -LiteralPath $tempRoot) {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force
  }
}
