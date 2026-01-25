# 🚀 FlowType (Rust)

**FlowType** is an ultra-low latency, AI-powered dictation application built with Rust and Tauri. It enables seamless, "imperceptible" voice-to-text by combining local Whisper inference with advanced Windows text injection.

---

## ✨ Features

- **⚡ Ultra-Low Latency:** Optimized Rust pipeline targeting <200ms end-to-end delay.
- **🎙️ Advanced Audio Pipeline:** 
  - Energy-based **Voice Activity Detection (VAD)** for automatic capture.
  - **Dynamic Silence Timeout:** Adjustable 300ms to 2500ms timeout via a real-time GUI slider—give yourself more time to think between sentences.
  - **Pre-Roll Buffering (500ms):** Never miss the start of a sentence; FlowType captures the audio *before* the VAD even triggers.
  - **Stereo-to-Mono Downmixing:** Full support for multi-channel array and stereo microphones.
- **🧠 Local Intelligence:** 
  - Uses `whisper.cpp` (via `whisper-rs`) for privacy-first, on-device transcription.
  - **Smart Noise Filtering:** Automatically strips hallucinated non-speech tags like `[BLANK_AUDIO]`, `(upbeat music)`, or `(keyboard clicking)`.
- **⌨️ Universal Injection:** 
  - **Silent Injection:** Uses Windows UI Automation (UIA) to insert text directly into target fields without modifying the clipboard.
  - **Unicode Typing:** Native keyboard simulation using `KEYEVENTF_UNICODE` for robust support in apps that ignore standard accessibility patterns.
  - **Smart Strategy:** Automatically detects focus. Special handling for **VS Code** and **Antigravity** ensures dictation works perfectly in Monaco-based editors.
- **🪟 Premium Overlay:** 
  - Glassmorphic, movable UI built with React.
  - **Minimizable:** Hide the overlay to the taskbar with a click.
  - **Auto-Space:** Optional automatic spacing after each transcribed sentence.
  - **Auto-Start:** Option to automatically launch FlowType with Windows.

---

## 🛠️ Tech Stack

- **Core:** Rust (2021 Edition)
- **GUI:** Tauri v2 + React/TypeScript
- **Audio:** CPAL (Cross-Platform Audio Library)
- **Transcription Engine:** OpenAI Whisper (via `whisper-rs` / `whisper.cpp`)
- **Text Injection:** Native Windows `SendInput` and `UI Automation` via `windows-rs`.

---

## 🚀 Getting Started

### Prerequisites

1.  **Rust:** Install via [rustup.rs](https://rustup.rs/).
2.  **Node.js:** Needed for the Tauri frontend.
3.  **LLVM:** Required for Whisper compilation (bindgen). Ensure `LIBCLANG_PATH` is set on Windows.

### Installation & Launch

We provide a simple launcher to handle the environment setup:

```powershell
# Run the PowerShell launcher
.\start.ps1

# Or the Batch version
.\start.bat
```

The launcher will:
- Set necessary environment variables (`LIBCLANG_PATH`).
- Install frontend dependencies (`npm install`).
- Download the Whisper `tiny.en` model automatically on first run.
- Compile and launch the application in dev mode.

---

## 🏗️ Architecture

```text
src-tauri/
├── src/
│   ├── audio/           # Capture, VAD, Downmixing, and Resampling
│   ├── transcription/   # Whisper Engine logic & Background Threads
│   ├── injector.rs      # Windows UIA, Keyboard, and Clipboard Injection
│   ├── model.rs         # Model management & automated downloading
│   └── lib.rs           # Main Engine loop & Event Orchestration
ui/
└── src/                 # React Frontend (Overlay UI)
```

---

## ⚙️ Settings & Controls

Directly available on the GUI:
- **⏳ Silence Timeout (Slider):** Range from 300ms to 2.5s. Controls how long the app waits for silence before processing your speech.
- **Checkbox - Auto-Space:** When enabled, automatically inserts a space after the transcribed text.
- **Minimizer (_):** Click the dash to minimize the overlay to the taskbar.

---

## 📝 Usage Tips

- **Dragging:** Click and hold anywhere on the FlowType overlay to move it around your screen.
- **Editor Support:** FlowType detects **Antigravity** and **VS Code** automatically. It uses a high-compatibility keyboard-simulation mode to ensure text lands correctly in these editors.
- **Notepad:** Uses silent UIA injection—your clipboard remains completely untouched and clean.

---

## 📄 License

This project is specialized for high-performance Windows environments.
Developed for speed, accuracy, and privacy.
