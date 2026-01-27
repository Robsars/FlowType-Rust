# 🍎 FlowType MacOS Setup Cheat Sheet

This guide explains how to set up, run, and build **FlowType** on macOS.

## 1. Prerequisites (One-Time Setup)

### Install Rust
Open your Terminal and run:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
*Choose option **1** (default) when prompted.*

### Install Xcode Tools
Required for the compiler:
```bash
xcode-select --install
```

### Install LLVM & Node.js
Recommended using [Homebrew](https://brew.sh/):
```bash
# Install Homebrew if you don't have it
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install llvm node
```

---

## 2. Environment Variables
You must point Rust to your LLVM installation. 

### What is `~/.zshrc`?
It is a "hidden" configuration file in your Home folder that tells the Terminal which tools and paths to use every time you open it.

### How to add these lines:
1. Open your Terminal.
2. Type `nano ~/.zshrc` and hit Enter. (This opens a simple text editor inside the terminal).
3. Scroll to the bottom and paste these lines:
   ```bash
   export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
   export LDFLAGS="-L/opt/homebrew/opt/llvm/lib"
   export CPPFLAGS="-I/opt/homebrew/opt/llvm/include"
   export LIBCLANG_PATH="/opt/homebrew/opt/llvm/lib"
   ```
4. Press **`Control + O`** then **`Enter`** to Save.
5. Press **`Control + X`** to Exit the editor.
6. **Crucial:** Type `source ~/.zshrc` in your terminal to apply the changes immediately, or just close and reopen the terminal.

---

## 3. Running & Building

From the root `FlowType-Rust` folder:

### Development Mode
```bash
npm install
npm run tauri dev
```

### Building the Standalone App (.dmg / .app)
```bash
npm run tauri build
```
Produced files will be in: `src-tauri/target/release/bundle/dmg/`

---

## 4. MacOS Security Permissions
Since this app listens to audio and types for you, it needs special permission:

1.  **Microphone:** You will see a popup asking for access. Click **OK**.
2.  **Accessibility (Typing):**
    *   Go to **System Settings** > **Privacy & Security** > **Accessibility**.
    *   Click the **[+]** button or find "FlowType" in the list.
    *   Toggle it to **ON**.
    *   *If this is not on, the app will run but won't be able to "type" into other windows.*
