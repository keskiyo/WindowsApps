param(
  [Parameter(Mandatory = $true)][string]$CuratedPath,
  [Parameter(Mandatory = $true)][string]$GeneratedPath,
  [Parameter(Mandatory = $true)][string]$OutputPath
)

$ErrorActionPreference = "Stop"

function Read-RequiredNotes {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Label
  )

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "$Label release notes file is missing: $Path"
  }

  $content = [IO.File]::ReadAllText(
    [IO.Path]::GetFullPath($Path),
    [Text.Encoding]::UTF8
  ).Trim()
  if ([string]::IsNullOrWhiteSpace($content)) {
    throw "$Label release notes are empty: $Path"
  }
  return $content
}

$curated = Read-RequiredNotes -Path $CuratedPath -Label "Curated"
$generated = Read-RequiredNotes -Path $GeneratedPath -Label "Generated"
$newline = [Environment]::NewLine
$combined = "${curated}${newline}${newline}${generated}"
$resolvedOutput = [IO.Path]::GetFullPath($OutputPath)
[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($resolvedOutput)) | Out-Null
[IO.File]::WriteAllText(
  $resolvedOutput,
  $combined,
  [Text.UTF8Encoding]::new($false)
)
