@echo off
rem Relaunch as administrator (PktMon needs it), keep the window open afterwards.
net session >nul 2>&1
if %errorlevel% neq 0 (
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs -WorkingDirectory '%~dp0'"
    exit /b
)
cd /d "%~dp0"
target\release\hsr-sniff.exe
pause
