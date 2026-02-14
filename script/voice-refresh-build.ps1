param(
  [string]$Branch = "voice-input-local-whisper",
  [string]$BaseRemote = "upstream",
  [string]$BaseBranch = "dev",
  [switch]$SkipInstall,
  [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

function Run-Step {
  param(
    [string]$Title,
    [string]$Command,
    [string]$WorkingDirectory = $null
  )

  Write-Host ""
  Write-Host "> $Title" -ForegroundColor Cyan

  $oldDir = Get-Location
  if ($WorkingDirectory) {
    Set-Location $WorkingDirectory
  }

  try {
    & cmd /c $Command
    if ($LASTEXITCODE -ne 0) {
      throw "Command failed with exit code ${LASTEXITCODE}: $Command"
    }
  }
  finally {
    if ($WorkingDirectory) {
      Set-Location $oldDir
    }
  }
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $repoRoot

Run-Step -Title "Fetching latest refs" -Command "git fetch --all --prune"
Run-Step -Title "Switching to $Branch" -Command "git checkout $Branch"
Run-Step -Title "Rebasing onto $BaseRemote/$BaseBranch" -Command "git rebase $BaseRemote/$BaseBranch"

if (-not $SkipInstall) {
  Run-Step -Title "Installing workspace deps" -Command "bun install"
}

if (-not $SkipBuild) {
  Run-Step -Title "Building app package" -Command "bun run build" -WorkingDirectory (Join-Path $repoRoot "packages\app")
  Run-Step -Title "Checking tauri rust package" -Command "cargo check" -WorkingDirectory (Join-Path $repoRoot "packages\desktop\src-tauri")
}

Write-Host ""
Write-Host "Voice branch refresh complete." -ForegroundColor Green
Write-Host "Next: set OPENCODE_WHISPER_CPP_BIN and OPENCODE_WHISPER_CPP_MODEL, then run script/voice-dev.ps1" -ForegroundColor Green
