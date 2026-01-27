@echo off
setlocal

echo 🚀 Starting FlowType Environment...

:: Set LLVM path for Whisper compilation
set LIBCLANG_PATH=C:\Program Files\LLVM\bin

if exist "%LIBCLANG_PATH%" (
    echo ✅ LIBCLANG_PATH set to %LIBCLANG_PATH%
) else (
    echo ⚠️ WARNING: LLVM not found at %LIBCLANG_PATH%
    echo 💡 Please ensure LLVM is installed. See README.md for help.
)

:: Install root dependencies if needed
if not exist "node_modules" (
    echo 📦 Installing root dependencies...
    npm install
)

:: Install UI dependencies if needed
if not exist "ui\node_modules" (
    echo 📦 Installing UI dependencies...
    npm install --prefix ui
)

:: Run Tauri dev (handles frontend and backend)
echo 🛠️ Launching Tauri (this may take a minute on first run)...
npx tauri dev

pause
