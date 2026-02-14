@echo off
setlocal

set REPO=C:\projects\a.space\opencode-voice-plugin\opencode
set WHISPER_BIN=C:\Tools\whisper.cpp\whisper.cpp\build\bin\Release\whisper-cli.exe
set WHISPER_MODEL=C:\Tools\whisper.cpp\whisper.cpp\models\ggml-base.en.bin

cd /d "%REPO%"
if errorlevel 1 (
  echo Failed to change directory to %REPO%
  exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -File "script\voice-refresh-build.ps1" -SkipInstall
if errorlevel 1 exit /b 1

powershell -NoProfile -ExecutionPolicy Bypass -File "script\voice-dev.ps1" -WhisperBin "%WHISPER_BIN%" -WhisperModel "%WHISPER_MODEL%"
exit /b %errorlevel%
