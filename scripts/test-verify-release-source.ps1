$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$script = Join-Path $repoRoot "scripts/verify-release-source.ps1"
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-release-source-test-$([Guid]::NewGuid().ToString('N'))"

try {
  New-Item -ItemType Directory -Path $fixtureRoot | Out-Null
  Push-Location $fixtureRoot
  try {
    & git init -b master | Out-Null
    & git config user.email "release-test@example.invalid"
    & git config user.name "Release Test"

    [IO.File]::WriteAllText((Join-Path $fixtureRoot "fixture.txt"), "A")
    & git add fixture.txt
    & git commit -m "A" | Out-Null
    $baseCommit = (& git rev-parse HEAD).Trim()

    [IO.File]::WriteAllText((Join-Path $fixtureRoot "fixture.txt"), "B")
    & git commit -am "B" | Out-Null
    $masterCommit = (& git rev-parse HEAD).Trim()

    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & git switch -c feature $baseCommit 2>$null | Out-Null
    $switchExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorAction
    if ($switchExitCode -ne 0) {
      throw "Could not create feature fixture branch"
    }
    [IO.File]::WriteAllText((Join-Path $fixtureRoot "fixture.txt"), "C")
    & git commit -am "C" | Out-Null
    $featureCommit = (& git rev-parse HEAD).Trim()

    & powershell -NoProfile -File $script -Commit $masterCommit -MasterRef master | Out-Null
    if ($LASTEXITCODE -ne 0) {
      throw "Master commit was rejected"
    }

    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & powershell -NoProfile -File $script -Commit $featureCommit -MasterRef master 2>$null | Out-Null
    $featureExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorAction
    if ($featureExitCode -eq 0) {
      throw "Feature-only commit was accepted"
    }
  } finally {
    Pop-Location
  }

  Write-Output "Verified release source ancestry gate"
} finally {
  $resolvedTemp = [IO.Path]::GetFullPath($fixtureRoot)
  $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  if ($resolvedTemp.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolvedTemp)) {
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
  }
}
