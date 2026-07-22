param(
  [Parameter(Mandatory = $true)]
  [string]$AssetsDir,

  [Parameter(Mandatory = $true)]
  [string]$Tag
)

$ErrorActionPreference = "Stop"
$errors = New-Object System.Collections.Generic.List[string]

if (-not (Test-Path -LiteralPath $AssetsDir -PathType Container)) {
  throw "Assets directory does not exist: $AssetsDir"
}

$version = $Tag.TrimStart("v")
$latestPath = Join-Path $AssetsDir "latest.json"
$setupName = "Windows Apps_${version}_x64-setup.exe"
$setupPath = Join-Path $AssetsDir $setupName
$signaturePath = "$setupPath.sig"

foreach ($path in @($latestPath, $setupPath, $signaturePath)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    $errors.Add("Required release asset is missing: $([IO.Path]::GetFileName($path))")
  }
}

$installers = Get-ChildItem -LiteralPath $AssetsDir -File -Filter "*.exe"
if ($installers.Count -ne 1) {
  $errors.Add("Expected exactly one setup .exe, found $($installers.Count): $($installers.Name -join ', ')")
}

if ($installers.Count -ge 1 -and $installers[0].Name -ne $setupName) {
  $errors.Add("Setup file name does not match ${Tag}: $($installers[0].Name)")
}

$unexpected = Get-ChildItem -LiteralPath $AssetsDir -File | Where-Object {
  $_.Name -match '\.(pdb|log|key|pub|tmp)$' -or
  $_.Name -match 'debug' -or
  ($_.Name -match 'Windows\.Apps_([0-9]+\.[0-9]+\.[0-9]+)' -and $Matches[1] -ne $version)
}
if ($unexpected) {
  $errors.Add("Unexpected release assets: $($unexpected.Name -join ', ')")
}

if (Test-Path -LiteralPath $latestPath -PathType Leaf) {
  try {
    $manifestText = Get-Content -LiteralPath $latestPath -Raw
    $manifest = $manifestText | ConvertFrom-Json

    if ($manifest.version -ne $version) {
      $errors.Add("latest.json version '$($manifest.version)' does not match $version")
    }

    if ($manifestText -match '(?i)\.msi|windows-x86_64-msi') {
      $errors.Add("latest.json must not contain MSI targets or URLs")
    }

    $expectedSize = (Get-Item -LiteralPath $setupPath).Length
    if ($manifest.packageSize -ne $expectedSize) {
      $errors.Add("latest.json packageSize '$($manifest.packageSize)' does not match $expectedSize")
    }

    $expectedReleaseUrl = "https://github.com/keskiyo/WindowsApps/releases/tag/$Tag"
    if ($manifest.releaseUrl -ne $expectedReleaseUrl) {
      $errors.Add("latest.json releaseUrl '$($manifest.releaseUrl)' does not match $expectedReleaseUrl")
    }

    if (-not $manifest.pub_date) {
      $errors.Add("latest.json has no publication date")
    }

    if ($manifestText -notmatch [regex]::Escape($setupName)) {
      $errors.Add("latest.json does not reference $setupName")
    }

    $genericTarget = $manifest.platforms."windows-x86_64"
    $nsisTarget = $manifest.platforms."windows-x86_64-nsis"
    foreach ($target in @(
      @{ Name = "windows-x86_64"; Value = $genericTarget },
      @{ Name = "windows-x86_64-nsis"; Value = $nsisTarget }
    )) {
      if (-not $target.Value) {
        $errors.Add("latest.json is missing updater target '$($target.Name)'")
        continue
      }

      $targetFile = [IO.Path]::GetFileName(([Uri]$target.Value.url).AbsolutePath)
      if ($targetFile -ne $setupName) {
        $errors.Add("latest.json target '$($target.Name)' references '$targetFile' instead of $setupName")
      }
      if (-not $target.Value.signature) {
        $errors.Add("latest.json target '$($target.Name)' has no updater signature")
      }
    }

    if ($genericTarget.url -ne $nsisTarget.url -or $genericTarget.signature -ne $nsisTarget.signature) {
      $errors.Add("Generic and NSIS updater targets must reference the same package and signature")
    }

    if ($manifestText -notmatch '"signature"\s*:\s*"[^"]+"') {
      $errors.Add("latest.json does not contain an updater signature")
    }
  } catch {
    $errors.Add("latest.json is not valid JSON: $($_.Exception.Message)")
  }
}

if ($errors.Count -gt 0) {
  throw "Release asset verification failed:`n- $($errors -join "`n- ")"
}

Write-Output "Verified release assets for $Tag"
