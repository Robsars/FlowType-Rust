# 🚀 FlowType Launcher

# Set LLVM Path for Rust bindgen (Whisper)
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
Write-Host "✅ LIBCLANG_PATH set to $env:LIBCLANG_PATH" -ForegroundColor Green

# Check UI dependencies
if (-not (Test-Path "ui\node_modules")) {
    Write-Host "📦 Installing UI dependencies..." -ForegroundColor Cyan
    Push-Location ui
    npm install
    Pop-Location
}

# Run Tauri dev (starts both frontend and backend)
Write-Host "🛠️ Launching Tauri dev environment..." -ForegroundColor Yellow
npx tauri dev
