param(
  [Parameter(Mandatory = $true)]
  [string]$Tag
)

$expected = $Tag -replace '^v', ''
$package = (Get-Content package.json -Raw | ConvertFrom-Json).version
$tauriConfig = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
$tauri = $tauriConfig.version
$cargoText = Get-Content src-tauri/Cargo.toml -Raw
$cargo = [regex]::Match($cargoText, '(?ms)^\[package\].*?^version\s*=\s*"([^"]+)"').Groups[1].Value

$versions = @{
  package = $package
  cargo = $cargo
  tauri = $tauri
}

$mismatches = $versions.GetEnumerator() | Where-Object { $_.Value -ne $expected }
if ($mismatches) {
  throw "Tag $Tag does not match project versions: $($versions | ConvertTo-Json -Compress)"
}

if ($tauriConfig.bundle.publisher -ne "keskiyo") {
  throw "Tauri bundle publisher must be 'keskiyo', found '$($tauriConfig.bundle.publisher)'"
}

Write-Output "Verified release version $expected and publisher keskiyo"
