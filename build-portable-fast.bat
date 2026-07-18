@echo off
setlocal

rem Fast portable release build.
rem cargo build --release already performs full parsing, type checking, compilation,
rem and linking. Passing -SkipCheck avoids compiling the project once with the
rem development profile before compiling it again with the release profile.
rem This saves time and avoids creating unnecessary target\...\debug artifacts.

set "VSDEVCMD=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"

if exist "%VSDEVCMD%" goto vs_found
echo Visual Studio Build Tools developer environment was not found.
exit /b 1

:vs_found

call "%VSDEVCMD%" -arch=x64 -host_arch=x64 >nul
if errorlevel 1 exit /b 1

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-portable-custom.ps1" -SkipCheck %*
exit /b %errorlevel%
