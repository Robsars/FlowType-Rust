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

        pub fn inject(&self, text: &str, allow_commands: bool, shortcuts: &HashMap<String, String>, disable_punctuation: bool) -> Result<()> {
            if text.is_empty() { return Ok(()); }
            
            let mut text_to_inject = text.to_string();

            // 1. Punctuation removal
            if disable_punctuation {
                text_to_inject = text_to_inject.chars().filter(|c| !c.is_ascii_punctuation()).collect();
            }

            info!("Injecting (Windows): '{}' (commands: {})", text_to_inject, allow_commands);

            // 2. Shortcut/Command Handling
            if allow_commands {
                let clean = text_to_inject.trim().to_lowercase();
                
                // Check for dynamic shortcuts
                if let Some(result) = shortcuts.get(&clean) {
                    info!("Shortcut triggered: '{}' -> '{}'", clean, result);
                    match result.as_str() {
                        "[BACKSPACE]" => return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_BACK),
                        "[DELETE]" => return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_DELETE),
                        "[ENTER]" => return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN),
                        "[DELETE_LINE]" => return self.delete_line(),
                        other => {
                            // If it's just text (like an email), update text_to_inject and continue
                            text_to_inject = other.to_string();
                        }
                    }
                }
            }

            if text_to_inject.is_empty() { return Ok(()); }

            let target_is_vscode = self.is_vscode_focused();
            if !target_is_vscode {
                if let Ok(_) = self.inject_uia_text(&text_to_inject) { return Ok(()); }
                if let Ok(_) = self.inject_uia_value(&text_to_inject) { return Ok(()); }
            }

            // Before falling back to keyboard injection, check if we're in an editable element
            // This prevents scrolling in browsers when focus is not in a text field
            if target_is_vscode || self.is_editable_element() {
                if let Ok(_) = self.inject_keyboard_unicode(&text_to_inject) { return Ok(()); }
                self.inject_clipboard(&text_to_inject)
            } else {
                // Not in an editable element - skip injection to prevent unwanted behavior
                info!("Skipping injection: focus is not in an editable text field");
                Ok(())
            }
        }

        fn is_vscode_focused(&self) -> bool {
            use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
            unsafe {
                let hwnd = GetForegroundWindow();
                let mut buffer = [0u16; 512];
                let len = GetWindowTextW(hwnd, &mut buffer);
                if len > 0 {
                    let title = String::from_utf16_lossy(&buffer[..len as usize]);
                    title.contains("Visual Studio Code") || title.contains("Antigravity")
                } else {
                    false
                }
            }
        }

        /// Check if the focused element is an editable text field using UI Automation.
        /// Returns true ONLY if the element is a genuine text input field.
        /// This is deliberately strict to prevent unwanted side effects like scrolling.
        fn is_editable_element(&self) -> bool {
            unsafe {
                let Some(auto) = self.automation.as_ref() else { return false; };
                
                let Ok(element) = auto.GetFocusedElement() else { return false; };

                // Gather diagnostic info
                let name = element.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                let name_lower = name.to_lowercase();
                let control_type = element.CurrentControlType().ok();
                let class_name = element.CurrentClassName().map(|b| b.to_string()).unwrap_or_default();
                let automation_id = element.CurrentAutomationId().map(|b| b.to_string()).unwrap_or_default();
                
                // Check keyboard focusability - critical for determining if we can type here
                let is_keyboard_focusable = element.CurrentIsKeyboardFocusable().map(|b| b.as_bool()).unwrap_or(false);
                
                info!(
                    "Focused Element: Name='{}', TypeID={:?}, ClassName='{}', AutomationId='{}', KeyboardFocusable={}",
                    name, control_type, class_name, automation_id, is_keyboard_focusable
                );

                // ============================================================
                // SPECIAL CASE DETECTION (checked FIRST - before rejection rules)
                // These apps may not expose standard UIA properties correctly.
                // ============================================================
                
                // Google Docs uses a canvas-based editor that may not report as keyboard focusable
                if name_lower.contains("document content") || name_lower.contains("google docs") {
                    info!("Accepting: Google Docs editor detected (special case)");
                    return true;
                }

                // ============================================================
                // REJECTION RULES (checked after special cases)
                // ============================================================
                
                // Reject if not keyboard focusable (can't type into it anyway)
                if !is_keyboard_focusable {
                    info!("Skipping: Element is not keyboard focusable");
                    return false;
                }
                
                // Reject known browser chrome/viewport classes that are NOT text inputs
                let browser_non_edit_classes = [
                    "Chrome_RenderWidgetHostHWND",  // Chrome main render area
                    "MozillaWindowClass",            // Firefox window
                    "Internet Explorer_Server",      // IE/Edge Legacy
                    "CefBrowserWindow",              // CEF-based apps
                ];
                
                for browser_class in browser_non_edit_classes {
                    if class_name.contains(browser_class) {
                        // Even if class matches browser, check if it's specifically an edit control
                        if let Some(ct) = control_type {
                            // 50004 = Edit, 50025 = Document (for contenteditable)
                            if ct.0 != 50004 && ct.0 != 50025 {
                                info!("Skipping: Browser viewport class '{}' with non-edit control type {:?}", class_name, ct);
                                return false;
                            }
                        } else {
                            info!("Skipping: Browser viewport class '{}' with unknown control type", class_name);
                            return false;
                        }
                    }
                }

                // ============================================================
                // ACCEPTANCE RULES (in order of specificity)
                // ============================================================
                
                // 2. Control Type Detection - strict whitelist
                if let Some(ct) = control_type {
                    match ct.0 {
                        50004 => {
                            // UIA_EditControlTypeId - this is definitely a text input
                            info!("Accepting: Control type is Edit (50004)");
                            return true;
                        }
                        50025 => {
                            // UIA_DocumentControlTypeId - could be contenteditable
                            // Only accept if it also has ValuePattern (editable)
                            if element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId).is_ok() {
                                info!("Accepting: Document control with ValuePattern (likely contenteditable)");
                                return true;
                            }
                            // Check if it explicitly looks like an editor
                            if name_lower.contains("editor") || name_lower.contains("compose") || name_lower.contains("message body") {
                                info!("Accepting: Document control with editor-like name");
                                return true;
                            }
                            info!("Skipping: Document control without edit capability indicators");
                            return false;
                        }
                        50020 => {
                            // UIA_PaneControlTypeId - generic pane, usually NOT editable
                            // Exception: Some custom editors use Pane
                            if name_lower.contains("editor") || name_lower.contains("input") {
                                info!("Accepting: Pane control with editor-like name");
                                return true;
                            }
                            info!("Skipping: Generic Pane control");
                            return false;
                        }
                        50033 => {
                            // UIA_GroupControlTypeId - groups are not editable
                            info!("Skipping: Group control type");
                            return false;
                        }
                        _ => {
                            // Other control types - need more evidence
                        }
                    }
                }
                
                // 3. ValuePattern check with additional validation
                // ValuePattern alone isn't enough - many read-only elements expose it
                if let Ok(value_pattern) = element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
                    // Check if it's read-only
                    if let Ok(is_readonly) = value_pattern.CurrentIsReadOnly() {
                        if is_readonly.as_bool() {
                            info!("Skipping: ValuePattern element is read-only");
                            return false;
                        }
                    }
                    // Not read-only, accept it
                    info!("Accepting: Writable ValuePattern element");
                    return true;
                }
                
                // 4. TextPattern is NOT sufficient alone (read-only text areas have it)
                // Only accept TextPattern if the element also looks like an input
                if element.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId).is_ok() {
                    // Must have additional evidence of being editable
                    if name_lower.contains("edit") || name_lower.contains("input") || 
                       name_lower.contains("text box") || name_lower.contains("textarea") ||
                       class_name.to_lowercase().contains("edit") {
                        info!("Accepting: TextPattern with edit-like name/class");
                        return true;
                    }
                    info!("Skipping: TextPattern without edit evidence (likely read-only text)");
                    return false;
                }

                // 5. Name-based detection for rich editors (last resort)
                if name_lower.contains("rich text") || 
                   name_lower.contains("compose") ||
                   name_lower.contains("message body") {
                    info!("Accepting: Rich editor by name heuristic");
                    return true;
                }
                
                // Default: reject unknown elements
                info!("Skipping: No evidence of text input capability");
                false
            }
        }

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
