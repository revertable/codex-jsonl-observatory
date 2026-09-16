@echo off
setlocal

set "FRONTEND_DIR=%~dp0frontend"
set "APP_EXE=src-tauri\target\release\codex-jsonl-observatory.exe"

if not exist "%FRONTEND_DIR%\package.json" (
  echo [ERROR] The frontend project was not found.
  pause
  exit /b 1
)

where npm.cmd >nul 2>nul
if errorlevel 1 (
  echo [ERROR] npm.cmd was not found in PATH.
  echo Install Node.js and make sure npm is available, then try again.
  pause
  exit /b 1
)

pushd "%FRONTEND_DIR%"
if errorlevel 1 (
  echo [ERROR] Could not enter the frontend directory.
  pause
  exit /b 1
)

echo Building Codex JSONL Observatory...
call npm.cmd --silent run tauri:build -- --no-bundle
if errorlevel 1 goto :build_failed

if not exist "%APP_EXE%" goto :missing_executable

echo Starting Codex JSONL Observatory...
start "" "%APP_EXE%"
if errorlevel 1 goto :launch_failed

popd
exit /b 0

:build_failed
set "RESULT=%ERRORLEVEL%"
echo [ERROR] The application build failed.
popd
pause
exit /b %RESULT%

:missing_executable
echo [ERROR] The build completed, but the application executable was not found.
popd
pause
exit /b 1

:launch_failed
set "RESULT=%ERRORLEVEL%"
echo [ERROR] The application could not be started.
popd
pause
exit /b %RESULT%
