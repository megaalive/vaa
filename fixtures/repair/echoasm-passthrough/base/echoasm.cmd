@echo off
REM BROKEN echoasm ??? prepends a hostile marker (repair demo base).
if "%~1"=="" exit /b 2
if "%~2"=="" exit /b 2
echo ; BROKEN > "%~2"
type "%~1" >> "%~2"
exit /b 0
