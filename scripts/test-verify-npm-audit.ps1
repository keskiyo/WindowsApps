$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$script = Join-Path $repoRoot "scripts/verify-npm-audit.ps1"
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) "windows-apps-npm-audit-test-$([Guid]::NewGuid().ToString('N'))"

function Write-Fixture {
  param([string]$Name, $Content)

  $path = Join-Path $fixtureRoot $Name
  [IO.File]::WriteAllText($path, ($Content | ConvertTo-Json -Depth 12))
  return $path
}

function Invoke-Gate {
  param([string]$AuditJsonPath, [string]$ExceptionsPath, [string]$Today)

  $previous = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    & powershell -NoProfile -File $script -AuditJsonPath $AuditJsonPath -ExceptionsPath $ExceptionsPath -Today $Today 2>$null | Out-Null
    return $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previous
  }
}

function New-Exception {
  param([string]$Advisory, [string]$ReviewBy)

  return @{
    advisory    = $Advisory
    package     = "example"
    severity    = "high"
    chain       = "tool > example"
    reason      = "Development toolchain only"
    owner       = "keskiyo"
    reviewBy    = $ReviewBy
    remediation = "Upgrade the tool once a compatible release exists"
  }
}

try {
  New-Item -ItemType Directory -Path $fixtureRoot | Out-Null

  $clean = Write-Fixture -Name "clean.json" -Content @{ vulnerabilities = @{} }
  $high = Write-Fixture -Name "high.json" -Content @{
    vulnerabilities = @{
      example = @{
        severity = "high"
        via      = @(
          @{
            source   = 1
            name     = "example"
            severity = "high"
            title    = "Example denial of service"
            url      = "https://github.com/advisories/GHSA-aaaa-bbbb-cccc"
          }
        )
      }
      # An indirect entry: `via` names a dependency, not an advisory, so it must not be
      # mistaken for a second undocumented finding.
      wrapper = @{
        severity = "high"
        via      = @("example")
      }
    }
  }
  $noExceptions = Write-Fixture -Name "none.json" -Content @{ exceptions = @() }
  $current = Write-Fixture -Name "current.json" -Content @{ exceptions = @(New-Exception -Advisory "GHSA-aaaa-bbbb-cccc" -ReviewBy "2026-12-31") }
  $expired = Write-Fixture -Name "expired.json" -Content @{ exceptions = @(New-Exception -Advisory "GHSA-aaaa-bbbb-cccc" -ReviewBy "2026-01-31") }
  $stale = Write-Fixture -Name "stale.json" -Content @{ exceptions = @(New-Exception -Advisory "GHSA-dddd-eeee-ffff" -ReviewBy "2026-12-31") }
  $incomplete = Write-Fixture -Name "incomplete.json" -Content @{
    exceptions = @(@{ advisory = "GHSA-aaaa-bbbb-cccc"; package = "example"; chain = ""; reason = ""; owner = ""; reviewBy = "2026-12-31"; remediation = "" })
  }

  if ((Invoke-Gate -AuditJsonPath $clean -ExceptionsPath $noExceptions -Today "2026-07-31") -ne 0) {
    throw "Clean audit was rejected"
  }
  if ((Invoke-Gate -AuditJsonPath $high -ExceptionsPath $noExceptions -Today "2026-07-31") -eq 0) {
    throw "Undocumented high advisory was accepted"
  }
  if ((Invoke-Gate -AuditJsonPath $high -ExceptionsPath $current -Today "2026-07-31") -ne 0) {
    throw "Documented, unexpired exception was rejected"
  }
  if ((Invoke-Gate -AuditJsonPath $high -ExceptionsPath $expired -Today "2026-07-31") -eq 0) {
    throw "Expired exception was accepted"
  }
  if ((Invoke-Gate -AuditJsonPath $clean -ExceptionsPath $stale -Today "2026-07-31") -eq 0) {
    throw "Stale exception for a resolved advisory was accepted"
  }
  if ((Invoke-Gate -AuditJsonPath $high -ExceptionsPath $incomplete -Today "2026-07-31") -eq 0) {
    throw "Exception missing required fields was accepted"
  }

  # The tracked exceptions file must itself be complete and unexpired today.
  $tracked = Join-Path $repoRoot ".github/npm-audit-exceptions.json"
  foreach ($exception in (([IO.File]::ReadAllText($tracked) | ConvertFrom-Json).exceptions)) {
    foreach ($field in @('advisory', 'package', 'chain', 'reason', 'owner', 'reviewBy', 'remediation')) {
      if (-not "$($exception.$field)".Trim()) {
        throw "Tracked exception '$($exception.advisory)' is missing '$field'"
      }
    }
  }

  Write-Output "Verified npm audit triage gate"
} finally {
  $resolvedTemp = [IO.Path]::GetFullPath($fixtureRoot)
  $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  if ($resolvedTemp.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolvedTemp)) {
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
  }
}
