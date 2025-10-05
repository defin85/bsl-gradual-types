@echo off
REM Wrapper для LSP сервера с логированием
set LOG_FILE=%TEMP%\bsl_lsp_server.log

echo [%date% %time%] LSP Server wrapper started >> "%LOG_FILE%"
echo [%date% %time%] Environment: >> "%LOG_FILE%"
echo RUST_LOG=%RUST_LOG% >> "%LOG_FILE%"
echo RUST_BACKTRACE=%RUST_BACKTRACE% >> "%LOG_FILE%"
echo Working directory: %CD% >> "%LOG_FILE%"
echo. >> "%LOG_FILE%"

echo [%date% %time%] Starting lsp_server.exe... >> "%LOG_FILE%"
"%~dp0lsp_server.exe" 2>>"%LOG_FILE%"

echo [%date% %time%] LSP Server exited with code: %ERRORLEVEL% >> "%LOG_FILE%"
