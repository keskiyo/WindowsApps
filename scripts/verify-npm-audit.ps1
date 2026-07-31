<#
Triage gate for `npm audit`, strictly stronger than a bare `npm audit --audit-level=high`.

A bare audit can only be green or red. Red forever is not triage: the advisory stops being read
and a genuinely new one hides behind it. This gate fails on any high/critical advisory that is not
individually documented in .github/npm-audit-exceptions.json, and it also fails when an exception
goes stale — its review date passed, or the advisory it covers is gone and the entry was not
removed. Runtime dependencies are gated separately and accept no exceptions.
#>
param(
  # Pre-captured `npm audit --json` output. Omit to run npm audit here.
  [string]$AuditJsonPath,
  [string]$ExceptionsPath,
  # Overridable so the gate's own date handling is testable without waiting for a calendar day.
  [string]$Today
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $ExceptionsPath) {
  $ExceptionsPath = Join-Path $repoRoot ".github/npm-audit-exceptions.json"
}
$reviewDate = if ($Today) { [datetime]::ParseExact($Today, 'yyyy-MM-dd', $null) } else { (Get-Date).Date }

if ($AuditJsonPath) {
  $auditText = [IO.File]::ReadAllText($AuditJsonPath)
} else {
  # npm audit exits non-zero when it finds anything, which is not an error for this gate.
  $previous = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  $auditText = (& npm audit --json 2>$null) -join "`n"
  $ErrorActionPreference = $previous
}
if (-not $auditText.Trim()) {
  throw "npm audit produced no output"
}
$audit = $auditText | ConvertFrom-Json

# `via` holds either a dependency name (an indirect entry that repeats a downstream advisory) or
# the advisory object itself. Only the objects carry a GHSA identifier, so only they are reported.
$reported = @{}
if ($audit.PSObject.Properties.Name -contains 'vulnerabilities' -and $audit.vulnerabilities) {
  foreach ($entry in $audit.vulnerabilities.PSObject.Properties) {
    foreach ($via in @($entry.Value.via)) {
      if ($via -is [string]) {
        continue
      }
      if ($via.severity -notin @('high', 'critical')) {
        continue
      }
      $advisory = if ("$($via.url)" -match '(GHSA-[0-9a-z-]+)') { $Matches[1] } else { "source-$($via.source)" }
      $reported[$advisory] = [pscustomobject]@{
        Package  = $via.name
        Severity = $via.severity
        Title    = $via.title
      }
    }
  }
}

$exceptions = @()
if (Test-Path -LiteralPath $ExceptionsPath) {
  $exceptions = @(([IO.File]::ReadAllText($ExceptionsPath) | ConvertFrom-Json).exceptions)
}

$failures = New-Object System.Collections.Generic.List[string]
$allowed = @{}

foreach ($exception in $exceptions) {
  foreach ($field in @('advisory', 'package', 'chain', 'reason', 'owner', 'reviewBy', 'remediation')) {
    if (-not "$($exception.$field)".Trim()) {
      $failures.Add("Exception '$($exception.advisory)' is missing a non-empty '$field'")
    }
  }
  $allowed[$exception.advisory] = $true

  $reviewBy = [datetime]::MinValue
  if (-not [datetime]::TryParseExact(
      "$($exception.reviewBy)",
      'yyyy-MM-dd',
      [Globalization.CultureInfo]::InvariantCulture,
      [Globalization.DateTimeStyles]::None,
      [ref]$reviewBy)) {
    $failures.Add("Exception '$($exception.advisory)' has an unparsable reviewBy '$($exception.reviewBy)' (expected yyyy-MM-dd)")
    continue
  }
  if ($reviewBy -lt $reviewDate) {
    $failures.Add("Exception '$($exception.advisory)' expired on $($exception.reviewBy); re-triage it or remove it")
  }
  if (-not $reported.ContainsKey($exception.advisory)) {
    $failures.Add("Exception '$($exception.advisory)' no longer matches any reported advisory; remove it")
  }
}

foreach ($advisory in $reported.Keys) {
  if (-not $allowed.ContainsKey($advisory)) {
    $entry = $reported[$advisory]
    $failures.Add("Undocumented $($entry.Severity) advisory $advisory in $($entry.Package): $($entry.Title)")
  }
}

if ($failures.Count -gt 0) {
  throw "npm audit triage failed:`n - $($failures -join "`n - ")"
}

if ($reported.Count -eq 0) {
  Write-Output "Verified npm audit: no high or critical advisories"
} else {
  Write-Output "Verified npm audit: $($reported.Count) high/critical advisory/advisories, all documented in $ExceptionsPath"
}
