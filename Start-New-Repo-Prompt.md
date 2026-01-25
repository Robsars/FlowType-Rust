# 🚀 Project FlowType: Rust Rewrite Specification (Ultra-Low Latency)

**Role:** You are a Senior Systems Engineer expert in Rust, Windows Internals, and Audio Processing.
**Objective:** Rewrite an existing Python dictation app ("FlowType") into **Rust** to achieve imperceptible latency (target < 200ms end-to-end).

## 1. Technology Stack
*   **Core:** Rust (2021 edition or newer).
*   **GUI:** [Tauri](https://tauri.app/) (v2 preferred) with React/TypeScript frontend.
    *   *Reason:* Lightweight, uses native WebView, allows creating a "transparent overlay" window easily.
*   **Audio Capture:** `cpal` (Cross-platform audio library).
*   **Transcription:** `whisper-rs` (Bindings for `whisper.cpp`).
    *   *Model:* Support `distil-whisper-small` or `distil-whisper-medium` for speed.
    *   *Feature:* usage of `stream` or `stateful` decoding if possible, otherwise fast chunk decoding.
*   **Windows Automation:** `windows` crate (Microsoft's official bindings).
    *   *Critical:* Must use **UI Automation (UIA)** patterns, not just virtual keys.
*   **Global Hotkeys:** `global-hotkey` crate.
*   **State Management:** `once_cell` or `std::sync` primitives (Arc, Mutex, atomic RwLock).

## 2. Core Architecture: The "Pipeline"
The app must run as a pipeline of independent threads connected by high-performance channels (`crossbeam-channel` or `mpsc`).

### A. Audio Capture Thread (High Priority)
1.  **Input:** `cpal` stream (Wasapi on Windows).
2.  **Preprocessing:**
    *   **Resample:** Convert input to 16kHz mono (mandatory for Whisper).
    *   **High-Pass Filter:** Apply a simple IIR high-pass at ~80-100Hz to remove mic rumble.
    *   **Noise Gate:** Silence signals below a configurable RMS threshold (e.g., 0.015).
3.  **VAD (Voice Activity Detection):**
    *   Implement an energy-based VAD (or use `webrtc-vad` crate if available/stable).
    *   **Logic:**
        *   maintain a ring buffer of ~1-3 seconds.
        *   Trigger "Start" when energy > Threshold for `StartWindow` (e.g., 200ms).
        *   Trigger "Stop" when energy < Threshold for `StopWindow` (e.g., 500ms).
4.  **Partial Flushing (Critical for Speed):**
    *   Do **NOT** wait for the sentence to finish.
    *   Every `PartialFlushInterval` (e.g., 300ms) of active speech, copy the current buffer + context and send it to the Transcription Thread.
    *   Mark this chunk as `is_partial: true`.

### B. Transcription Thread (GPU/AVX accelerated)
1.  **Input:** Audio chunks (f32 vectors).
2.  **Engine:** `whisper-rs`.
    *   Load quantized model (`ggml-base.en.bin` or `distil-medium.en`) into memory at startup.
    *   Use **CoreML** (on macOS) or **OpenVINO/CUDA/Vulkan** (on Windows) if enabled in `whisper.cpp` build, otherwise AVX2 CPU is surprisingly fast for `distil` models.
3.  **Logic:**
    *   Receive chunk.
    *   Run inference.
    *   **Diffing:** If `is_partial`, compare the new text with the previously committed text. Only emit the *new* stable characters.
    *   Send `TextEvent` to Injection Thread.

### C. Injection Thread (Windows UI Automation)
*This is the unique selling point. Do not use generic `send_keys`.*

1.  **Input:** Text strings.
2.  **Strategy:**
    *   **Attempt 1: UIA TextPattern.**
        *   Get `UIAutomation` COM object.
        *   Get `FocusedElement`.
        *   Try `GetCurrentPattern(TextPattern)`.
        *   Get `Selection` (TextRange).
        *   Call `InsertText(text)` on the range.
        *   *Benefit:* Text appears instantly, no caret flashing, no interference with user typing.
    *   **Attempt 2: UIA ValuePattern.**
        *   If TextPattern fails, try `ValuePattern`.
        *   Read `CurrentValue`, append text, call `SetValue`.
    *   **Attempt 3: Clipboard Fallback.**
        *   Save current clipboard.
        *   Set clipboard to text.
        *   Send `Ctrl+V`.
        *   Restore clipboard (async).
3.  **Formatting:**
    *   **Space-on-Interval:** If time since last injection > `PauseThreshold`, prepend a space.
    *   **Capitalization:** Auto-capitalize first letter if previous ended in punctuation.

### D. GUI (Tauri)
1.  **Dashboard Window:** Settings (Model selection, VAD sensitivity, Hotkeys), Logs.
2.  **Overlay Window:**
    *   Small, transparent, click-through (optional), always-on-top.
    *   Visualizes microphone energy (Green/Red bar).
    *   Shows "Listening..." status.

## 3. Implementation Plan (Step-by-Step)

### Phase 1: The "Ear" (Audio & VAD)
*   Initialize `cpal` input stream.
*   Implement `RingBuffer` for audio history.
*   Implement `EnergyVad` struct.
*   **Result:** Console app that prints "Speaking..." / "Silence..." accurately.

### Phase 2: The "Brain" (Whisper)
*   Integrate `whisper-rs`.
*   Download a model (e.g., `ggml-tiny.en.bin`) automatically if missing.
*   Feed audio ring buffer to Whisper; print text to stdout.
*   **Result:** Console app that dictates to terminal.

### Phase 3: The "Hand" (Windows Injection)
*   Use `windows` crate features: `Win32_UI_Accessibility`, `Win32_System_Com`.
*   Implement the `TextInjector` struct with the 3 fallback strategies.
*   **Result:** App dictates into Notepad/Word.

### Phase 4: The Body (Tauri GUI)
*   Scaffold Tauri app.
*   Build React frontend for the dashboard.
*   Connect Rust events (`emit_all`) to frontend for VU meter updates.

## 4. Key Configuration Consts (Tunable)
```rust
const SAMPLE_RATE: u32 = 16000;
const PARTIAL_FLUSH_MS: u64 = 250;     // Lower = faster feedback, higher = more context
const SILENCE_TIMEOUT_MS: u64 = 600;   // Wait this long before committing a sentence
const SPEECH_THRESHOLD: f32 = 0.015;   // RMS Energy threshold
const HPF_FREQ: f32 = 90.0;            // High-pass filter Hz
```

## 5. Definition of Done
1.  User presses Global Hotkey.
2.  Overlay appears/turns green.
3.  User speaks.
4.  Text appears in the active window (e.g. VS Code) within **200ms** of speaking.
5.  User stops speaking.
6.  App inserts final punctuation (if enabled) and spaces correctly for next phrase.
