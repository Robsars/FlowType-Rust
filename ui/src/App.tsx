import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

interface VadPayload {
  state: "speaking" | "silence";
  rms: number;
}

interface TranscriptionPayload {
  text: string;
}

interface AppSettings {
  auto_space: boolean;
  silence_timeout: number;
  allow_commands: boolean;
  disable_punctuation: boolean;
  shortcuts: Record<string, string>;
}

import { invoke } from "@tauri-apps/api/core";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import { ShortcutsModal } from "./ShortcutsModal";

function App() {
  const [vadState, setVadState] = useState<"speaking" | "silence">("silence");
  const [lastText, setLastText] = useState("");
  const [history, setHistory] = useState<string[]>([]);

  // Settings State
  const [autoSpace, setAutoSpace] = useState(true);
  const [silenceTimeout, setSilenceTimeout] = useState(500);
  const [autostart, setAutostart] = useState(false);
  const [allowCommands, setAllowCommands] = useState(true);
  const [disablePunctuation, setDisablePunctuation] = useState(false);
  const [shortcuts, setShortcuts] = useState<Record<string, string>>({});

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);

  const minimize = () => {
    invoke("minimize_window");
  };

  const handleToggleSpace = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newVal = e.target.checked;
    setAutoSpace(newVal);
    invoke("set_auto_space", { state: newVal });
  };

  const handleToggleCommands = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newVal = e.target.checked;
    setAllowCommands(newVal);
    invoke("set_allow_commands", { state: newVal });
  };

  const handleTogglePunctuation = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newVal = e.target.checked;
    setDisablePunctuation(newVal);
    invoke("set_disable_punctuation", { state: newVal });
  };

  const handleTimeoutChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseInt(e.target.value);
    setSilenceTimeout(val);
    invoke("set_silence_timeout", { ms: val });
  };

  const handleToggleAutostart = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const newVal = e.target.checked;
    setAutostart(newVal);
    try {
      if (newVal) await enable();
      else await disable();
    } catch (err) {
      console.error("Autostart error:", err);
    }
  };

  useEffect(() => {
    // Load Settings from Backend
    invoke<AppSettings>("get_settings").then((settings) => {
      setAutoSpace(settings.auto_space);
      setSilenceTimeout(settings.silence_timeout);
      setAllowCommands(settings.allow_commands);
      setDisablePunctuation(settings.disable_punctuation);
      setShortcuts(settings.shortcuts);
    });

    // Check initial autostart status
    isEnabled().then(setAutostart);

    // Listen for VAD updates
    const unlistenVad = listen<VadPayload>("vad-update", (event) => {
      setVadState(event.payload.state);
    });

    // Listen for Transcription updates
    const unlistenTrans = listen<TranscriptionPayload>("transcription", (event) => {
      setLastText(event.payload.text);
      setHistory((prev) => [event.payload.text, ...prev.slice(0, 9)]);
    });

    return () => {
      unlistenVad.then((fn) => fn());
      unlistenTrans.then((fn) => fn());
    };
  }, []);

  return (
    <div className="container">
      <div className={`status-bar ${vadState}`}>
        <span>{vadState === "speaking" ? "🗣️ LISTENING" : "🤫 IDLE"}</span>
        <div className="controls">
          <div className="slider-group" title="Silence timeout (ms)">
            <span>⏳ {silenceTimeout}ms</span>
            <input type="range" min="300" max="2500" step="100" value={silenceTimeout} onChange={handleTimeoutChange} />
          </div>

          <button className="icon-btn" onClick={() => setSettingsOpen(!settingsOpen)} title="Settings">
            ⚙️
          </button>

          <button className="minimize-btn" onClick={minimize} title="Minimize">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M5 12h14" /></svg>
          </button>
        </div>
      </div>

      {settingsOpen && (
        <div className="settings-overlay">
          <div className="settings-modal">
            <div className="settings-header">
              <h3>Settings</h3>
              <button className="close-btn" onClick={() => setSettingsOpen(false)}>×</button>
            </div>

            <div className="setting-item">
              <label>
                <input type="checkbox" checked={autoSpace} onChange={handleToggleSpace} />
                Auto-Space after sentence
              </label>
            </div>

            <div className="setting-item">
              <label>
                <input type="checkbox" checked={disablePunctuation} onChange={handleTogglePunctuation} />
                Disable Punctuation
              </label>
            </div>

            <div className="setting-item">
              <label>
                <input type="checkbox" checked={autostart} onChange={handleToggleAutostart} />
                Start with OS
              </label>
            </div>

            <div className="setting-item">
              <label>
                <input type="checkbox" checked={allowCommands} onChange={handleToggleCommands} />
                Voice Commands & Shortcuts
              </label>
              <button className="shortcut-mgr-btn" onClick={() => setShortcutsOpen(true)}>Manage Shortcuts ({Object.keys(shortcuts).length})</button>
            </div>
          </div>
        </div>
      )}

      {shortcutsOpen && (
        <ShortcutsModal
          shortcuts={shortcuts}
          onClose={() => setShortcutsOpen(false)}
          onUpdate={setShortcuts}
        />
      )}

      {!settingsOpen && !shortcutsOpen && (
        <>
          <div className="main-display">
            <h1>{lastText || "Start speaking..."}</h1>
          </div>

          <div className="history">
            {history.map((text, i) => (
              <p key={i} className="history-item">{text}</p>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

export default App;
