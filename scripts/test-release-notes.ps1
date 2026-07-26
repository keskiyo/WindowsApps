$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$syncScript = Join-Path $repoRoot "scripts/sync-release-notes.ps1"
$composeScript = Join-Path $repoRoot "scripts/compose-release-notes.ps1"
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-release-notes-test-$([Guid]::NewGuid().ToString('N'))"
$sourcePath = Join-Path $fixtureRoot "Release.md"
$snapshotPath = Join-Path $fixtureRoot "snapshot/release-notes.md"
$generatedPath = Join-Path $fixtureRoot "generated-release-notes.md"
$combinedPath = Join-Path $fixtureRoot "release-notes.md"
$utf8NoBom = [Text.UTF8Encoding]::new($false)

function Assert-Throws {
  param(
    [Parameter(Mandatory = $true)][scriptblock]$Action,
    [Parameter(Mandatory = $true)][string]$ExpectedMessage
  )

  try {
    & $Action
  } catch {
    if (-not $_.Exception.Message.Contains($ExpectedMessage)) {
      throw "Expected error containing '$ExpectedMessage', got '$($_.Exception.Message)'"
    }
    return
  }

  throw "Expected error containing '$ExpectedMessage'"
}

try {
  New-Item -ItemType Directory -Path $fixtureRoot | Out-Null
  $newline = [Environment]::NewLine
  $expectedSnapshot = "## Highlights${newline}${newline}- Curated change: Каталог."
  $expectedGenerated = "## What's Changed${newline}${newline}- Generated entry."
  $expectedCombined = "${expectedSnapshot}${newline}${newline}${expectedGenerated}"

  [IO.File]::WriteAllText(
    $sourcePath,
    "${newline}  ${expectedSnapshot}  ${newline}",
    $utf8NoBom
  )
  [IO.File]::WriteAllText($generatedPath, $expectedGenerated, $utf8NoBom)

  & $syncScript -SourcePath $sourcePath -DestinationPath $snapshotPath
  $actualSnapshot = [IO.File]::ReadAllText($snapshotPath, [Text.Encoding]::UTF8)
  if ($actualSnapshot -ne $expectedSnapshot) {
    throw "Synced release notes do not match the trimmed curated source"
  }

  & $composeScript `
    -CuratedPath $snapshotPath `
    -GeneratedPath $generatedPath `
    -OutputPath $combinedPath
  $actualCombined = [IO.File]::ReadAllText($combinedPath, [Text.Encoding]::UTF8)
  if ($actualCombined -ne $expectedCombined) {
    throw "Combined release notes do not preserve curated-first ordering"
  }

  Assert-Throws `
    -ExpectedMessage "Curated release notes file is missing" `
    -Action {
      & $syncScript `
        -SourcePath (Join-Path $fixtureRoot "missing.md") `
        -DestinationPath $snapshotPath
    }

  [IO.File]::WriteAllText($sourcePath, " `t${newline}", $utf8NoBom)
  Assert-Throws `
    -ExpectedMessage "Curated release notes are empty" `
    -Action {
      & $syncScript -SourcePath $sourcePath -DestinationPath $snapshotPath
    }

  [IO.File]::WriteAllText($snapshotPath, $expectedSnapshot, $utf8NoBom)
  [IO.File]::WriteAllText($generatedPath, " `t${newline}", $utf8NoBom)
  Assert-Throws `
    -ExpectedMessage "Generated release notes are empty" `
    -Action {
      & $composeScript `
        -CuratedPath $snapshotPath `
        -GeneratedPath $generatedPath `
        -OutputPath $combinedPath
    }

  Write-Output "Verified curated release-note synchronization and composition"
} finally {
  $resolvedTemp = [IO.Path]::GetFullPath($fixtureRoot)
  $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  if ($resolvedTemp.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolvedTemp)) {
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
  }
}
