param(
    [string]$Output = "dist",
    [string]$SpMcpPath = "target\x86_64-pc-windows-msvc\release\sp-mcp.exe",
    [string]$TypstPath = "",
    [string]$PandocPath = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$Bundle = Join-Path $RepoRoot "dist\scholarpress"
$OutputDir = Join-Path $RepoRoot $Output

if (-not $TypstPath) { $TypstPath = (Get-Command typst).Source }
if (-not $PandocPath) { $PandocPath = (Get-Command pandoc).Source }

Remove-Item -Recurse -Force $Bundle -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path "$Bundle\bin", "$Bundle\catalog" | Out-Null
Copy-Item (Join-Path $RepoRoot $SpMcpPath) "$Bundle\sp-mcp.exe"
Copy-Item $TypstPath "$Bundle\bin\typst.exe"
Copy-Item $PandocPath "$Bundle\bin\pandoc.exe"
Copy-Item -Recurse (Join-Path $RepoRoot "..\scholarpress-catalog\institutions") "$Bundle\catalog\institutions"
Copy-Item "$PSScriptRoot\setup-goose.ps1" $Bundle
Copy-Item "$PSScriptRoot\README-WINDOWS.md" $Bundle

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Compress-Archive -Path $Bundle -DestinationPath (Join-Path $OutputDir "scholarpress-windows-x86_64.zip") -Force
Write-Output (Join-Path $OutputDir "scholarpress-windows-x86_64.zip")
