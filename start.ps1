# 🚀 FlowType Launcher

# Set LLVM Path for Rust bindgen (Whisper)
$llvmPath = "C:\Program Files\LLVM\bin"

if (Test-Path $llvmPath) {
    $env:LIBCLANG_PATH = $llvmPath
    Write-Host "✅ LIBCLANG_PATH set to $env:LIBCLANG_PATH" -ForegroundColor Green
}
else {
    Write-Host "⚠️ WARNING: LLVM not found at $llvmPath" -ForegroundColor Red
    Write-Host "💡 Please ensure LLVM is installed. See README.md for help." -ForegroundColor Yellow
}

# Check root dependencies
if (-not (Test-Path "node_modules")) {
    Write-Host "📦 Installing root dependencies..." -ForegroundColor Cyan
    npm install
}

# Check UI dependencies
if (-not (Test-Path "ui\node_modules")) {
    Write-Host "📦 Installing UI dependencies..." -ForegroundColor Cyan
    npm install --prefix ui
}

# Run Tauri dev (starts both frontend and backend)
Write-Host "🛠️ Launching Tauri dev environment..." -ForegroundColor Yellow
npx tauri dev
