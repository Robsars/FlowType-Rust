# MacOS Compatibility Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable full functionality on MacOS by abstracting text injection logic and ensuring standalone executables (`.app`/`.dmg`) can be built.

**Architecture:** 
- The `TextInjector` struct will be refactored to use platform-specific implementations.
- Windows will retain the existing UIA/WinAPI logic.
- MacOS will use the `enigo` crate (keyboard simulation) to replace the Windows `SendInput` and UIA calls.
- `Cargo.toml` will be updated to conditionally include dependencies.

**Tech Stack:** Rust, Tauri, Enigo (MacOS input), WinAPI (Windows input).

---

### Task 1: Conditional Dependencies
**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Move `windows` to target-specific dependency**
Currently `[dependencies.windows]` is global. We need to put it under `[target.'cfg(target_os = "windows")'.dependencies]`.

**Step 2: Add `enigo` for MacOS**
Add `enigo = "0.2"` (or latest stable) under `[target.'cfg(target_os = "macos")'.dependencies]`.

**Step 3: Verification**
Run `cargo check` (on Windows) to ensure it still resolves.

---

### Task 2: Abstract Injector Logic
**Files:**
- Modify: `src-tauri/src/injector.rs`

**Step 1: Refactor Structure**
Wrap the current `TextInjector` in `#[cfg(target_os = "windows")]` modules.

**Step 2: Implement MacOS Injector**
Create a new `impl TextInjector` for `#[cfg(target_os = "macos")]`.
Logic:
- `new()`: Initialize Enigo.
- `inject(text)`: Use `enigo.key_sequence(text)`.
- Command Handling:
    - "delete" -> `Key::Backspace`
    - "delete that" -> `Key::Command` + `Key::Backspace` (Standard Mac "Delete Line" behavior)
    - "enter" -> `Key::Return`

**Step 3: Fix Compilation Errors**
Ensure all imports (WinAPI vs Enigo) are correctly gated so compilation doesn't fail on the other OS.

---

### Task 3: MacOS Bundle Configuration
**Files:**
- Modify: `src-tauri/tauri.conf.json`

**Step 1: Add Bundle Permissions**
Add `macOS` strict configuration to ensure Microphone permission is requested (Info.plist).

---

## Acceptance Criteria
*   [ ] **Compile Safe:** `cargo check` passes on Windows (and theoretically MacOS).
*   [ ] **Windows Unaffected:** Existing WinAPI injection still works 100% same as before.
*   [ ] **Mac Logic:** 
    *   Text is typed via `Enigo` simulation.
    *   "Delete that" triggers `Cmd+Backspace` (Delete Line).
*   [ ] **Standalone:** MacOS build produces `.app` with bundled model (inherited from previous step).

## Tests
1.  **Build Verification:** Run `cargo check`. Success = No errors about missing `windows` crate.
2.  **Logic Verification:** Review `injector.rs` to ensure `inject()` signature is identical for both platforms.
