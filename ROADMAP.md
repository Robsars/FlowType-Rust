# 🗺️ Roadmap: FlowType Rust Rewrite

## ✅ Completed (The Core)
- [x] Foundation: Rust 2021 + Dependency Audit.
- [x] Audio Pipeline: CPAL + RingBuf integration.
- [x] VAD: High-performance energy evaluation.
- [x] Brain: Whisper inference thread + Model downloading.
- [x] Hand: Windows UIA & SendInput fallback.

## 🏃 Immediate Next Steps
- [x] **Fix Build Lock:** Resolved via `cargo clean` and migration.
- [x] **Phase 4: The Body (Tauri GUI Integration)**
    - [x] Scaffold Tauri v2 + React/Vite.
    - [x] Port Rust Engine to `src-tauri`.
    - [x] Connect Events (VAD/Transcription) to UI.
- [ ] **Phase 5: Performance Tuning**
    - Implement `distil-whisper` support for even lower latency.
    - Add "Preroll" buffering (capturing the 100ms *before* VAD triggers speaking).

## 🌟 Future Features
- [ ] **UIA TextPattern Deep Integration:** Direct character insertion without selection replacement.
- [ ] **Global Hotkeys:** `Alt+D` to toggle manual dictation mode.
- [ ] **Formatting Engine:** Rule-based auto-capitalization and spacing.
