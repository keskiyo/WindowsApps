$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-manifest-test-$([Guid]::NewGuid().ToString('N'))"
$assetsDir = Join-Path $fixtureRoot "assets"
$notesPath = Join-Path $fixtureRoot "release-notes.md"

try {
  New-Item -ItemType Directory -Path $assetsDir | Out-Null
  [IO.File]::WriteAllBytes((Join-Path $assetsDir "Windows Apps_9.8.7_x64-setup.exe"), [byte[]](1, 2, 3))
  [IO.File]::WriteAllText(
    (Join-Path $assetsDir "Windows Apps_9.8.7_x64-setup.exe.sig"),
    "test-signature",
    [Text.UTF8Encoding]::new($false)
  )
  $unicodeWord = -join @(
    [char]0x041A,
    [char]0x0430,
    [char]0x0442,
    [char]0x0430,
    [char]0x043B,
    [char]0x043E,
    [char]0x0433
  )
  $expectedNotes = "## Highlights`n`n- First change.`n- Unicode: $unicodeWord."
  [IO.File]::WriteAllText($notesPath, $expectedNotes, [Text.UTF8Encoding]::new($false))

  & (Join-Path $repoRoot "scripts/prepare-release-manifest.ps1") `
    -AssetsDir $assetsDir `
    -Tag "v9.8.7" `
    -NotesPath $notesPath

  $manifestText = [IO.File]::ReadAllText(
    (Join-Path $assetsDir "latest.json"),
    [Text.Encoding]::UTF8
  )
  $manifest = $manifestText | ConvertFrom-Json
  if ($manifest.notes -ne $expectedNotes) {
    throw "Manifest notes do not match the release notes file: expected $($expectedNotes.Length) characters, got $($manifest.notes.Length)"
  }

  Write-Output "Verified release notes are embedded in latest.json"
} finally {
  if (Test-Path -LiteralPath $fixtureRoot) {
    Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
  }
}
