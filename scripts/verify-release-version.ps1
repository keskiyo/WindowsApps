param(
  [Parameter(Mandatory = $true)]
  [string]$Tag
)

if ($Tag -notmatch '^v(?<version>\d+\.\d+\.\d+)$') {
  throw "Release tag must use v<major>.<minor>.<patch>: $Tag"
}
$expected = $Matches.version
$package = (Get-Content package.json -Raw | ConvertFrom-Json).version
$packageLockRoot = (& node -e "process.stdout.write(require('./package-lock.json').packages[''].version)").Trim()
if ($LASTEXITCODE -ne 0) {
  throw "Could not read the root package-lock.json version"
}
$tauriConfig = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
$tauri = $tauriConfig.version
$cargoText = Get-Content src-tauri/Cargo.toml -Raw
$cargo = [regex]::Match($cargoText, '(?ms)^\[package\].*?^version\s*=\s*"([^"]+)"').Groups[1].Value
$cargoLockText = Get-Content src-tauri/Cargo.lock -Raw
$cargoLock = [regex]::Match(
  $cargoLockText,
  '(?ms)^\[\[package\]\]\r?\nname = "app"\r?\nversion = "([^"]+)"'
).Groups[1].Value

$versions = @{
  package = $package
  packageLock = $packageLockRoot
  cargo = $cargo
  cargoLock = $cargoLock
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
