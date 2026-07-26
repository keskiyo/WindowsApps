param(
  [Parameter(Mandatory = $true)][string]$SourcePath,
  [Parameter(Mandatory = $true)][string]$DestinationPath
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $SourcePath -PathType Leaf)) {
  throw "Curated release notes file is missing: $SourcePath"
}

$content = [IO.File]::ReadAllText(
  [IO.Path]::GetFullPath($SourcePath),
  [Text.Encoding]::UTF8
).Trim()
if ([string]::IsNullOrWhiteSpace($content)) {
  throw "Curated release notes are empty: $SourcePath"
}

$resolvedDestination = [IO.Path]::GetFullPath($DestinationPath)
[IO.Directory]::CreateDirectory(
  [IO.Path]::GetDirectoryName($resolvedDestination)
) | Out-Null
[IO.File]::WriteAllText(
  $resolvedDestination,
  $content,
  [Text.UTF8Encoding]::new($false)
)

