$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$script = Join-Path $repoRoot "scripts/verify-release-source.ps1"
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-release-source-test-$([Guid]::NewGuid().ToString('N'))"

# Runs the gate and returns its exit code without letting an expected failure abort the test.
function Invoke-Gate {
  param([string]$Commit, [string]$MasterRef)

  $previous = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    & powershell -NoProfile -File $script -Commit $Commit -MasterRef $MasterRef 2>$null | Out-Null
    return $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previous
  }
}

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

    if ((Invoke-Gate -Commit $masterCommit -MasterRef "master") -ne 0) {
      throw "Master head commit was rejected"
    }

    # A tag left on an older master commit still resolves through `master`, so an ancestry check
    # accepts it. The release contract is "this tag is the current master", which only exact SHA
    # equality proves: an ancestor build would be signed and published after master moved on.
    if ((Invoke-Gate -Commit $baseCommit -MasterRef "master") -eq 0) {
      throw "Superseded master ancestor was accepted"
    }

    if ((Invoke-Gate -Commit $featureCommit -MasterRef "master") -eq 0) {
      throw "Feature-only commit was accepted"
    }

    # A missing ref must fail the gate, not crash it into an ambiguous state.
    if ((Invoke-Gate -Commit $masterCommit -MasterRef "origin/does-not-exist") -eq 0) {
      throw "Missing release source ref was accepted"
    }

    if ((Invoke-Gate -Commit "0000000000000000000000000000000000000000" -MasterRef "master") -eq 0) {
      throw "Missing release commit was accepted"
    }
  } finally {
    Pop-Location
  }

  Write-Output "Verified release source exact-SHA gate"
} finally {
  $resolvedTemp = [IO.Path]::GetFullPath($fixtureRoot)
  $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  if ($resolvedTemp.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolvedTemp)) {
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
  }
}
