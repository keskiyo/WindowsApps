param(
  [Parameter(Mandatory = $true)]
  [string]$Commit,

  [Parameter(Mandatory = $true)]
  [string]$MasterRef
)

$ErrorActionPreference = "Stop"

& git rev-parse --verify "$Commit^{commit}" 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
  throw "Release commit does not exist: $Commit"
}

& git rev-parse --verify "$MasterRef^{commit}" 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
  throw "Release source ref does not exist: $MasterRef"
}

& git merge-base --is-ancestor $Commit $MasterRef
if ($LASTEXITCODE -ne 0) {
  throw "Release commit $Commit is not reachable from $MasterRef"
}

Write-Output "Verified release commit $Commit is reachable from $MasterRef"
