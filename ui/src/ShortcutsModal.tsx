import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ShortcutsModalProps {
    shortcuts: Record<string, string>;
    onClose: () => void;
    onUpdate: (shortcuts: Record<string, string>) => void;
}

export function ShortcutsModal({ shortcuts, onClose, onUpdate }: ShortcutsModalProps) {
    const [newKey, setNewKey] = useState("");
    const [newValue, setNewValue] = useState("");

    const handleAdd = async () => {
        if (!newKey || !newValue) return;
        await invoke("upsert_shortcut", { key: newKey, value: newValue });
        onUpdate({ ...shortcuts, [newKey.toLowerCase()]: newValue });
        setNewKey("");
        setNewValue("");
    };

    const handleDelete = async (key: string) => {
        await invoke("delete_shortcut", { key });
        const next = { ...shortcuts };
        delete next[key];
        onUpdate(next);
    };

    return (
        <div className="settings-overlay">
            <div className="settings-modal shortcuts-modal">
                <div className="settings-header">
                    <h3>Voice Shortcuts</h3>
                    <button className="close-btn" onClick={onClose}>×</button>
                </div>

                <div className="shortcut-form">
                    <input
                        type="text"
                        placeholder="When I say..."
                        value={newKey}
                        onChange={(e) => setNewKey(e.target.value)}
                    />
                    <div className="arrow">→</div>
                    <input
                        type="text"
                        placeholder="Type this / [TOKEN]"
                        value={newValue}
                        onChange={(e) => setNewValue(e.target.value)}
                    />
                    <button className="add-btn" onClick={handleAdd}>Add</button>
                </div>

                <div className="shortcut-tokens">
                    <span>Tokens:</span>
                    <code>[BACKSPACE]</code>
                    <code>[ENTER]</code>
                    <code>[DELETE_LINE]</code>
                </div>

                <div className="shortcut-list">
                    {Object.entries(shortcuts).map(([key, value]) => (
                        <div key={key} className="shortcut-item">
                            <div className="shortcut-info">
                                <span className="key">{key}</span>
                                <span className="arrow">→</span>
                                <span className="value">{value}</span>
                            </div>
                            <button className="delete-btn" onClick={() => handleDelete(key)}>×</button>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
