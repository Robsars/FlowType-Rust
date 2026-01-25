# Handoff: FlowType Rust Rewrite

## 🎯 Current Status
The core engine for the "Ultra-Low Latency" dictation app is **successfully implemented and logic-verified**. We have moved from a Python concept to a high-performance Rust pipeline using multi-threading and channels.

### 🏗️ What is Done
1.  **Phase 1: The Ear (Audio & VAD)**
    *   `src/audio/capture.rs`: Handles `cpal` stream capture with automatic normalization of `f32`, `i16`, and `u16` microphone data.
    *   `src/audio/vad.rs`: Implemented energy-based Voice Activity Detection with configurable hysteresis windows (Start/Stop windows) to prevent erratic triggering.
2.  **Phase 2: The Brain (Whisper Integration)**
    *   `src/model.rs`: Automated model manager that downloads the requested GGML model (defaulting to `tiny.en`) from HuggingFace on first run.
    *   `src/transcription/engine.rs`: Multi-threaded Whisper inference context using `whisper-rs`. Processes audio chunks into text within milliseconds.
3.  **Phase 3: The Hand (Windows Transcription Injection)**
    *   `src/injector.rs`: Robust text injection system.
        *   **Strategy 1 (Primary):** Windows UI Automation (UIA) focused element pattern.
        *   **Strategy 2 (Secondary):** UIA ValuePattern for legacy inputs.
        *   **Strategy 3 (Fallback):** Clipboard + `Ctrl+V` simulation using the `arboard` crate.

### 🛠️ Technical Environment Requirements
*   **LLVM:** Must be installed for `whisper-rs`.
*   **Env Var:** `$env:LIBCLANG_PATH="C:\Program Files\LLVM\bin"`
*   **Run Command:** `npm run tauri dev` (Run from the root or `src-tauri` depending on your workflow, but usually root maps to tauri CLI).
    *   *Note:* First time run might take a while to compile `whisper-sys`.

## 🚀 Execution Command
```powershell
# In the root 'FlowType-Rust' folder:
$env:LIBCLANG_PATH="C:\Program Files\LLVM\bin"
npm run tauri dev
```
*(If `npm run tauri` fails, use `cargo tauri dev` or `npx tauri dev`)*

---

## 🎨 UI & Engine Integration
*   **Frontend:** `ui/src/App.tsx` listens for `vad-update` and `transcription`.
*   **Backend:** `src-tauri/src/lib.rs` spawns the engine thread and emits events.
*   **Logic:** The engine runs autonomously, pushing text to the UI and injecting it into the OS.

