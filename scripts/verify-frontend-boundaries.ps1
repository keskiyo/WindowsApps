$ErrorActionPreference = "Stop"

$storeImports = & rg -n "lib/tauri" src/store
$storeSearchExit = $LASTEXITCODE
if ($storeSearchExit -gt 1) {
  throw "Could not scan frontend store imports"
}
if ($storeImports) {
  throw "Frontend store depends on the concrete Tauri client:`n$($storeImports -join "`n")"
}

$singleton = & rg -n "export const appStore" src
$singletonSearchExit = $LASTEXITCODE
if ($singletonSearchExit -gt 1) {
  throw "Could not scan frontend singleton stores"
}
if ($singleton) {
  throw "Runtime store must be created by the composition root:`n$($singleton -join "`n")"
}

Write-Output "Verified frontend dependency boundaries"
