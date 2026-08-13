$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$workflowPath = Join-Path $repoRoot ".github/workflows/release.yml"
$verifyWorkflowPath = Join-Path $repoRoot ".github/workflows/verify.yml"
$securityWorkflowPath = Join-Path $repoRoot ".github/workflows/security-audit.yml"
$nodeVersionPath = Join-Path $repoRoot ".node-version"
$rustToolchainPath = Join-Path $repoRoot "rust-toolchain.toml"
if (-not (Test-Path -LiteralPath $workflowPath)) {
  throw "Release workflow not found: $workflowPath"
}
if (-not (Test-Path -LiteralPath $verifyWorkflowPath)) {
  throw "Verification workflow not found: $verifyWorkflowPath"
}
if (-not (Test-Path -LiteralPath $securityWorkflowPath)) {
  throw "Security workflow not found: $securityWorkflowPath"
}
if (-not (Test-Path -LiteralPath $nodeVersionPath) -or -not (Test-Path -LiteralPath $rustToolchainPath)) {
  throw "Pinned toolchain files are missing"
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
$verifyWorkflowText = [IO.File]::ReadAllText($verifyWorkflowPath)
$securityWorkflowText = [IO.File]::ReadAllText($securityWorkflowPath)
$nodeVersion = [IO.File]::ReadAllText($nodeVersionPath, [Text.Encoding]::UTF8).Trim()
$rustToolchainText = [IO.File]::ReadAllText($rustToolchainPath, [Text.Encoding]::UTF8)
if ($nodeVersion -ne "22.22.2") {
  throw "Pinned Node.js version must be 22.22.2"
}
if ($rustToolchainText -notmatch '(?m)^channel\s*=\s*"1\.96\.0"\s*$') {
  throw "Pinned Rust toolchain must be 1.96.0"
}

foreach ($workflow in @($workflowText, $verifyWorkflowText, $securityWorkflowText)) {
  if ($workflow -match "Setup Node.js" -and $workflow -notmatch "node-version-file: '.node-version'") {
    throw "Node.js workflow setup must read .node-version"
  }
}

foreach ($workflow in @($workflowText, $verifyWorkflowText, $securityWorkflowText)) {
  if ($workflow -match "Setup Rust" -and $workflow -notmatch "toolchain: 1.96.0") {
    throw "Rust workflow setup must pin 1.96.0"
  }
}

if ($verifyWorkflowText -notmatch "toolchain: '1.88.0'") {
  throw "Verification workflow must retain the declared MSRV"
}

if ($verifyWorkflowText -notmatch '& rustc \+1\.88\.0 --version') {
  throw "MSRV declaration check must bypass the repository Rust toolchain override"
}

if ($verifyWorkflowText -notmatch 'cargo \+1\.88\.0 check --locked') {
  throw "MSRV build must bypass the repository Rust toolchain override"
}

if ($securityWorkflowText -notmatch "cargo install cargo-audit --version 0.22.2 --locked") {
  throw "Security workflow must pin cargo-audit 0.22.2"
}

foreach ($workflowPathToCheck in @($workflowPath, $verifyWorkflowPath)) {
  $workflowLines = [IO.File]::ReadAllLines($workflowPathToCheck)
  foreach ($block in @(Get-RunBlock -WorkflowLines $workflowLines)) {
    if ($block.Body -match '(?m)^\s*cargo (test|clippy|check)\b' -and $block.Body -notmatch '--locked') {
      throw "Cargo verification command at $workflowPathToCheck line $($block.Line) must use --locked"
    }
  }
}

foreach ($scriptName in @(
  "scripts/test-verify-updater-signature.ps1",
  "scripts/test-verify-updater-signature-wrapper.ps1",
  "scripts/verify-updater-signature.ps1"
)) {
  if ($workflowText -notmatch [regex]::Escape($scriptName)) {
    throw "Release workflow does not run $scriptName"
  }
}

foreach ($scriptName in @(
  "scripts/test-verify-updater-signature.ps1",
  "scripts/test-verify-updater-signature-wrapper.ps1"
)) {
  if ($verifyWorkflowText -notmatch [regex]::Escape($scriptName)) {
    throw "Verification workflow does not run $scriptName"
  }
}

$collectAssetsIndex = $workflowText.IndexOf("Collect signed bundle assets", [StringComparison]::Ordinal)
$verifySignatureIndex = $workflowText.IndexOf("Verify updater signature", [StringComparison]::Ordinal)
$prepareManifestIndex = $workflowText.IndexOf("Prepare updater manifest", [StringComparison]::Ordinal)
if ($collectAssetsIndex -lt 0 -or $verifySignatureIndex -lt $collectAssetsIndex -or $prepareManifestIndex -lt $verifySignatureIndex) {
  throw "Release workflow does not verify the updater signature before preparing latest.json"
}

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
