param(
  [Parameter(Mandatory = $true)]
  [string]$WhisperBin,

  [Parameter(Mandatory = $true)]
  [string]$WhisperModel,

  [string]$RustTarget = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$desktopDir = Join-Path $repoRoot "packages\desktop"

if (-not (Test-Path $WhisperBin)) {
  throw "Whisper binary not found: $WhisperBin"
}

if (-not (Test-Path $WhisperModel)) {
  throw "Whisper model not found: $WhisperModel"
}

$env:OPENCODE_WHISPER_CPP_BIN = (Resolve-Path $WhisperBin).Path
$env:OPENCODE_WHISPER_CPP_MODEL = (Resolve-Path $WhisperModel).Path
$env:RUST_TARGET = $RustTarget
$env:TAURI_ENV_TARGET_TRIPLE = $RustTarget

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if ((Test-Path $cargoBin) -and (-not $env:PATH.Contains($cargoBin))) {
  $env:PATH = "$cargoBin;$env:PATH"
}

Write-Host "Starting OpenCode desktop with local voice STT..." -ForegroundColor Cyan
Write-Host "OPENCODE_WHISPER_CPP_BIN=$($env:OPENCODE_WHISPER_CPP_BIN)"
Write-Host "OPENCODE_WHISPER_CPP_MODEL=$($env:OPENCODE_WHISPER_CPP_MODEL)"

Push-Location $desktopDir
try {
  & bun run tauri dev
  if ($LASTEXITCODE -ne 0) {
    throw "bun run tauri dev failed with exit code $LASTEXITCODE"
  }
}
finally {
  Pop-Location
}
