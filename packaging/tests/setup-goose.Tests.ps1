param(
    [string]$ServerPath = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$SetupScript = Join-Path $RepoRoot "packaging\setup-goose.ps1"
if (-not $ServerPath) {
    $ServerPath = Join-Path $RepoRoot "target\x86_64-pc-windows-msvc\release\sp-mcp.exe"
}

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "ASSERTION FAILED: $Message" }
}

$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "sp-goose-test-$([guid]::NewGuid())"
$Bundle = Join-Path $TempRoot "bundle"
$Project = Join-Path $TempRoot "project"
$Config = Join-Path $TempRoot "goose\config.yaml"
try {
    New-Item -ItemType Directory -Force -Path "$Bundle\bin", "$Bundle\catalog\institutions", $Project | Out-Null
    Copy-Item $ServerPath "$Bundle\sp-mcp.exe"
    Set-Content -LiteralPath "$Bundle\bin\typst.exe" -Value "stub"
    Set-Content -LiteralPath "$Bundle\bin\pandoc.exe" -Value "stub"
    Set-Content -LiteralPath $Config -Value "extensions:`n  other:`n    cmd: other-tool`n    enabled: true`n"

    & powershell -NoProfile -ExecutionPolicy Bypass -File $SetupScript `
        -ProjectPath $Project -BundlePath $Bundle -GooseConfigPath $Config
    Assert-True (Test-Path "$Project\.scholarpress\workspaces") "workspace root was not created"
    $text = Get-Content -LiteralPath $Config -Raw
    Assert-True ($text.Contains("scholarpress:")) "ScholarPress extension was not written"
    Assert-True ($text.Contains("other-tool")) "unrelated extension was not preserved"
    Assert-True ((Get-ChildItem (Split-Path $Config) -Filter "config.yaml.bak-*" | Measure-Object).Count -eq 1) "config backup was not written"
    Write-Output "setup-goose smoke test passed"
} finally {
    Remove-Item -Recurse -Force $TempRoot -ErrorAction SilentlyContinue
}
