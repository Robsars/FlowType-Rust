#!/bin/bash

# 🍎 FlowType MacOS Launcher

echo "🚀 Starting FlowType Environment (MacOS)..."

# 1. Set LLVM path for Whisper compilation (Homebrew default)
if [ -d "/opt/homebrew/opt/llvm/bin" ]; then
    export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
    export LDFLAGS="-L/opt/homebrew/opt/llvm/lib"
    export CPPFLAGS="-I/opt/homebrew/opt/llvm/include"
    export LIBCLANG_PATH="/opt/homebrew/opt/llvm/lib"
    echo "✅ LLVM paths set via Homebrew."
else
    echo "⚠️ Warning: LLVM not found in /opt/homebrew/opt/llvm. Compilation might fail."
    echo "💡 Try running: brew install llvm"
fi

# 2. Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "📦 Installing project dependencies..."
    npm install
fi

if [ ! -d "ui/node_modules" ]; then
    echo "📦 Installing UI dependencies..."
    npm install --prefix ui
fi

# 3. Run Tauri dev
echo "🛠️ Launching Tauri dev environment..."
npm run dev
