@echo off
setlocal

set "FRONTEND_DIR=%~dp0frontend"
set "APP_EXE=src-tauri\target\release\codex-jsonl-observatory.exe"
set "OBSERVATORY_RELEASE_EXE=%FRONTEND_DIR%\%APP_EXE%"
set "PACKAGE_SCRIPT=%~dp0frontend\src-tauri\scripts\package-portable.ps1"

if not exist "%FRONTEND_DIR%\package.json" (
  call :write_status ERROR Red "The frontend project was not found."
  pause
  exit /b 1
)

if not exist "%PACKAGE_SCRIPT%" (
  call :write_status ERROR Red "The portable packaging script was not found."
  pause
  exit /b 1
)

where npm.cmd >nul 2>nul
if errorlevel 1 (
  call :write_status ERROR Red "npm.cmd was not found in PATH."
  call :write_status ACTION Yellow "Install Node.js and make sure npm is available, then try again."
  pause
  exit /b 1
)

call :write_status CHECK DarkCyan "Checking for a running Observatory instance..."
powershell.exe -NoProfile -Command "$ErrorActionPreference = 'Stop'; $targetPath = [IO.Path]::GetFullPath($env:OBSERVATORY_RELEASE_EXE); $running = Get-CimInstance Win32_Process -Filter 'Name = ''codex-jsonl-observatory.exe''' | Where-Object { $_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath) -ieq $targetPath }; if ($null -ne $running) { exit 2 }"
set "PROCESS_CHECK_RESULT=%ERRORLEVEL%"
if "%PROCESS_CHECK_RESULT%"=="2" goto :app_running
if not "%PROCESS_CHECK_RESULT%"=="0" goto :process_check_failed

pushd "%FRONTEND_DIR%"
if errorlevel 1 (
  call :write_status ERROR Red "Could not enter the frontend directory."
  pause
  exit /b 1
)

call :write_status BUILD Cyan "Building Codex JSONL Observatory..."
call npm.cmd --silent run tauri:build -- --no-bundle
if errorlevel 1 goto :build_failed

if not exist "%APP_EXE%" goto :missing_executable

call :write_status PACKAGE Cyan "Creating portable archive..."
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%PACKAGE_SCRIPT%" -ExecutablePath "%CD%\%APP_EXE%"
if errorlevel 1 goto :package_failed

call :write_status RUN Cyan "Starting Codex JSONL Observatory..."
start "" "%APP_EXE%"
if errorlevel 1 goto :launch_failed

popd
exit /b 0

:app_running
call :write_status BLOCKED Yellow "Codex JSONL Observatory is currently running."
call :write_status ACTION Yellow "Close the application, then run build-and-run.bat again."
echo Press any key to exit.
pause >nul
exit /b 1

:process_check_failed
call :write_status ERROR Red "Could not determine whether Codex JSONL Observatory is running."
pause
exit /b %PROCESS_CHECK_RESULT%

:build_failed
set "RESULT=%ERRORLEVEL%"
call :write_status ERROR Red "The application build failed."
popd
pause
exit /b %RESULT%

:missing_executable
call :write_status ERROR Red "The build completed, but the application executable was not found."
popd
pause
exit /b 1

:package_failed
set "RESULT=%ERRORLEVEL%"
call :write_status ERROR Red "The portable archive could not be created."
popd
pause
exit /b %RESULT%

:launch_failed
set "RESULT=%ERRORLEVEL%"
call :write_status ERROR Red "The application could not be started."
popd
pause
exit /b %RESULT%

:write_status
powershell.exe -NoProfile -Command "Write-Host '[%~1]' -ForegroundColor %~2 -NoNewline; Write-Host ' %~3'"
exit /b 0
