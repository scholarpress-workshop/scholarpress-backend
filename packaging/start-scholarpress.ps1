param(
    [string]$OpenWorkWorkspace = (Get-Location).Path,
    [int]$Port = 8765,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: .\start-scholarpress.ps1 -OpenWorkWorkspace PATH [-Port 8765]"
    exit 0
}

$BundleRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ScholarPressRoot = Join-Path $OpenWorkWorkspace ".scholarpress"
$WorkspaceRoot = Join-Path $ScholarPressRoot "workspaces"
$CatalogRoot = Join-Path $ScholarPressRoot "catalog"
$BundledCatalog = Join-Path $BundleRoot "catalog"
$TypstPath = Join-Path $BundleRoot "bin\typst.exe"
$PandocPath = Join-Path $BundleRoot "bin\pandoc.exe"
$ServerPath = Join-Path $BundleRoot "sp-mcp.exe"

if (-not (Test-Path $BundledCatalog)) { throw "catalog directory not found: $BundledCatalog" }
if (-not (Test-Path $ServerPath)) { throw "sp-mcp.exe not found: $ServerPath" }

New-Item -ItemType Directory -Force -Path $WorkspaceRoot, $CatalogRoot | Out-Null
if (-not (Get-ChildItem -Force $CatalogRoot | Select-Object -First 1)) {
    Copy-Item -Recurse -Force (Join-Path $BundledCatalog "*") $CatalogRoot
}

$env:SCHOLARPRESS_CATALOG_PATH = $CatalogRoot
$env:SCHOLARPRESS_WORKSPACE_ROOT = $WorkspaceRoot
$env:SCHOLARPRESS_TYPST_PATH = $TypstPath
$env:SCHOLARPRESS_PANDOC_PATH = $PandocPath

$process = Start-Process -FilePath $ServerPath `
    -ArgumentList @("--transport", "http", "--bind", "127.0.0.1", "--port", $Port) `
    -WorkingDirectory $BundleRoot -NoNewWindow -PassThru

Write-Output "ScholarPress MCP"
Write-Output "Transport: streamable HTTP"
Write-Output "Endpoint: http://127.0.0.1:$Port/mcp"
Write-Output "Catalog: $CatalogRoot"
Write-Output "Typst: $TypstPath"
Write-Output "Pandoc: $PandocPath"

try {
    Wait-Process -Id $process.Id
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}
