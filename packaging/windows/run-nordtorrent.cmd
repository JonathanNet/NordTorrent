@echo off
setlocal

rem Run NordTorrent from the folder that contains this file.
rem Data and downloads are created next to the executable, so the whole
rem install can live on a USB disk or in C:\NordTorrent.

set "HERE=%~dp0"
set "NORDTORRENT_DATA_DIR=%HERE%data"
set "NORDTORRENT_DOWNLOAD_DIR=%HERE%downloads"
set "NORDTORRENT_LISTEN=0.0.0.0:8080"
set "NORDTORRENT_PUBLIC_URL=http://127.0.0.1:8080"
set "RUST_LOG=nordtorrent=info,tower_http=warn,librqbit=warn"

if not exist "%NORDTORRENT_DATA_DIR%" mkdir "%NORDTORRENT_DATA_DIR%"
if not exist "%NORDTORRENT_DOWNLOAD_DIR%" mkdir "%NORDTORRENT_DOWNLOAD_DIR%"

echo.
echo   NordTorrent is starting.
echo   Web panel: http://127.0.0.1:8080
echo   Data:      %NORDTORRENT_DATA_DIR%
echo   Downloads: %NORDTORRENT_DOWNLOAD_DIR%
echo.
echo   Close this window or press Ctrl+C to stop the server.
echo.

start "" "http://127.0.0.1:8080"
"%~dp0nordtorrent.exe"

endlocal
