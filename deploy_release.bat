@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0deploy_release.ps1" %*
