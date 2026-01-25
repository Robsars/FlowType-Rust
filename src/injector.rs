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

    pub fn inject(&self, text: &str) -> Result<()> {
        if text.is_empty() { return Ok(()); }
        
        // Strategy 1: UIA Text Pattern
        if let Ok(_) = self.inject_uia_text(text) {
             info!("Injected via UIA TextPattern");
             return Ok(());
        }

        // Strategy 2: UIA Value Pattern
        if let Ok(_) = self.inject_uia_value(text) {
            info!("Injected via UIA ValuePattern");
            return Ok(());
        }

        // Strategy 3: Clipboard/SendKeys Fallback
        warn!("UIA failed. Falling back to Clipboard.");
        self.inject_clipboard(text)
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
}
