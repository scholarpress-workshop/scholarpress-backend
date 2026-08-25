param(
    [Parameter(Mandatory = $true)]
    [string]$ProjectPath,
    [string]$BundlePath = "",
    [string]$CatalogPath = "",
    [string]$GooseConfigPath = "",
    [string]$TypstPath = "",
    [string]$PandocPath = "",
    [switch]$StartGoose,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "Usage: .\setup-goose.ps1 -ProjectPath PATH [-BundlePath PATH] [-CatalogPath PATH] [-GooseConfigPath PATH] [-TypstPath PATH] [-PandocPath PATH] [-StartGoose]"
    exit 0
}

function Resolve-ExistingPath([string]$Path, [string]$Label) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Label not found: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $BundlePath) { $BundlePath = $ScriptRoot }
if (-not $GooseConfigPath) { $GooseConfigPath = Join-Path $HOME ".config\goose\config.yaml" }

$ProjectRoot = Resolve-ExistingPath $ProjectPath "project directory"
$BundleRoot = Resolve-ExistingPath $BundlePath "bundle directory"
$ServerPath = Resolve-ExistingPath (Join-Path $BundleRoot "sp-mcp.exe") "sp-mcp.exe"
$DefaultCatalogPath = Join-Path $BundleRoot "catalog"
$CatalogRoot = if ($CatalogPath) {
    Resolve-ExistingPath $CatalogPath "catalog directory"
} else {
    Resolve-ExistingPath $DefaultCatalogPath "bundled catalog directory"
}
$DefaultTypstPath = Join-Path $BundleRoot "bin\typst.exe"
$DefaultPandocPath = Join-Path $BundleRoot "bin\pandoc.exe"
$TypstPath = if ($TypstPath) { Resolve-ExistingPath $TypstPath "Typst executable" } else { Resolve-ExistingPath $DefaultTypstPath "bundled Typst executable" }
$PandocPath = if ($PandocPath) { Resolve-ExistingPath $PandocPath "Pandoc executable" } else { Resolve-ExistingPath $DefaultPandocPath "bundled Pandoc executable" }

$WorkspaceRoot = Join-Path $ProjectRoot ".scholarpress\workspaces"
New-Item -ItemType Directory -Force -Path $WorkspaceRoot | Out-Null

$ConfigParent = Split-Path -Parent $GooseConfigPath
if ($ConfigParent) { New-Item -ItemType Directory -Force -Path $ConfigParent | Out-Null }

$SetupArguments = @(
    "setup-goose",
    "--config", (Resolve-Path -LiteralPath $GooseConfigPath -ErrorAction SilentlyContinue).Path,
    "--command", $ServerPath,
    "--catalog", $CatalogRoot,
    "--workspace-root", (Resolve-Path -LiteralPath $WorkspaceRoot).Path,
    "--typst", $TypstPath,
    "--pandoc", $PandocPath
)

if (-not (Test-Path -LiteralPath $GooseConfigPath)) {
    $SetupArguments[2] = $GooseConfigPath
}

& $ServerPath @SetupArguments
if ($LASTEXITCODE -ne 0) {
    throw "sp-mcp Goose configuration failed with exit code $LASTEXITCODE"
}

Write-Output "Project: $ProjectRoot"
Write-Output "Workspace: $WorkspaceRoot"
Write-Output "Catalog: $CatalogRoot"
Write-Output "Server: $ServerPath"
Write-Output "Typst: $TypstPath"
Write-Output "Pandoc: $PandocPath"
Write-Output "Goose config: $GooseConfigPath"

if ($StartGoose) {
    $Goose = Get-Command goose -ErrorAction SilentlyContinue
    if (-not $Goose) { throw "goose was not found on PATH; omit -StartGoose or install Goose first" }
    & $Goose.Source -WorkingDirectory $ProjectRoot
    exit $LASTEXITCODE
}
