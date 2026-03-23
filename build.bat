@echo off
:: bruecke build script for Windows
setlocal enabledelayedexpansion

cd /d "%~dp0"

echo.
echo   bruecke build
echo   -----------------------------------------
echo   Detected OS: Windows
echo.
echo   Build targets:
echo     [1] Native only (Windows)
echo     [2] Native + Linux x86_64
echo     [3] All platforms (Windows + Linux)
echo.
set /p CHOICE="  Choose [1-3]: "
echo.

set BUILD_LINUX=false
if "%CHOICE%"=="2" set BUILD_LINUX=true
if "%CHOICE%"=="3" set BUILD_LINUX=true

:: check cross if needed
if "%BUILD_LINUX%"=="true" (
    where cross >nul 2>&1
    if errorlevel 1 (
        echo   'cross' not found — install it first:
        echo     cargo install cross
        echo.
        echo   Continuing with native only.
        set BUILD_LINUX=false
    )
)

:: build WASM
echo ^> 1/3  Building WASM...
wasm-pack build --target web --release --quiet
if errorlevel 1 ( echo ERROR: wasm-pack failed & exit /b 1 )

:: build server
echo ^> 2/3  Building server (native)...
cargo build --bin server --release --quiet
if errorlevel 1 ( echo ERROR: cargo build failed & exit /b 1 )

:: assemble dist/
echo ^> 3/3  Assembling dist/...
if not exist dist mkdir dist
copy /y pkg\bruecke_bg.wasm dist\bruecke_bg.wasm >nul
copy /y app.py dist\app.py >nul
copy /y target\release\server.exe dist\server.exe >nul
echo     OK server.exe (Windows)

:: cross-compile Linux
if "%BUILD_LINUX%"=="true" (
    echo     cross -^> Linux x86_64...
    cross build --bin server --release --target x86_64-unknown-linux-musl --quiet
    if errorlevel 1 ( echo ERROR: cross build Linux failed & exit /b 1 )
    copy /y target\x86_64-unknown-linux-musl\release\server dist\server-linux-x86_64 >nul
    echo     OK server-linux-x86_64
)

:: README
(
echo bruecke
echo =======
echo.
echo Start the server for your platform:
echo   Windows:         server.exe
echo   Linux ^(x86_64^):  ./server-linux-x86_64
echo.
echo Then open: http://127.0.0.1:7777
echo.
echo Edit app.py — browser updates instantly on save.
) > dist\README.txt

echo.
echo OK dist/ ready:
dir /b dist
echo.
echo   Run:  cd dist ^&^& server.exe
echo   Open: http://127.0.0.1:7777
echo.
