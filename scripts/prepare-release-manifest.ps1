param(
  [Parameter(Mandatory = $true)]
  [string]$AssetsDir,

  [Parameter(Mandatory = $true)]
  [string]$Tag,

  [Parameter(Mandatory = $true)]
  [string]$NotesPath,

  [string]$Repository = "keskiyo/WindowsApps"
)

$ErrorActionPreference = "Stop"
$version = $Tag.TrimStart("v")
$latestPath = Join-Path $AssetsDir "latest.json"
$setupName = "Windows Apps_${version}_x64-setup.exe"
$setupPath = Join-Path $AssetsDir $setupName
$signaturePath = "$setupPath.sig"
$publishedSetupName = $setupName.Replace(" ", ".")

if (-not (Test-Path -LiteralPath $setupPath -PathType Leaf)) {
  throw "$setupName is missing from $AssetsDir"
}
if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
  throw "$setupName.sig is missing from $AssetsDir"
}
if (-not (Test-Path -LiteralPath $NotesPath -PathType Leaf)) {
  throw "Release notes file is missing: $NotesPath"
}

$signature = (Get-Content -LiteralPath $signaturePath -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($signature)) {
  throw "$setupName.sig is empty"
}
$notes = [IO.File]::ReadAllText($NotesPath, [Text.Encoding]::UTF8).Trim()
if ([string]::IsNullOrWhiteSpace($notes)) {
  throw "Release notes file is empty: $NotesPath"
}

$downloadUrl = "https://github.com/$Repository/releases/download/$Tag/$([Uri]::EscapeDataString($publishedSetupName))"
$target = [ordered]@{
  signature = $signature
  url = $downloadUrl
}
$platforms = [ordered]@{
  "windows-x86_64" = $target
  "windows-x86_64-nsis" = $target
}
$manifest = [ordered]@{
  version = $version
  notes = $notes
  pub_date = [DateTime]::UtcNow.ToString("o")
  platforms = $platforms
  packageSize = (Get-Item -LiteralPath $setupPath).Length
  releaseUrl = "https://github.com/$Repository/releases/tag/$Tag"
}

$json = $manifest | ConvertTo-Json -Depth 20
[IO.File]::WriteAllText($latestPath, $json, [Text.UTF8Encoding]::new($false))
Write-Output "Prepared NSIS updater manifest for $Tag"
