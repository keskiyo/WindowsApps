param(
  [Parameter(Mandatory = $true)]
  [string]$AssetsDir,

  [Parameter(Mandatory = $true)]
  [string]$Tag,

  [string]$Repository = "keskiyo/WindowsApps"
)

$ErrorActionPreference = "Stop"
$version = $Tag.TrimStart("v")
$latestPath = Join-Path $AssetsDir "latest.json"
$setupName = "Windows Apps_${version}_x64-setup.exe"
$setupPath = Join-Path $AssetsDir $setupName

if (-not (Test-Path -LiteralPath $latestPath -PathType Leaf)) {
  throw "latest.json is missing from $AssetsDir"
}
if (-not (Test-Path -LiteralPath $setupPath -PathType Leaf)) {
  throw "$setupName is missing from $AssetsDir"
}

$manifest = Get-Content -LiteralPath $latestPath -Raw | ConvertFrom-Json
$nsis = $manifest.platforms."windows-x86_64-nsis"
if (-not $nsis) {
  $nsis = $manifest.platforms."windows-x86_64"
}
if (-not $nsis) {
  throw "latest.json has no Windows NSIS updater target"
}

$platforms = [ordered]@{
  "windows-x86_64" = $nsis
  "windows-x86_64-nsis" = $nsis
}
$manifest.platforms = [PSCustomObject]$platforms
$manifest | Add-Member -NotePropertyName packageSize -NotePropertyValue (Get-Item -LiteralPath $setupPath).Length -Force
$manifest | Add-Member -NotePropertyName releaseUrl -NotePropertyValue "https://github.com/$Repository/releases/tag/$Tag" -Force

$json = $manifest | ConvertTo-Json -Depth 20
[IO.File]::WriteAllText($latestPath, $json, [Text.UTF8Encoding]::new($false))
Write-Output "Prepared NSIS updater manifest for $Tag"
