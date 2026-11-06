# Records the evidence GIFs: renders each shot offline (no window, no screen
# capture) with --record-frames and assembles the frames with ffmpeg.
#
# Usage (from any directory):
#   powershell -ExecutionPolicy Bypass -File scripts/record_gif.ps1
#   powershell -ExecutionPolicy Bypass -File scripts/record_gif.ps1 -Shots vortice,nubes -Width 640
#
# Output: evidence/<shot>.gif
# Requires: cargo and ffmpeg on the PATH.

param(
    [string[]]$Shots = @("estrella", "nubes", "sombras_nubes", "dia_noche", "especular",
                         "bump_rocoso", "bump_luna", "vortice", "lava", "hielo", "toxico", "todos"),
    [int]$Width = 480
)

$ErrorActionPreference = "Stop"

# Must match RECORD_FPS in src/main.rs.
$Fps = 20

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    throw "ffmpeg was not found on the PATH"
}

cargo build --release --quiet
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$Exe = Join-Path $Root "target/release/shaders_lab.exe"
if (-not (Test-Path $Exe)) { $Exe = Join-Path $Root "target/release/shaders_lab" }

foreach ($Shot in $Shots) {
    $Frames = Join-Path $Root "target/gif_frames/$Shot"
    if (Test-Path $Frames) { Remove-Item -Recurse -Force $Frames }

    & $Exe --record-frames $Shot $Frames
    if ($LASTEXITCODE -ne 0) { throw "frame recording failed for $Shot" }

    # One filter graph: build a 256-color palette from all frames, then map
    # every frame to it with light dithering. `-loop 0` repeats forever.
    $Gif = Join-Path $Root "evidence/$Shot.gif"
    $Filter = "scale=${Width}:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=256:stats_mode=full[p];[b][p]paletteuse=dither=bayer:bayer_scale=4"
    ffmpeg -y -loglevel error -framerate $Fps -i (Join-Path $Frames "frame_%04d.png") -vf $Filter -loop 0 $Gif
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed for $Shot" }

    $SizeKb = [math]::Round((Get-Item $Gif).Length / 1KB)
    Write-Host "Saved $Gif ($SizeKb KB)"
}
