@echo off
REM EchoAsm generator (cmd twin): copy locked input bytes to candidate output.
REM Usage: echoasm.cmd <input> <output>
if "%~1"=="" goto usage
if "%~2"=="" goto usage
if not exist "%~1" (
  echo echoasm: input not found: %~1 1>&2
  exit /b 1
)
for %%I in ("%~2") do if not exist "%%~dpI" mkdir "%%~dpI"
copy /Y "%~1" "%~2" >nul
if errorlevel 1 (
  echo echoasm: copy failed 1>&2
  exit /b 1
)
echo echoasm: wrote %~2
exit /b 0

:usage
echo usage: echoasm.cmd ^<input^> ^<output^> 1>&2
exit /b 2
