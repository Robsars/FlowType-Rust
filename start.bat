@echo off
setlocal

echo 🚀 Starting FlowType Environment...

:: Set LLVM path for Whisper compilation
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
echo ✅ LIBCLANG_PATH set to %LIBCLANG_PATH%

:: Install UI dependencies if needed
if not exist "ui\node_modules" (
    echo 📦 Installing UI dependencies...
    cd ui && npm install && cd ..
)

:: Run Tauri dev (handles frontend and backend)
echo 🛠️ Launching Tauri (this may take a minute on first run)...
npx tauri dev

pause
