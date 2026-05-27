export function invoke(cmd, args) {
    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
        return window.__TAURI_INTERNALS__.invoke(cmd, args);
    }
    return Promise.reject('Tauri not ready');
}

export function listen(event, handler) {
    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.transformCallback) {
        window.__TAURI_INTERNALS__.invoke('plugin:event|listen', {
            event,
            target: { kind: 'Any' },
            handler: window.__TAURI_INTERNALS__.transformCallback(handler)
        });
    }
}

export function escapeHtml(text) {
    if (!text) return '';
    const d = document.createElement('div');
    d.textContent = text;
    return d.innerHTML;
}
