# Regenerates every evidence PNG with --screenshot.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts/screenshots.ps1
# Output: evidence/<shot>.png

$ErrorActionPreference = "Stop"

$Shots = @("estrella", "rocoso", "gaseoso", "lava", "hielo", "toxico", "todos",
           "nubes", "sombras_nubes", "dia_noche", "especular", "bump_rocoso", "bump_luna",
           "vortice", "anillos")

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

cargo build --release --quiet
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$Exe = Join-Path $Root "target/release/shaders_lab.exe"
if (-not (Test-Path $Exe)) { $Exe = Join-Path $Root "target/release/shaders_lab" }

foreach ($Shot in $Shots) {
    & $Exe --screenshot $Shot
    if ($LASTEXITCODE -ne 0) { throw "screenshot failed for $Shot" }
}
