$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$workflowPath = Join-Path $repoRoot ".github/workflows/release.yml"
if (-not (Test-Path -LiteralPath $workflowPath)) {
  throw "Release workflow not found: $workflowPath"
}
$lines = [IO.File]::ReadAllLines($workflowPath)
$workflowDirectory = Join-Path $repoRoot ".github/workflows"

# GitHub expands `${{ }}` before the shell ever sees the script, so an expression inside a `run:`
# body is not a variable — it is source code pasted from an attacker-controllable value. Extracting
# the block scalars by indentation lets the check distinguish that from a legitimate expression in
# a declarative `with:`/`env:` field.
function Get-RunBlock {
  param([string[]]$WorkflowLines)

  $blocks = @()
  for ($index = 0; $index -lt $WorkflowLines.Count; $index += 1) {
    $header = [regex]::Match($WorkflowLines[$index], '^(?<indent>\s*)run:\s*(?<inline>.*)$')
    if (-not $header.Success) {
      continue
    }
    $indent = $header.Groups['indent'].Value.Length
    $body = New-Object System.Collections.Generic.List[string]
    $inline = $header.Groups['inline'].Value
    if ($inline -and $inline -notmatch '^[|>][-+]?\s*$') {
      $body.Add($inline)
    }
    for ($cursor = $index + 1; $cursor -lt $WorkflowLines.Count; $cursor += 1) {
      $next = $WorkflowLines[$cursor]
      if ($next.Trim().Length -eq 0) {
        $body.Add($next)
        continue
      }
      $nextIndent = ([regex]::Match($next, '^(\s*)')).Groups[1].Value.Length
      if ($nextIndent -le $indent) {
        break
      }
      $body.Add($next)
    }
    $blocks += [pscustomobject]@{
      Line = $index + 1
      Body = ($body -join "`n")
    }
  }
  return $blocks
}

$runBlocks = @(Get-RunBlock -WorkflowLines $lines)
if ($runBlocks.Count -eq 0) {
  throw "No run blocks found; the workflow parser is broken"
}

# Every workflow, not just the release one: a shell step in any of them expands expressions the
# same way, so the rule has to cover the whole directory or it only holds where someone remembered.
$workflows = @(Get-ChildItem -LiteralPath $workflowDirectory -Filter *.yml -File)
if ($workflows.Count -lt 2) {
  throw "Expected several workflows under .github/workflows; found $($workflows.Count)"
}
foreach ($workflow in $workflows) {
  foreach ($block in @(Get-RunBlock -WorkflowLines ([IO.File]::ReadAllLines($workflow.FullName)))) {
    if ($block.Body -match '\$\{\{') {
      throw "GitHub expression interpolated into shell source at $($workflow.Name) line $($block.Line). Pass the value through env: and read it as `$env:NAME."
    }
  }
}

# Every shell step that names the release tag must read it from the environment.
foreach ($block in $runBlocks) {
  $referencesTag = $block.Body -match 'gh release|gh api|-Tag |TrimStart\(''v''\)'
  if ($referencesTag -and $block.Body -notmatch '\$env:RELEASE_TAG') {
    throw "Tag-dependent shell step at release.yml line $($block.Line) does not read `$env:RELEASE_TAG"
  }
}

$workflowText = [IO.File]::ReadAllText($workflowPath)
foreach ($name in @('RELEASE_TAG', 'REPOSITORY', 'COMMIT_SHA')) {
  $used = $runBlocks | Where-Object { $_.Body -match "\`$env:$name" }
  if (-not $used) {
    continue
  }
  if ($workflowText -notmatch "(?m)^\s*${name}:\s*\`$\{\{") {
    throw "Shell steps read `$env:$name but the workflow never declares $name in an env: block"
  }
}

# The tag reaches the shell as data, so a ref crafted to close a quoted string cannot open a new
# statement. `git check-ref-format` accepts this name, and the trigger is `v*`.
$hostileTag = 'v9.9.9"; Write-Output INJECTED; #'
$previousTag = $env:RELEASE_TAG
try {
  $env:RELEASE_TAG = $hostileTag
  # The exact expression the workflow uses to derive the bundle version from the tag.
  $version = $env:RELEASE_TAG.TrimStart('v')
  $rendered = "Windows Apps_${version}_x64-setup.exe"
  if ($rendered -notlike "*Write-Output INJECTED*") {
    throw "Crafted-tag fixture did not exercise the injection payload"
  }
  $probe = [scriptblock]::Create('$env:RELEASE_TAG.TrimStart(''v'')')
  $output = (& $probe) -join "`n"
  if ($output -match '^INJECTED$') {
    throw "Crafted tag executed as a separate PowerShell statement"
  }
} finally {
  $env:RELEASE_TAG = $previousTag
}

Write-Output "Verified release workflow tag transport"
