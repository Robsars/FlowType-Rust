# Refined MacOS Compatibility Plan

**Goal:** Finalize macOS compatibility by ensuring all Tauri permissions (capabilities) are correctly configured and code is clean.

## Changes

### 1. Update Capabilities
**File:** `src-tauri/capabilities/default.json`
**Why:** The application uses `tauri-plugin-autostart` and `tauri-plugin-log` in `lib.rs`, but these permissions are missing from the default capability set. This will cause these features to fail on both Windows and macOS.
**Action:** Add `"autostart:default"` and `"log:default"` to the permissions list.

### 2. Clean Up Injector Code
**File:** `src-tauri/src/injector.rs`
**Why:** The code currently has unused imports (`warn`) which cause compiler warnings. Keeping the build clean is important for cross-platform maintenance.
**Action:** Remove the unused `warn` import.

### 3. Verify MacOS Bundle Configuration
**File:** `src-tauri/tauri.conf.json`
**Why:** Ensure the `macOS` bundle configuration explicitly points to the `Info.plist` if strictly necessary, or at least ensures the bundle identifier is correct.
**Action:** (Already mostly done, just verification) - We will ensure `identifier` is consistent.

## Acceptance Criteria
*   [ ] **Capabilities Configured:** `default.json` includes `autostart:default` and `log:default`.
*   [ ] **Clean Build:** `cargo check` executes with **zero warnings** regarding unused imports in `injector.rs`.
*   [ ] **MacOS Permissions:** `Info.plist` is created (done) and contains microphone usage description.

## Tests
1.  **Capabilities Check:** Inspect `src-tauri/capabilities/default.json` to verify permissions are present.
2.  **Clean Compile:** Run `cargo check` in `src-tauri` and verify exit code 0 and no output about `unused import: warn`.
