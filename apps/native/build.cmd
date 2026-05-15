@echo off
cd /d %~dp0
bun run build %*
