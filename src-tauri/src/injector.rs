use anyhow::{Result, anyhow};
use log::{info, warn};
use windows::Win32::System::Com::{CoInitializeEx, CoCreateInstance, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, UIA_TextPatternId, UIA_ValuePatternId, IUIAutomationTextPattern, IUIAutomationValuePattern};
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V};

pub struct TextInjector {
    automation: Option<IUIAutomation>,
}

impl TextInjector {
    pub fn new() -> Result<Self> {
        unsafe {
            // Initialize COM for this thread
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)?;
            
            // Create UIA instance
            let automation: IUIAutomation = CoCreateInstance(
                &CUIAutomation, 
                None, 
                CLSCTX_INPROC_SERVER
            ).map_err(|e| anyhow!("Failed to create IUIAutomation: {}", e))?;

            Ok(Self { automation: Some(automation) })
        }
    }

    pub fn inject(&self, text: &str, allow_commands: bool) -> Result<()> {
        if text.is_empty() { return Ok(()); }
        
        info!("Injecting: '{}' (commands: {})", text, allow_commands);

        // --- Voice Command Triggers ---
        if allow_commands {
            let clean = text.trim().to_lowercase();
            let word_only = clean.replace(".", "").replace("?", "").replace("!", "").replace(",", "");
            
            // 1. "delete" -> Delete key
            if word_only == "delete" {
                info!("Trigger: DELETE");
                return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_DELETE);
            }
            
            // 2. "backspace" -> Backspace key
            if word_only == "backspace" {
                 info!("Trigger: BACKSPACE");
                 return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_BACK);
            }

            // 3. "period" or "." -> Backspace + "." + Space
            if word_only == "period" || clean == "." {
                 info!("Trigger: PERIOD");
                 self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_BACK)?;
                 self.inject_keyboard_unicode(".")?;
                 self.inject_keyboard_unicode(" ")?;
                 return Ok(()); 
            }

            // 4. "delete that" -> Delete line
            if word_only == "delete that" {
                 info!("Trigger: DELETE LINE");
                 return self.delete_line();
            }

            // 5. "space" -> Space key
            if word_only == "space" {
                 info!("Trigger: SPACE");
                 return self.inject_keyboard_unicode(" ");
            }

            // 6. "new line" or "enter" -> Enter key
            if word_only == "new line" || word_only == "enter" {
                 info!("Trigger: ENTER");
                 return self.send_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN);
            }
        }

        // Normal Injection Flow...
        let target_is_vscode = self.is_vscode_focused();

        // Strategy 1: UIA (Silent) - Skip for VS Code as it's notoriously unreliable
        if !target_is_vscode {
            if let Ok(_) = self.inject_uia_text(text) {
                 info!("Injected via UIA TextPattern");
                 return Ok(());
            }

            if let Ok(_) = self.inject_uia_value(text) {
                info!("Injected via UIA ValuePattern");
                return Ok(());
            }
        } else {
            info!("Target is VS Code. Using Clipboard fallback for reliability.");
        }

        // Strategy 2: Unicode Keyboard Input (High Compatibility)
        if let Ok(_) = self.inject_keyboard_unicode(text) {
             info!("Injected via Unicode Keyboard Input");
             return Ok(());
        }

        // Strategy 3: Clipboard/SendKeys Fallback
        warn!("Full fallback to Clipboard.");
        self.inject_clipboard(text)
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

    fn inject_keyboard_unicode(&self, text: &str) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{KEYBDINPUT, KEYEVENTF_UNICODE};
        
        let mut inputs = Vec::new();
        for c in text.encode_utf16() {
            // Key Down
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wScan: c,
                        dwFlags: KEYEVENTF_UNICODE,
                        ..Default::default()
                    }
                }
            });
            // Key Up
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wScan: c,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        ..Default::default()
                    }
                }
            });
        }

        unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    fn inject_uia_text(&self, text: &str) -> Result<()> {
        unsafe {
            let auto = self.automation.as_ref().unwrap();
            let element = auto.GetFocusedElement()?;
            
            // Try Get Pattern
            let pattern_obj: IUIAutomationTextPattern = element.GetCurrentPatternAs(UIA_TextPatternId)?;
            
            // If we got here, we have a text pattern!
            // Get selection or insertion point
            let ranges = pattern_obj.GetSelection()?;
            if ranges.Length()? > 0 {
                let range = ranges.GetElement(0)?;
                
                // Construct BSTR manually or let windows crate handle it
                let _bstr = windows::core::BSTR::from(text);
                range.Select()?; // Ensure active
                
                // This replaces specific selection. Ideally we want 'Insert' but standard UIA is often 'SetValue' on range
                // A common way to insert at cursor:
                // text_pattern.GetSelection() -> range[0].ExpandToEnclosingUnit(Character) -> range.Select().
                
                // For simplified "Pipeline":
                // Just use the TextPattern to Set the text of the selection (which replaces it)
                // Warning: This implies we need a 'Paste' behavior or 'Type' behavior.
                // UIA TextPattern doesn't have a simple 'Type' method. It has 'Move endpoints' and 'Select'.
                
                // Wait, 'IUIAutomationTextRange::AddToSelection' adds to selection.
                // We typically use standard OS paste if we can't deeply manipulate.
                // But Prompt said: "Call InsertText(text) on the range" -> Let's verify if that method exists in Windows API.
                // It does not exist on standard IUIAutomationTextRange.
                
                // Standard UIA approach:
                // 1. ValuePattern.SetValue() (Replaces ALL text) -> BAD for typing.
                // 2. TextPattern: We can read, but modifying is hard without specific provider support.
                // The spec mentioned "InsertText". Maybe it meant `LegacyIAccessiblePattern` -> `SetValue`?
                
                // Let's rely on ValuePattern first if TextPattern is complex/unsupported for write.
                return Err(anyhow!("UIA Text write not fully impl (safe fallback)"));
            }
            Err(anyhow!("No selection range found"))
        }
    }
    
    fn inject_uia_value(&self, text: &str) -> Result<()> {
         unsafe {
            let auto = self.automation.as_ref().unwrap();
            let element = auto.GetFocusedElement()?;
            let pattern_obj: IUIAutomationValuePattern = element.GetCurrentPatternAs(UIA_ValuePatternId)?;
            
            // ValuePattern usually replaces EVERYTHING.
            // So we must Read, Append, Write.
            let current_val = pattern_obj.CurrentValue()?;
            let new_val = format!("{}{}", current_val, text);
            let bstr = windows::core::BSTR::from(new_val);
            pattern_obj.SetValue(&bstr)?;
            
            Ok(())
         }
    }

    fn inject_clipboard(&self, text: &str) -> Result<()> {
        // 1. Set Clipboard
        let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow!("Clipboard init failed: {}", e))?;
        clipboard.set_text(text).map_err(|e| anyhow!("Clipboard set failed: {}", e))?;

        // 2. Send Ctrl+V
        unsafe {
            let k_ctrl = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        ..Default::default()
                    }
                }
            };
            
            let k_v = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_V,
                        ..Default::default()
                    }
                }
            };

            let k_v_up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_V,
                        dwFlags: KEYEVENTF_KEYUP,
                        ..Default::default()
                    }
                }
            };

            let k_ctrl_up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        dwFlags: KEYEVENTF_KEYUP,
                        ..Default::default()
                    }
                }
            };

            let inputs = [k_ctrl, k_v, k_v_up, k_ctrl_up];
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        
        info!("Injected via Clipboard: '{}'", text);
        Ok(())
    }

    fn send_key(&self, vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{KEYBDINPUT, KEYEVENTF_KEYUP};
        
        unsafe {
            let down = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, ..Default::default() } }
            };
            let up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: KEYEVENTF_KEYUP, ..Default::default() } }
            };
            let inputs = [down, up];
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    fn delete_line(&self) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{KEYBDINPUT, KEYEVENTF_KEYUP, VK_SHIFT, VK_HOME, VK_BACK};
        
        // Simulating: Shift Down -> Home -> Shift Up -> Backspace
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
