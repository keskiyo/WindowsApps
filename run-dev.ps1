$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$logPath = Join-Path $PSScriptRoot 'dev-launcher.log'

Set-Content -LiteralPath $logPath -Value "[$(Get-Date -Format o)] Starting Windows Apps dev" -Encoding UTF8

try {
    foreach ($command in 'node', 'npm', 'cargo') {
        if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
            throw "$command was not found. Install Node.js and Rust before starting the app."
        }
    }

    if (-not (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'node_modules'))) {
        & npm install 2>&1 | Out-File -LiteralPath $logPath -Append -Encoding utf8
        if ($LASTEXITCODE -ne 0) {
            throw "npm install exited with code $LASTEXITCODE"
        }
    }

    & npm run tauri dev 2>&1 | Out-File -LiteralPath $logPath -Append -Encoding utf8
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri dev exited with code $LASTEXITCODE"
    }
} catch {
    Add-Content -LiteralPath $logPath -Value "[$(Get-Date -Format o)] ERROR: $($_.Exception.Message)" -Encoding UTF8
    exit 1
}
