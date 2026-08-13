param(
  [Parameter(Mandatory = $true)]
  [int]$ProcessId,

  [Parameter(Mandatory = $true)]
  [string]$ProcessPath,

  [ValidateRange(1, 1000)]
  [int]$RefreshCancelCycles = 100,

  [ValidateRange(0, 240)]
  [int]$IdleMinutes = 120,

  [ValidateRange(1, 60)]
  [int]$SampleIntervalSeconds = 30,

  [Nullable[double]]$SearchP95Ms,

  [Nullable[double]]$CachedCatalogVisibleMs,

  [Nullable[double]]$CancelAcknowledgementMs
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$evidenceDirectory = Join-Path $repoRoot ".1localDocuments"
$expectedProcessPath = [IO.Path]::GetFullPath($ProcessPath)
$logsDirectory = Join-Path $env:LOCALAPPDATA "dev.neiroslop.windowsapps\logs"

function Get-TargetProcess {
  try {
    $target = Get-Process -Id $ProcessId -ErrorAction Stop
    $actualProcessPath = [IO.Path]::GetFullPath($target.Path)
    if (-not $actualProcessPath.Equals($expectedProcessPath, [StringComparison]::OrdinalIgnoreCase)) {
      throw "Unexpected process path"
    }
    return $target
  } catch {
    throw "Release soak target unavailable"
  }
}

function Get-DescendantWorkingSet {
  param([int]$RootProcessId)

  $seen = [Collections.Generic.HashSet[int]]::new()
  [void]$seen.Add($RootProcessId)
  $pending = [Collections.Generic.Queue[int]]::new()
  $pending.Enqueue($RootProcessId)
  $totalBytes = [long]0
  $privateBytes = [long]0
  $processCount = 0

  while ($pending.Count -gt 0) {
    $parentProcessId = $pending.Dequeue()
    try {
      $children = @(Get-CimInstance -ClassName Win32_Process -Filter "ParentProcessId = $parentProcessId" -ErrorAction Stop)
    } catch {
      $children = @()
    }
    foreach ($child in $children) {
      $childProcessId = [int]$child.ProcessId
      if (-not $seen.Add($childProcessId)) {
        continue
      }
      $pending.Enqueue($childProcessId)
      $processCount += 1
      if ($null -ne $child.WorkingSetSize) {
        $totalBytes += [long]$child.WorkingSetSize
      }
      if ($null -ne $child.PrivatePageCount) {
        $privateBytes += [long]$child.PrivatePageCount
      }
    }
  }

  return [ordered]@{
    bytes = $totalBytes
    privateBytes = $privateBytes
    count = $processCount
  }
}

function Get-MemorySample {
  $target = Get-TargetProcess
  $descendants = Get-DescendantWorkingSet -RootProcessId $target.Id
  return [ordered]@{
    observedAt = [DateTime]::UtcNow.ToString("o")
    workingSetBytes = $target.WorkingSet64
    privateMemoryBytes = $target.PrivateMemorySize64
    virtualMemoryBytes = $target.VirtualMemorySize64
    childWorkingSetBytes = $descendants.bytes
    childPrivateMemoryBytes = $descendants.privateBytes
    childProcessCount = $descendants.count
    totalWorkingSetBytes = [long]$target.WorkingSet64 + $descendants.bytes
    totalPrivateMemoryBytes = [long]$target.PrivateMemorySize64 + $descendants.privateBytes
  }
}

function Get-LogFileEvidence {
  if (-not (Test-Path -LiteralPath $logsDirectory -PathType Container)) {
    return @()
  }

  return @(Get-ChildItem -LiteralPath $logsDirectory -File -ErrorAction Stop | ForEach-Object {
    [ordered]@{
      path = $_.FullName
      sizeBytes = $_.Length
      lastWriteAt = $_.LastWriteTimeUtc.ToString("o")
    }
  })
}

function Read-CycleOutcome {
  param([int]$Cycle)

  while ($true) {
    $outcome = (Read-Host "Cycle $Cycle/${RefreshCancelCycles}: complete Refresh then Cancel; enter success, cancel or error").Trim().ToLowerInvariant()
    if ($outcome -in @("success", "cancel", "error")) {
      return $outcome
    }
    Write-Output "Enter success, cancel or error"
  }
}

$target = Get-TargetProcess
$samples = [Collections.Generic.List[object]]::new()
$samples.Add((Get-MemorySample))
$outcomes = [ordered]@{ success = 0; cancel = 0; error = 0 }
$startedAt = [DateTime]::UtcNow

for ($cycle = 1; $cycle -le $RefreshCancelCycles; $cycle += 1) {
  $outcome = Read-CycleOutcome -Cycle $cycle
  $outcomes[$outcome] += 1
  $samples.Add((Get-MemorySample))
}

$idleUntil = [DateTime]::UtcNow.AddMinutes($IdleMinutes)
while ([DateTime]::UtcNow -lt $idleUntil) {
  $remainingSeconds = [Math]::Ceiling(($idleUntil - [DateTime]::UtcNow).TotalSeconds)
  Start-Sleep -Seconds ([Math]::Min($SampleIntervalSeconds, $remainingSeconds))
  $samples.Add((Get-MemorySample))
}

$samples.Add((Get-MemorySample))
$initialWorkingSet = $samples[0].workingSetBytes
$finalWorkingSet = $samples[$samples.Count - 1].workingSetBytes
$workingSetGrowthPercent = if ($initialWorkingSet -eq 0) { $null } else { (($finalWorkingSet - $initialWorkingSet) / $initialWorkingSet) * 100 }
$initialTotalWorkingSet = $samples[0].totalWorkingSetBytes
$finalTotalWorkingSet = $samples[$samples.Count - 1].totalWorkingSetBytes
$totalWorkingSetGrowthPercent = if ($initialTotalWorkingSet -eq 0) { $null } else { (($finalTotalWorkingSet - $initialTotalWorkingSet) / $initialTotalWorkingSet) * 100 }
$record = [ordered]@{
  sourceCommit = (& git -C $repoRoot rev-parse HEAD).Trim()
  process = [ordered]@{
    id = $target.Id
    path = $expectedProcessPath
  }
  startedAt = $startedAt.ToString("o")
  finishedAt = [DateTime]::UtcNow.ToString("o")
  refreshCancelCycles = $RefreshCancelCycles
  outcomes = $outcomes
  idleMinutes = $IdleMinutes
  sampleIntervalSeconds = $SampleIntervalSeconds
  measurements = [ordered]@{
    searchP95Ms = $SearchP95Ms
    cachedCatalogVisibleMs = $CachedCatalogVisibleMs
    cancelAcknowledgementMs = $CancelAcknowledgementMs
    workingSetGrowthPercent = $workingSetGrowthPercent
    totalWorkingSetGrowthPercent = $totalWorkingSetGrowthPercent
  }
  memorySamples = $samples
  logs = [ordered]@{
    directory = $logsDirectory
    files = Get-LogFileEvidence
  }
}

[IO.Directory]::CreateDirectory($evidenceDirectory) | Out-Null
$outputPath = Join-Path $evidenceDirectory "release-soak-$($startedAt.ToString('yyyyMMdd-HHmmss')).json"
[IO.File]::WriteAllText($outputPath, ($record | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
Write-Output "Release soak evidence: $outputPath"
