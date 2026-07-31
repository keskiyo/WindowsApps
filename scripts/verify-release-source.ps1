param(
  [Parameter(Mandatory = $true)]
  [string]$Commit,

  [Parameter(Mandatory = $true)]
  [string]$MasterRef
)

$ErrorActionPreference = "Stop"

$commitSha = (& git rev-parse --verify "$Commit^{commit}" 2>&1 | Select-Object -Last 1)
if ($LASTEXITCODE -ne 0) {
  throw "Release commit does not exist: $Commit"
}

$masterSha = (& git rev-parse --verify "$MasterRef^{commit}" 2>&1 | Select-Object -Last 1)
if ($LASTEXITCODE -ne 0) {
  throw "Release source ref does not exist: $MasterRef"
}

# Exact equality, not ancestry. A tag on a superseded master commit is still reachable from
# `master`, so `merge-base --is-ancestor` signed and published it after master had moved on —
# breaking the "releases come from current master" promise and the rollback audit trail.
if ("$commitSha".Trim() -ne "$masterSha".Trim()) {
  throw "Release commit $Commit is not the current $MasterRef commit"
}

Write-Output "Verified release commit $Commit is the current $MasterRef commit"
