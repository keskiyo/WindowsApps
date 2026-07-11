param(
  [Parameter(Mandatory = $true)]
  [string]$Path,

  [Parameter(Mandatory = $true)]
  [string]$Tag
)

$ErrorActionPreference = "Stop"
$errors = New-Object System.Collections.Generic.List[string]

if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
  throw "Release notes file does not exist: $Path"
}

$version = $Tag.TrimStart("v")
$text = Get-Content -LiteralPath $Path -Raw

if ($text -notmatch [regex]::Escape($Tag) -and $text -notmatch [regex]::Escape($version)) {
  $errors.Add("Release notes do not mention $Tag or $version")
}

$highlightMatch = [regex]::Match($text, '(?ms)^##\s+Highlights\s*$([\s\S]*?)(?=^##\s+|\z)')
if (-not $highlightMatch.Success) {
  $errors.Add("Release notes are missing a '## Highlights' section")
} else {
  $bullets = [regex]::Matches($highlightMatch.Groups[1].Value, '(?m)^\s*[-*]\s+(.+)$')
  if ($bullets.Count -lt 1) {
    $errors.Add("Highlights section must contain at least one bullet")
  }
  if ($bullets.Count -gt 4) {
    $errors.Add("Highlights section should contain at most 4 bullets for the updater modal")
  }
  foreach ($bullet in $bullets) {
    $value = $bullet.Groups[1].Value.Trim()
    if ($value.Length -gt 180) {
      $errors.Add("Highlight is longer than 180 characters: $value")
    }
    if ($value -match '<[^>]+>') {
      $errors.Add("Highlight contains raw HTML: $value")
    }
    if ($value -match '\b[0-9a-f]{7,40}\b') {
      $errors.Add("Highlight appears to contain a commit hash: $value")
    }
  }
}

if ($text -match 'TAURI_SIGNING_PRIVATE_KEY|BEGIN (RSA |OPENSSH |PRIVATE )?KEY|password\s*=') {
  $errors.Add("Release notes appear to contain secret-like text")
}

if ($errors.Count -gt 0) {
  throw "Release notes verification failed:`n- $($errors -join "`n- ")"
}

Write-Output "Verified release notes for $Tag"
