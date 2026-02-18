use anyhow::{Result, anyhow};
use log::info;
use std::collections::HashMap;

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows::Win32::System::Com::{CoInitializeEx, CoCreateInstance, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, UIA_TextPatternId, UIA_ValuePatternId, IUIAutomationTextPattern, IUIAutomationValuePattern};
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V, VIRTUAL_KEY};

    pub struct PlatformInjector {
        automation: Option<IUIAutomation>,
    }

    #[derive(Debug)]
    enum WindowContext {
        VSCode,
        GoogleDocs,
        Browser,   // Chrome, Edge, Firefox, Opera, Brave, etc.
        NativeApp, // Notepad, Word, etc.
    }

    impl PlatformInjector {
        pub fn new() -> Result<Self> {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED)?;
                let automation: IUIAutomation = CoCreateInstance(
                    &CUIAutomation, 
                    None, 
                    CLSCTX_INPROC_SERVER
                ).map_err(|e| anyhow!("Failed to create IUIAutomation: {}", e))?;
                Ok(Self { automation: Some(automation) })
            }
        }

        // ============================================================
        // Window Context Detection
        // ============================================================
        fn get_window_context(&self) -> WindowContext {
            use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
            unsafe {
                let hwnd = GetForegroundWindow();
                let mut buffer = [0u16; 512];
                let len = GetWindowTextW(hwnd, &mut buffer);
                if len > 0 {
                    let title = String::from_utf16_lossy(&buffer[..len as usize]);
                    let title_lower = title.to_lowercase().trim().to_string();
                    
                    info!("Window title: '{}'", title_lower);

                    // VS Code / Antigravity
                    if title.contains("Visual Studio Code") || title.contains("Antigravity") {
                        return WindowContext::VSCode;
                    }

                    // Google Docs/Sheets/Slides (special canvas-based editor)
                    if title_lower.contains("google docs") ||
                       title_lower.contains("google sheets") ||
                       title_lower.contains("google slides") {
                        return WindowContext::GoogleDocs;
                    }

                    // Browser detection — use contains() to handle trailing whitespace/chars
                    // Covers all major Chromium-based and non-Chromium browsers
                    if title_lower.contains("- google chrome") ||
                       title_lower.contains("- microsoft edge") ||
                       title_lower.contains("- mozilla firefox") ||
                       title_lower.contains("- firefox") ||
                       title_lower.contains("- opera") ||
                       title_lower.contains("- brave") ||
                       title_lower.contains("- vivaldi") ||
                       title_lower.contains("- arc") {
                        return WindowContext::Browser;
                    }
                }

                WindowContext::NativeApp
            }
        }

        // ============================================================
        // Universal Text-Field Detection
        // ============================================================

        /// Determine if the UIA focused element is a genuine text input field.
        /// This is the UNIVERSAL GATE for ALL injection (browsers AND native apps).
        /// Only VS Code and Google Docs bypass this check.
        ///
        /// Accepts:
        ///   • Control type 50004 (Edit)            – <input>, <textarea>, Notepad, etc.
        ///   • Control type 50025 (Document) IF writable AND has editor-like name
        ///
        /// Rejects EVERYTHING else — file lists, toolbars, browser viewports, menus, etc.
        fn is_text_field(&self) -> bool {
            unsafe {
                let Some(auto) = self.automation.as_ref() else { return false; };
                let Ok(element) = auto.GetFocusedElement() else { return false; };

                let name = element.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                let name_lower = name.to_lowercase();
                let control_type = element.CurrentControlType().ok();
                let class_name = element.CurrentClassName().map(|b| b.to_string()).unwrap_or_default();
                let is_kbd_focusable = element.CurrentIsKeyboardFocusable().map(|b| b.as_bool()).unwrap_or(false);

                info!(
                    "Focused element: Name='{}', TypeID={:?}, Class='{}', KbdFocus={}",
                    name, control_type, class_name, is_kbd_focusable
                );

                // Not keyboard focusable → can't type here
                if !is_kbd_focusable {
                    info!("  → Not keyboard focusable → reject");
                    return false;
                }

                if let Some(ct) = control_type {
                    match ct.0 {
                        // ── Edit (50004) ─────────────────────────────────
                        // Notepad text area, browser <input>, <textarea>, etc.
                        50004 => {
                            info!("  → Edit control (50004) → accept");
                            return true;
                        }
                        // ── Document (50025) ────────────────────────────
                        // Could be contenteditable div, Word doc, or browser page body.
                        // Require writable ValuePattern + editor-like naming.
                        50025 => {
                            if let Ok(vp) = element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
                                let read_only = vp.CurrentIsReadOnly().map(|b| b.as_bool()).unwrap_or(true);
                                if !read_only {
                                    let is_editor =
                                        name_lower.contains("editor") ||
                                        name_lower.contains("compose") ||
                                        name_lower.contains("message body") ||
                                        name_lower.contains("rich text") ||
                                        name_lower.contains("mail body") ||
                                        name_lower.contains("editing");
                                    if is_editor {
                                        info!("  → Writable Document + editor name → accept");
                                        return true;
                                    }
                                }
                            }
                            info!("  → Document without editor evidence → reject");
                            return false;
                        }
                        // ── Everything else → reject ─────────────────────
                        // ListItem (file explorer), Pane, Group, Button, etc.
                        other => {
                            info!("  → Control type {} → reject (not Edit/Document)", other);
                            return false;
                        }
                    }
                }

                info!("  → Unknown control type → reject");
                false
            }
        }

        // ============================================================
        // Main injection entry-point
        // ============================================================

        pub fn inject(&self, text: &str, allow_commands: bool, shortcuts: &HashMap<String, String>, disable_punctuation: bool) -> Result<()> {
            if text.is_empty() { return Ok(()); }

            let mut text_to_inject = text.to_string();

            // 1. Punctuation removal
            if disable_punctuation {
                text_to_inject = text_to_inject.chars().filter(|c| !c.is_ascii_punctuation()).collect();
            }

            info!("Injecting (Windows): '{}' (commands: {})", text_to_inject, allow_commands);

            // 2. Determine window context ONCE
            let ctx = self.get_window_context();
            info!("Window context: {:?}", ctx);

            // 3. Shortcut / command handling
            if allow_commands {
                let clean: String = text_to_inject
                    .trim()
                    .to_lowercase()
                    .chars()
                    .filter(|c| !c.is_ascii_punctuation())
                    .collect();

                info!("🎤 Command check: looking for '{}' in {} shortcuts", clean, shortcuts.len());

                if let Some(result) = shortcuts.get(&clean) {
                    info!("✅ Shortcut triggered: '{}' -> '{}'", clean, result);
                    match result.as_str() {
                        "[BACKSPACE]" => return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_BACK),
                        "[DELETE]"    => return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_DELETE),
                        "[ENTER]"     => return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN),
                        "[DELETE_LINE]" => return self.delete_line(),
                        other => {
                            text_to_inject = other.to_string();
                        }
                    }
                }
            }

            if text_to_inject.is_empty() { return Ok(()); }

            // 4. Injection strategy — determined by window context
            match ctx {
                WindowContext::VSCode => {
                    info!("📝 VS Code → keyboard injection");
                    if let Ok(_) = self.inject_keyboard_unicode(&text_to_inject) { return Ok(()); }
                    self.inject_clipboard(&text_to_inject)
                }

                WindowContext::GoogleDocs => {
                    info!("📝 Google Docs → keyboard injection");
                    if let Ok(_) = self.inject_keyboard_unicode(&text_to_inject) { return Ok(()); }
                    self.inject_clipboard(&text_to_inject)
                }

                WindowContext::Browser => {
                    // For ANY browser: only inject if we're in a real text field
                    if self.is_text_field() {
                        info!("📝 Browser text field → clipboard injection");
                        self.inject_clipboard(&text_to_inject)
                    } else {
                        info!("🛑 Browser: not in text field → blocking injection");
                        Ok(())
                    }
                }

                WindowContext::NativeApp => {
                    // Native apps: MUST also check if in a text field first!
                    // Without this, File Explorer items get renamed, etc.
                    if self.is_text_field() {
                        info!("📝 Native text field → UIA/keyboard injection");
                        if let Ok(_) = self.inject_uia_text(&text_to_inject) { return Ok(()); }
                        if let Ok(_) = self.inject_uia_value(&text_to_inject) { return Ok(()); }
                        if let Ok(_) = self.inject_keyboard_unicode(&text_to_inject) { return Ok(()); }
                        self.inject_clipboard(&text_to_inject)
                    } else {
                        info!("🛑 Native app: not in text field → blocking injection");
                        Ok(())
                    }
                }
            }
        }

        // ============================================================
        // Low-level injection methods
        // ============================================================

        fn inject_keyboard_unicode(&self, text: &str) -> Result<()> {
            use windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_UNICODE;
            let mut inputs = Vec::new();
            for c in text.encode_utf16() {
                inputs.push(INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT { wScan: c, dwFlags: KEYEVENTF_UNICODE, ..Default::default() }
                    }
                });
                inputs.push(INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT { wScan: c, dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, ..Default::default() }
                    }
                });
            }
            unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32); }
            Ok(())
        }

        fn inject_uia_text(&self, _text: &str) -> Result<()> {
            unsafe {
                let auto = self.automation.as_ref().unwrap();
                let element = auto.GetFocusedElement()?;
                let _pattern_obj: IUIAutomationTextPattern = element.GetCurrentPatternAs(UIA_TextPatternId)?;
                Err(anyhow!("UIA Text write not fully impl (safe fallback)"))
            }
        }
        
        fn inject_uia_value(&self, text: &str) -> Result<()> {
             unsafe {
                let auto = self.automation.as_ref().unwrap();
                let element = auto.GetFocusedElement()?;
                let pattern_obj: IUIAutomationValuePattern = element.GetCurrentPatternAs(UIA_ValuePatternId)?;
                let current_val = pattern_obj.CurrentValue()?;
                let new_val = format!("{}{}", current_val, text);
                let bstr = windows::core::BSTR::from(new_val);
                pattern_obj.SetValue(&bstr)?;
                Ok(())
             }
        }

        fn inject_clipboard(&self, text: &str) -> Result<()> {
            let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow!("Clipboard init failed: {}", e))?;
            clipboard.set_text(text).map_err(|e| anyhow!("Clipboard set failed: {}", e))?;
            unsafe {
                let k_ctrl = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, ..Default::default() } } };
                let k_v = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_V, ..Default::default() } } };
                let k_v_up = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_V, dwFlags: KEYEVENTF_KEYUP, ..Default::default() } } };
                let k_ctrl_up = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, dwFlags: KEYEVENTF_KEYUP, ..Default::default() } } };
                let inputs = [k_ctrl, k_v, k_v_up, k_ctrl_up];
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }

        fn send_key(&self, vk: VIRTUAL_KEY) -> Result<()> {
            unsafe {
                let down = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, ..Default::default() } } };
                let up = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: KEYEVENTF_KEYUP, ..Default::default() } } };
                let inputs = [down, up];
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }

        fn delete_line(&self) -> Result<()> {
            use windows::Win32::UI::Input::KeyboardAndMouse::{VK_SHIFT, VK_HOME, VK_BACK};
            unsafe {
                let shift_down = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_SHIFT, ..Default::default() } } };
                let home_down = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_HOME, ..Default::default() } } };
                let home_up = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_HOME, dwFlags: KEYEVENTF_KEYUP, ..Default::default() } } };
                let shift_up = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_SHIFT, dwFlags: KEYEVENTF_KEYUP, ..Default::default() } } };
                let back_down = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_BACK, ..Default::default() } } };
                let back_up = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_BACK, dwFlags: KEYEVENTF_KEYUP, ..Default::default() } } };
                let inputs = [shift_down, home_down, home_up, shift_up, back_down, back_up];
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use enigo::{Enigo, Keyboard, Settings, Key, Direction};

    pub struct PlatformInjector {
        enigo: Enigo,
    }

    impl PlatformInjector {
        pub fn new() -> Result<Self> {
            let enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow!("Failed to init Enigo: {}", e))?;
            Ok(Self { enigo })
        }

        pub fn inject(&self, text: &str, allow_commands: bool, shortcuts: &HashMap<String, String>, disable_punctuation: bool) -> Result<()> {
            if text.is_empty() { return Ok(()); }
            
            let mut text_to_inject = text.to_string();

            // 1. Punctuation removal
            if disable_punctuation {
                text_to_inject = text_to_inject.chars().filter(|c| !c.is_ascii_punctuation()).collect();
            }

            info!("Injecting (MacOS): '{}' (commands: {})", text_to_inject, allow_commands);

            let mut enigo = self.enigo.clone();

            // 2. Shortcut/Command Handling
            if allow_commands {
                let clean = text_to_inject.trim().to_lowercase();
                
                if let Some(result) = shortcuts.get(&clean) {
                    info!("Shortcut triggered: '{}' -> '{}'", clean, result);
                    match result.as_str() {
                        "[BACKSPACE]" => return enigo.key(Key::Backspace, Direction::Click).map_err(|e| anyhow!("{}", e)),
                        "[DELETE]" => return enigo.key(Key::Delete, Direction::Click).map_err(|e| anyhow!("{}", e)),
                        "[ENTER]" => return enigo.key(Key::Return, Direction::Click).map_err(|e| anyhow!("{}", e)),
                        "[DELETE_LINE]" => {
                            enigo.key(Key::Command, Direction::Press).ok();
                            enigo.key(Key::Backspace, Direction::Click).ok();
                            return enigo.key(Key::Command, Direction::Release).map_err(|e| anyhow!("{}", e));
                        },
                        other => {
                            text_to_inject = other.to_string();
                        }
                    }
                }
            }

            if text_to_inject.is_empty() { return Ok(()); }
            enigo.text(&text_to_inject).map_err(|e| anyhow!("Enigo injection failed: {}", e))?;
            Ok(())
        }
    }
}

pub struct TextInjector {
    inner: platform::PlatformInjector,
}

impl TextInjector {
    pub fn new() -> Result<Self> {
        Ok(Self { inner: platform::PlatformInjector::new()? })
    }

    pub fn inject(&self, text: &str, allow_commands: bool, shortcuts: &HashMap<String, String>, disable_punctuation: bool) -> Result<()> {
        self.inner.inject(text, allow_commands, shortcuts, disable_punctuation)
    }
}
