// ── Theme ──
function applyTheme(mode) {
    if (!mode) mode = document.getElementById('themeMode').value;
    localStorage.setItem('claudemeow_theme', mode);
    document.documentElement.classList.remove('dark');
    if (mode === 'dark' || (mode === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
        document.documentElement.classList.add('dark');
    }
}
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    const mode = localStorage.getItem('claudemeow_theme') || 'dark';
    if (mode === 'system') applyTheme('system');
});
applyTheme(localStorage.getItem('claudemeow_theme') || 'dark');

function cycleTheme() {
    const current = localStorage.getItem('claudemeow_theme') || 'dark';
    const next = current === 'dark' ? 'light' : 'dark';
    applyTheme(next);
    updateThemeIcon();
}
function updateThemeIcon() {
    const mode = localStorage.getItem('claudemeow_theme') || 'dark';
    const moon = document.getElementById('themeIconMoon');
    const sun = document.getElementById('themeIconSun');
    const label = document.getElementById('themeLabel');
    if (!moon) return;
    moon.style.display = mode === 'dark' ? '' : 'none';
    sun.style.display = mode === 'light' ? '' : 'none';
    if (label) label.textContent = mode === 'dark' ? 'Dark' : 'Light';
}
updateThemeIcon();

// ── Navigation ──
function switchPage(name, el) {
    document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
    document.querySelectorAll('.page').forEach(p => { p.classList.remove('active'); });
    el.classList.add('active');
    const page = document.getElementById('page-' + name);
    void page.offsetWidth;
    page.classList.add('active');
}

// ── Window controls ──
function hideWindow() {
    window.__TAURI_INTERNALS__.invoke('plugin:window|hide', { label: 'settings' });
}
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hideWindow();
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        clearTimeout(saveTimer);
        autoSave();
    }
});

// ── Provider fields ──
function updateProviderFields() {
    const v = document.getElementById('providerSelect').value;
    document.getElementById('bedrockFields').style.display = (v === 'bedrock_apikey' || v === 'bedrock_iam') ? '' : 'none';
    document.getElementById('bedrockKeyRow').style.display = (v === 'bedrock_apikey') ? '' : 'none';
    document.getElementById('bedrockIamNote').style.display = (v === 'bedrock_iam') ? '' : 'none';
    document.getElementById('anthropicFields').style.display = (v === 'anthropic') ? '' : 'none';
    document.getElementById('openaiFields').style.display = (v === 'openai') ? '' : 'none';
    scheduleAutoSave();
}

// ── Auto-Save ──
let saveTimer = null;
function scheduleAutoSave() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(autoSave, 800);
}
async function autoSave() {
    const provider = document.getElementById('providerSelect').value;
    let apiKey = '';
    if (provider === 'anthropic') apiKey = document.getElementById('anthropicApiKey').value;
    else if (provider !== 'openai') apiKey = document.getElementById('apiKey').value;

    const config = {
        provider,
        aws_region: document.getElementById('region').value,
        model_id: document.getElementById('modelId').value,
        anthropic_model_id: document.getElementById('anthropicModelId').value,
        openai_api_key: document.getElementById('openaiApiKey').value,
        openai_model_id: document.getElementById('openaiModelId').value,
        persona: document.getElementById('persona').value,
        interval_minutes: parseInt(document.getElementById('interval').value),
        activity_enabled: document.getElementById('activityEnabled').checked,
        dev_mode: document.getElementById('devMode').checked,
        auth_mode: 'apikey',
        api_key: apiKey,
        user_nickname: userNicknames[0] || '',
        nicknames: userNicknames,
        chattiness: parseInt(document.getElementById('chattiness').value),
        sass_level: parseInt(document.getElementById('sassLevel').value),
        language_mix: document.getElementById('languageMix').value,
        health_enabled: document.getElementById('healthEnabled').checked,
        health_interval_minutes: parseInt(document.getElementById('healthInterval').value),
        bubble_duration_secs: parseInt(document.getElementById('bubbleDisplayTime')?.value || document.getElementById('bubbleDuration')?.value || 30),
        quiet_hours_start: parseInt(document.getElementById('quietStart').value),
        quiet_hours_end: parseInt(document.getElementById('quietEnd').value),
        idle_gap_minutes: parseInt(document.getElementById('idleGap')?.value || 3),
        reaction_cooldown_secs: parseInt(document.getElementById('reactionCooldown')?.value || 10),
    };
    try {
        await window.__TAURI_INTERNALS__.invoke('set_config', { config });
        showSaved();
        updateStatusRail();
    } catch (e) { console.error('autoSave error:', e); }
}

function showSaved() {
    const header = document.querySelector('.page.active .page-title') || document.querySelector('.page.active .page-header');
    if (!header) return;
    let indicator = header.querySelector('.save-indicator');
    if (!indicator) {
        indicator = document.createElement('span');
        indicator.className = 'save-indicator';
        indicator.textContent = 'Saved';
        header.appendChild(indicator);
    }
    indicator.classList.remove('show');
    void indicator.offsetWidth;
    indicator.classList.add('show');
    setTimeout(() => indicator.classList.remove('show'), 1500);
}

document.addEventListener('DOMContentLoaded', () => {
    document.querySelectorAll('input:not(.nick-input):not([type="checkbox"]), select, textarea').forEach(el => {
        el.addEventListener('input', scheduleAutoSave);
        el.addEventListener('change', scheduleAutoSave);
    });
    document.querySelectorAll('.toggle input[type="checkbox"]').forEach(el => {
        if (el.id !== 'amazonCrEnabled' && el.id !== 'amazonCalendarEnabled') {
            el.addEventListener('change', scheduleAutoSave);
        }
    });
});

// ── Test API ──
async function testApi() {
    const v = document.getElementById('providerSelect').value;
    let statusEl, apiKey, modelId, region;

    if (v === 'anthropic') {
        statusEl = document.getElementById('anthropicTestStatus');
        apiKey = document.getElementById('anthropicApiKey').value.trim();
        modelId = document.getElementById('anthropicModelId').value;
        region = '';
    } else if (v === 'openai') {
        statusEl = document.getElementById('openaiTestStatus');
        apiKey = document.getElementById('openaiApiKey').value.trim();
        modelId = document.getElementById('openaiModelId').value;
        region = '';
    } else {
        statusEl = document.getElementById('testStatus');
        apiKey = document.getElementById('apiKey').value.trim();
        modelId = document.getElementById('modelId').value;
        region = document.getElementById('region').value;
    }

    if (!apiKey && v !== 'bedrock_iam') {
        statusEl.textContent = 'Enter an API key first';
        statusEl.className = 'test-status error';
        return;
    }
    statusEl.textContent = 'Testing...'; statusEl.className = 'test-status loading';
    try {
        await window.__TAURI_INTERNALS__.invoke('test_api', { provider: v, apiKey, region, modelId });
        statusEl.textContent = 'Connected!';
        statusEl.className = 'test-status success';
        await autoSave();
    } catch (e) {
        statusEl.textContent = 'Failed: ' + String(e).slice(0, 80);
        statusEl.className = 'test-status error';
    }
}

// ── Nickname Chips ──
let userNicknames = [];

function renderNicknames() {
    const container = document.getElementById('nicknameChips');
    if (!container) return;
    container.innerHTML = userNicknames.map((nick, i) =>
        '<span class="chip">' + escHtml(nick) + '<button class="chip-remove" onclick="removeNickname(' + i + ')">×</button></span>'
    ).join('');
    container.classList.toggle('has-chips', userNicknames.length > 0);
    // Sync first nickname to hidden field for auto-save
    const hidden = document.getElementById('userNickname');
    if (hidden) hidden.value = userNicknames[0] || '';
}

function addNickname() {
    const input = document.getElementById('nicknameInput');
    const val = input.value.trim();
    if (!val || userNicknames.includes(val) || userNicknames.length >= 10) return;
    userNicknames.push(val);
    input.value = '';
    renderNicknames();
    scheduleAutoSave();
}

function removeNickname(idx) {
    const removed = userNicknames.splice(idx, 1)[0];
    // Track removed .meow items so they don't come back
    const dismissed = JSON.parse(localStorage.getItem('meow_dismissed_nicks') || '[]');
    if (removed && !dismissed.includes(removed)) { dismissed.push(removed); localStorage.setItem('meow_dismissed_nicks', JSON.stringify(dismissed)); }
    renderNicknames();
    scheduleAutoSave();
}

// ── User Facts (chips) ──
let userFacts = [];

function renderFacts() {
    const container = document.getElementById('userFactsContainer');
    if (!container) return;
    container.innerHTML = userFacts.map((fact, i) =>
        '<span class="chip">' + escHtml(fact) + '<button class="chip-remove" onclick="removeUserFact(' + i + ')">×</button></span>'
    ).join('');
    container.classList.toggle('has-chips', userFacts.length > 0);
}

function addUserFact() {
    const input = document.getElementById('userFactInput');
    const val = input.value.trim();
    if (!val || userFacts.length >= 20) return;
    userFacts.push(val);
    input.value = '';
    renderFacts();
    saveUserFacts();
}

function removeUserFact(idx) {
    const removed = userFacts.splice(idx, 1)[0];
    // Track removed .meow items so they don't come back
    const dismissed = JSON.parse(localStorage.getItem('meow_dismissed_facts') || '[]');
    if (removed && !dismissed.includes(removed)) { dismissed.push(removed); localStorage.setItem('meow_dismissed_facts', JSON.stringify(dismissed)); }
    renderFacts();
    saveUserFacts();
}

function saveUserFacts() {
    const content = userFacts.join('\n');
    window.__TAURI_INTERNALS__.invoke('set_user_profile', { content }).catch(() => {});
}

// Enter key to add
document.addEventListener('DOMContentLoaded', () => {
    const factInput = document.getElementById('userFactInput');
    if (factInput) factInput.addEventListener('keypress', (e) => { if (e.key === 'Enter') { e.preventDefault(); addUserFact(); } });
    const nickInput = document.getElementById('nicknameInput');
    if (nickInput) nickInput.addEventListener('keypress', (e) => { if (e.key === 'Enter') { e.preventDefault(); addNickname(); } });
});

// ── Load Settings ──
async function loadSettings() {
    document.getElementById('themeMode').value = localStorage.getItem('claudemeow_theme') || 'dark';
    try {
        const profile = await window.__TAURI_INTERNALS__.invoke('get_user_profile');
        userFacts = (profile || '').split('\n').map(s => s.trim()).filter(s => s.length > 0);
        renderFacts();
    } catch(e) {}
    try {
        const config = await window.__TAURI_INTERNALS__.invoke('get_config');
        document.getElementById('providerSelect').value = config.provider || 'bedrock_apikey';
        document.getElementById('region').value = config.aws_region;
        document.getElementById('apiKey').value = config.api_key || '';
        document.getElementById('modelId').value = config.model_id;
        document.getElementById('anthropicApiKey').value = config.api_key || '';
        document.getElementById('anthropicModelId').value = config.anthropic_model_id || 'claude-haiku-4-5-20251001';
        document.getElementById('openaiApiKey').value = config.openai_api_key || '';
        document.getElementById('openaiModelId').value = config.openai_model_id || 'gpt-4o-mini';
        document.getElementById('persona').value = config.persona;
        document.getElementById('interval').value = config.interval_minutes;
        // Load nicknames as chips
        if (config.user_nickname) {
            userNicknames = [config.user_nickname];
        }
        if (config.nicknames && config.nicknames.length > 0) {
            userNicknames = config.nicknames;
        }
        renderNicknames();
        document.getElementById('activityEnabled').checked = config.activity_enabled !== false;
        document.getElementById('devMode').checked = config.dev_mode || false;
        document.getElementById('chattiness').value = config.chattiness || 3;
        document.getElementById('sassLevel').value = config.sass_level || 3;
        document.getElementById('languageMix').value = config.language_mix || 'bilingual';
        document.getElementById('healthEnabled').checked = config.health_enabled !== false;
        document.getElementById('healthInterval').value = config.health_interval_minutes || 45;
        if (document.getElementById('bubbleDuration')) document.getElementById('bubbleDuration').value = config.bubble_duration_secs || 30;
        if (document.getElementById('bubbleDisplayTime')) document.getElementById('bubbleDisplayTime').value = config.bubble_duration_secs || 30;
        document.getElementById('quietStart').value = config.quiet_hours_start !== undefined ? config.quiet_hours_start : 23;
        document.getElementById('quietEnd').value = config.quiet_hours_end !== undefined ? config.quiet_hours_end : 8;
        if (document.getElementById('idleGap')) document.getElementById('idleGap').value = config.idle_gap_minutes || 3;
        if (document.getElementById('reactionCooldown')) document.getElementById('reactionCooldown').value = config.reaction_cooldown_secs || 10;
        updateProviderFields();
        updateStatusRail();
    } catch (e) { console.error('loadSettings:', e); }

    // Load .meow public info — merge as additional (skip dismissed + duplicates)
    try {
        const meow = await window.__TAURI_INTERNALS__.invoke('get_meow_public_info');
        if (meow && meow.loaded) {
            const dismissedNicks = JSON.parse(localStorage.getItem('meow_dismissed_nicks') || '[]');
            const dismissedFacts = JSON.parse(localStorage.getItem('meow_dismissed_facts') || '[]');

            if (meow.nicknames && meow.nicknames.length > 0) {
                meow.nicknames.forEach(n => {
                    if (!userNicknames.includes(n) && !dismissedNicks.includes(n)) userNicknames.push(n);
                });
                renderNicknames();
            }
            if (meow.personality) {
                const meowFacts = meow.personality.split('\n').map(s => s.trim()).filter(s => s.length > 0);
                meowFacts.forEach(f => {
                    const clean = f.replace(/^(A close friend says about this user: |Their friend says they love )/, '');
                    if (clean && !userFacts.includes(clean) && !dismissedFacts.includes(clean)) userFacts.push(clean);
                });
                renderFacts();
            }
        }
    } catch (e) {}
}

function toggleDevWindow(enabled) {
    window.__TAURI_INTERNALS__.invoke(enabled ? 'plugin:window|show' : 'plugin:window|hide', { label: 'dev' });
}

// ── Permissions ──
async function checkPerms() {
    const container = document.getElementById('permStatus');
    container.innerHTML = '<div style="color:var(--text-muted);">Checking...</div>';
    try {
        const perms = await window.__TAURI_INTERNALS__.invoke('check_permissions_status');
        const items = [
            { key: 'accessibility', label: 'Accessibility', fix: 'x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility' },
            { key: 'window_titles', label: 'Window Titles', fix: 'x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility' },
            { key: 'browser_url', label: 'Browser URL', fix: 'x-apple.systempreferences:com.apple.preference.security?Privacy_Automation' },
        ];
        let html = '';
        for (const item of items) {
            const ok = perms[item.key];
            html += '<div class="perm-row">' +
                '<span class="perm-label">' + item.label + '</span>' +
                '<span class="perm-status">' + (ok ? '<span class="perm-granted">● Granted</span>' : '<span class="perm-denied">● Denied</span>') + '</span>' +
                (ok ? '' : '<button class="btn-secondary" onclick="openSettings(\'' + item.fix + '\')">Fix</button>') +
            '</div>';
        }
        container.innerHTML = html;
    } catch(e) { container.innerHTML = '<div style="color:var(--error);">' + e + '</div>'; }
}
function openSettings(url) { window.__TAURI_INTERNALS__.invoke('open_system_settings', { url }); }

// ── Modules ──
async function loadModules() {
    try {
        const modules = await window.__TAURI_INTERNALS__.invoke('get_modules');
        renderModules(modules);
    } catch(e) { console.error('loadModules:', e); }
}

function renderModules(modules) {
    const container = document.getElementById('moduleList');
    container.textContent = '';
    const railMod = document.getElementById('railModules');
    if (railMod && modules) {
        const active = modules.filter(m => m.status === 'active').length;
        railMod.textContent = active + ' active / ' + modules.length + ' total';
    }
    if (!modules || modules.length === 0) {
        container.innerHTML = '<div class="ext-card" style="justify-content:center;opacity:0.5;"><div class="ext-desc">No modules found</div></div>';
        return;
    }
    for (const m of modules) {
        const card = document.createElement('div');
        card.className = 'ext-card';
        card.innerHTML = '<div class="ext-icon">' + (m.icon || '📦') + '</div>' +
            '<div class="ext-info"><div class="ext-name">' + escHtml(m.name) + ' <span style="font-size:10px;color:var(--text-muted);font-weight:400;">v' + m.version + '</span></div>' +
            '<div class="ext-desc">' + escHtml(m.description || '') + ' · ' + m.reaction_count + ' reactions</div>' +
            (m.error ? '<div style="font-size:10px;color:var(--error);margin-top:2px;">' + escHtml(m.error) + '</div>' : '') + '</div>';
        if (m.status === 'error') {
            card.innerHTML += '<span class="ext-badge" style="background:var(--accent-subtle);color:var(--error);">Error</span>';
        } else {
            const lbl = document.createElement('label');
            lbl.className = 'toggle'; lbl.style.margin = '0';
            const chk = document.createElement('input');
            chk.type = 'checkbox'; chk.checked = m.status === 'active';
            chk.addEventListener('change', () => toggleMod(m.id, chk.checked));
            lbl.appendChild(chk); card.appendChild(lbl);
        }
        container.appendChild(card);
    }
}

async function refreshModules() {
    const btn = document.getElementById('refreshBtn');
    btn.textContent = '...'; btn.disabled = true;
    try {
        renderModules(await window.__TAURI_INTERNALS__.invoke('refresh_modules'));
        loadPriorities();
    }
    catch(e) { console.error(e); }
    btn.textContent = 'Refresh'; btn.disabled = false;
}
async function toggleMod(id, enabled) {
    try { await window.__TAURI_INTERNALS__.invoke('toggle_module', { id, enabled }); } catch(e) {}
}
function showCreateForm() { document.getElementById('createModuleForm').style.display = ''; }
function hideCreateForm() { document.getElementById('createModuleForm').style.display = 'none'; }
function updateConditionFields() {
    const event = document.getElementById('newModEvent').value;
    const fieldSel = document.getElementById('newModCondField');
    while (fieldSel.options.length) fieldSel.remove(0);
    const opts = event === 'browser_url_changed' ? [['site','Site'],['title','URL']] : [['app_name','App Name'],['window_title','Window Title']];
    opts.forEach(([val, label]) => fieldSel.add(new Option(label, val)));
}
async function saveNewModule() {
    const statusEl = document.getElementById('createStatus');
    const name = document.getElementById('newModName').value.trim();
    const icon = document.getElementById('newModIcon').value.trim() || '📦';
    const description = document.getElementById('newModDesc').value.trim();
    const event = document.getElementById('newModEvent').value;
    const conditionField = document.getElementById('newModCondField').value;
    const conditionType = document.getElementById('newModCondType').value;
    const conditionValue = document.getElementById('newModCondVal').value.trim();
    const messages = document.getElementById('newModMessages').value.split('\n').map(m => m.trim()).filter(m => m.length > 0);
    const priority = parseInt(document.getElementById('newModPriority').value) || 5;
    const cooldownMinutes = parseInt(document.getElementById('newModCooldown').value) || 10;
    if (!name) { statusEl.textContent = 'Name required'; statusEl.className = 'test-status error'; return; }
    if (messages.length === 0) { statusEl.textContent = 'At least one message required'; statusEl.className = 'test-status error'; return; }
    const id = name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
    if (!id) { statusEl.textContent = 'Invalid name'; statusEl.className = 'test-status error'; return; }
    statusEl.textContent = 'Saving...'; statusEl.className = 'test-status loading';
    try {
        const modules = await window.__TAURI_INTERNALS__.invoke('create_module', { id, name, icon, description, event, conditionField, conditionType, conditionValue, messages, priority, cooldownMinutes });
        renderModules(modules); hideCreateForm();
        ['newModName','newModIcon','newModDesc','newModCondVal','newModMessages'].forEach(fid => document.getElementById(fid).value = '');
    } catch(e) { statusEl.textContent = 'Error: ' + String(e).slice(0, 80); statusEl.className = 'test-status error'; }
}

// ── Priority ──
let priorityData = {};
const DEFAULT_LEVELS = { health: 1, 'amazon-internal': 1, coding: 2, zoom: 2, slack: 2, browsing: 3, entertainment: 3, weather: 3, memes: 4 };

async function loadPriorities() {
    try {
        const modules = await window.__TAURI_INTERNALS__.invoke('get_modules');
        const overrides = await window.__TAURI_INTERNALS__.invoke('get_priority_overrides');
        priorityData = {};
        for (const m of modules) {
            const def = DEFAULT_LEVELS[m.id] || 3;
            priorityData[m.id] = { name: m.name, level: overrides[m.id] || def, default: def };
        }
        renderPriorities();
    } catch(e) { console.error(e); }
}
function renderPriorities() {
    const container = document.getElementById('priorityList');
    const sorted = Object.entries(priorityData).sort((a, b) => a[1].level - b[1].level);
    container.innerHTML = sorted.map(([id, data]) =>
        '<div class="priority-row"><div class="priority-name">' + escHtml(data.name) + '</div><div class="priority-levels">' +
        [1,2,3,4].map(l => '<span class="plvl plvl-' + l + (data.level === l ? ' active' : '') + '" onclick="setLevel(\'' + id + '\',' + l + ')">' + l + '</span>').join('') +
        '</div></div>'
    ).join('');
}
function setLevel(id, level) { priorityData[id].level = level; renderPriorities(); }
async function savePriorities() {
    const overrides = {};
    for (const [id, data] of Object.entries(priorityData)) overrides[id] = data.level;
    try {
        await window.__TAURI_INTERNALS__.invoke('set_priority_overrides', { overrides });
        document.getElementById('priorityStatus').textContent = 'Saved!';
        document.getElementById('priorityStatus').className = 'test-status success';
        setTimeout(() => document.getElementById('priorityStatus').textContent = '', 2000);
    } catch(e) { document.getElementById('priorityStatus').textContent = 'Error: ' + e; document.getElementById('priorityStatus').className = 'test-status error'; }
}
async function resetPriorities() {
    for (const [id, data] of Object.entries(priorityData)) data.level = data.default;
    renderPriorities();
    await window.__TAURI_INTERNALS__.invoke('set_priority_overrides', { overrides: {} });
    document.getElementById('priorityStatus').textContent = 'Reset';
    setTimeout(() => document.getElementById('priorityStatus').textContent = '', 2000);
}

// ── Amazon ──
const MCP_ID = 'amazon-internal';
async function mcpCall(command, args) {
    return await window.__TAURI_INTERNALS__.invoke('call_module_mcp', { moduleId: MCP_ID, command, args });
}

async function loadAmazonConfig() {
    try {
        const present = await window.__TAURI_INTERNALS__.invoke('is_mcp_module_present', { moduleId: MCP_ID });
        if (!present) return;
        document.getElementById('amazonNav').style.display = '';
        const cfg = await window.__TAURI_INTERNALS__.invoke('get_module_mcp_config', { moduleId: MCP_ID });
        if (cfg) {
            document.getElementById('amazonAlias').value = cfg.user_alias || '';
            document.getElementById('amazonCrEnabled').checked = cfg.cr_review_enabled || false;
            document.getElementById('amazonCalendarEnabled').checked = cfg.calendar_enabled || false;
            document.getElementById('amazonCrInterval').value = cfg.cr_interval_hours || '2';
            document.getElementById('amazonCrReset').value = cfg.cr_reset_days || '3';
            document.getElementById('amazonCrDays').value = cfg.cr_days_range || '14';
        }
        ['amazonCrEnabled','amazonCalendarEnabled','amazonCrInterval','amazonCrReset','amazonCrDays'].forEach(id => {
            document.getElementById(id).addEventListener('change', debounceAmazonSave);
        });
        try {
            const team = await mcpCall('get-team', ['--alias', cfg.user_alias || '']);
            if (team && team.teammates && team.teammates.length > 0) {
                renderTeamList(team);
                document.getElementById('amazonInitBtn').style.display = 'none';
                document.getElementById('amazonTestBtn').style.display = '';
            }
        } catch(e) {}
    } catch(e) {}
}

let amazonSaveTimer = null;
function debounceAmazonSave() { clearTimeout(amazonSaveTimer); amazonSaveTimer = setTimeout(saveAmazonConfig, 500); }

async function saveAmazonConfig() {
    const alias = document.getElementById('amazonAlias').value.trim();
    try {
        await window.__TAURI_INTERNALS__.invoke('save_module_mcp_config', {
            moduleId: MCP_ID,
            config: {
                user_alias: alias,
                calendar_enabled: document.getElementById('amazonCalendarEnabled').checked,
                cr_review_enabled: document.getElementById('amazonCrEnabled').checked,
                cr_interval_hours: parseInt(document.getElementById('amazonCrInterval').value),
                cr_reset_days: parseInt(document.getElementById('amazonCrReset').value),
                cr_days_range: parseInt(document.getElementById('amazonCrDays').value),
            }
        });
        showSaved();
    } catch(e) { console.error('saveAmazonConfig:', e); }
}

async function initAmazon() {
    const el = document.getElementById('amazonTestStatus');
    const alias = document.getElementById('amazonAlias').value.trim();
    if (!alias) { el.innerHTML = '<span style="color:var(--error);">Enter alias first</span>'; return; }
    el.innerHTML = '<span style="color:var(--text-muted);">Initializing...</span>';
    try {
        await window.__TAURI_INTERNALS__.invoke('save_module_mcp_config', {
            moduleId: MCP_ID, config: { user_alias: alias, cr_review_enabled: true, calendar_enabled: true, cr_interval_hours: 2, cr_reset_days: 3, cr_days_range: 14 }
        });
        const status = await mcpCall('test-connection', ['--alias', alias]);
        const team = await mcpCall('get-team', ['--alias', alias]);
        renderTeamList(team);
        el.innerHTML = '<span style="color:var(--success);">✓ Initialized</span>';
        document.getElementById('amazonInitBtn').style.display = 'none';
        document.getElementById('amazonTestBtn').style.display = '';
    } catch(e) { el.innerHTML = '<span style="color:var(--error);">' + e + '</span>'; }
}

async function testAmazonConnection() {
    const el = document.getElementById('amazonTestStatus');
    el.innerHTML = '<span style="color:var(--text-muted);">Testing...</span>';
    try {
        const alias = document.getElementById('amazonAlias').value.trim();
        if (!alias) { el.innerHTML = '<span style="color:var(--error);">Enter alias</span>'; return; }
        const status = await mcpCall('test-connection', ['--alias', alias]);
        el.innerHTML = '<div style="margin-top:6px;font-size:11px;">' +
            Object.entries(status).filter(([k]) => typeof status[k] === 'boolean').map(([k, v]) =>
                '<span style="margin-right:10px;color:' + (v ? 'var(--green)' : 'var(--red)') + ';font-weight:500;">' + (v ? '✓' : '✗') + ' ' + k.replace(/_/g, ' ') + '</span>'
            ).join('') + '</div>';
        const team = await mcpCall('get-team', ['--alias', alias]);
        renderTeamList(team);
    } catch(e) { el.innerHTML = '<span style="color:var(--error);">' + e + '</span>'; }
}

function renderTeamList(team) {
    const teamEl = document.getElementById('amazonTeamList');
    if (!team || !team.teammates || team.teammates.length === 0) {
        teamEl.innerHTML = '<span style="color:var(--text-muted);">No team data</span>';
        return;
    }
    const nicknames = JSON.parse(localStorage.getItem('amazonNicknames') || '{}');
    const names = team.teammate_names || {};
    let html = '<div style="font-size:12px;color:var(--text-secondary);margin-bottom:12px;">Manager: <strong>' + escHtml(team.manager_name || team.manager || '?') + '</strong></div>';
    html += '<table class="team-table"><thead><tr><th>Alias</th><th>Name</th><th>Nickname</th><th></th></tr></thead><tbody>';
    team.teammates.forEach(t => {
        html += '<tr><td class="alias alias-link" onclick="openTeammateProfile(\'' + escHtml(t) + '\')">' + escHtml(t) + '</td><td class="name">' + escHtml(names[t] || '') + '</td>' +
            '<td><input type="text" class="nick-input" data-alias="' + t + '" value="' + escHtml(nicknames[t] || '') + '" placeholder="..."></td>' +
            '<td><button class="btn-judge" onclick="judgeTeammate(\'' + escHtml(t) + '\', this)">Judge</button></td></tr>';
    });
    html += '</tbody></table>';
    teamEl.innerHTML = html;
    teamEl.querySelectorAll('.nick-input').forEach(el => el.addEventListener('change', saveNicknames));
}

async function openTeammateProfile(alias) {
    try {
        const cfg = await window.__TAURI_INTERNALS__.invoke('get_module_mcp_config', { moduleId: MCP_ID });
        const base = cfg.profile_url_base || '';
        if (base) {
            window.__TAURI_INTERNALS__.invoke('open_external_url', { url: base + alias });
        }
    } catch(e) {}
}

let judgeInProgress = false;
async function judgeTeammate(alias, btnEl) {
    if (judgeInProgress) return;
    judgeInProgress = true;

    // Disable all judge buttons and animate the clicked one
    document.querySelectorAll('.btn-judge').forEach(b => { b.disabled = true; });
    if (btnEl) { btnEl.classList.add('judging'); btnEl.innerHTML = '<span class="judge-dots"></span>'; }

    try {
        const msg = await window.__TAURI_INTERNALS__.invoke('trigger_targeted_gossip', { targetAlias: alias });
        if (btnEl) { btnEl.classList.remove('judging'); btnEl.classList.add('judge-success'); btnEl.textContent = 'Done'; }
        setTimeout(() => {
            document.querySelectorAll('.btn-judge').forEach(b => { b.disabled = false; b.textContent = 'Judge'; b.classList.remove('judging', 'judge-success'); });
            judgeInProgress = false;
        }, 3000);
    } catch(e) {
        if (btnEl) {
            btnEl.classList.remove('judging');
            btnEl.classList.add('judge-fail');
            btnEl.textContent = 'No data';
        }
        setTimeout(() => {
            document.querySelectorAll('.btn-judge').forEach(b => { b.disabled = false; b.textContent = 'Judge'; b.classList.remove('judging', 'judge-fail', 'judge-success'); });
            judgeInProgress = false;
        }, 3000);
    }
}

function saveNicknames() {
    const nicknames = {};
    document.querySelectorAll('.nick-input').forEach(el => {
        const nick = el.value.trim();
        if (nick) nicknames[el.dataset.alias] = nick;
    });
    localStorage.setItem('amazonNicknames', JSON.stringify(nicknames));
    window.__TAURI_INTERNALS__.invoke('get_module_mcp_config', { moduleId: MCP_ID }).then(mcfg => {
        mcfg.nicknames = nicknames;
        window.__TAURI_INTERNALS__.invoke('save_module_mcp_config', { moduleId: MCP_ID, config: mcfg });
    });
}

// ── SecretMeow ──
async function scanMeow() {
    const container = document.getElementById('meowResult');
    container.innerHTML = '<div style="color:var(--text-muted);">Scanning...</div>';
    try {
        const result = await window.__TAURI_INTERNALS__.invoke('scan_secret_meow');
        if (result.found) {
            container.innerHTML = '<div style="padding:12px;background:rgba(90,158,111,0.08);border:1px solid rgba(90,158,111,0.15);border-radius:8px;">' +
                '<div style="font-size:14px;font-weight:600;color:var(--success);margin-bottom:6px;">' + (result.from_littleshrimp ? '🦐 ' : '🐱 ') + escHtml(result.message) + '</div>' +
                '<div style="font-size:11px;color:var(--text-muted);">Files: ' + result.files.map(f => '<code>' + escHtml(f) + '</code>').join(', ') + '</div></div>';

            // Reload .meow and refresh UI with public fields
            try {
                await window.__TAURI_INTERNALS__.invoke('reload_secret_meow');
                const meow = await window.__TAURI_INTERNALS__.invoke('get_meow_public_info');
                if (meow && meow.loaded) {
                    if (meow.nicknames && meow.nicknames.length > 0) {
                        userNicknames = meow.nicknames;
                        renderNicknames();
                    }
                    if (meow.personality) {
                        const meowFacts = meow.personality.split('\n').map(s => s.trim()).filter(s => s.length > 0);
                        meowFacts.forEach(f => {
                            const clean = f.replace(/^(A close friend says about this user: |Their friend says they love )/, '');
                            if (clean && !userFacts.includes(clean)) userFacts.push(clean);
                        });
                        renderFacts();
                    }
                }
            } catch(e2) {}
        } else {
            container.innerHTML = '<div style="padding:12px;background:rgba(0,0,0,0.03);border-radius:8px;color:var(--text-muted);">No .meow files found yet</div>';
        }
    } catch(e) { container.innerHTML = '<div style="color:var(--error);">' + e + '</div>'; }
}

// ── Updates ──
async function checkForUpdate() {
    const status = document.getElementById('updateStatus');
    const btn = document.getElementById('updateBtn');
    btn.textContent = 'Checking...'; btn.disabled = true;
    try {
        const result = await window.__TAURI_INTERNALS__.invoke('plugin:updater|check');
        if (result && result.available) {
            status.innerHTML = '<div style="margin-top:8px;padding:10px 14px;background:rgba(90,158,111,0.08);border-radius:8px;display:flex;align-items:center;justify-content:space-between;">' +
                '<div><div style="font-weight:600;color:var(--success);">v' + result.version + ' available</div></div>' +
                '<button class="btn-primary" onclick="installUpdate()">Update</button></div>';
        }
    } catch(e) {
        if (!String(e).includes('valid release')) {
            status.innerHTML = '<div style="margin-top:6px;font-size:11px;color:var(--text-muted);">Unable to check</div>';
        }
    }
    btn.textContent = 'Check'; btn.disabled = false;
}
async function installUpdate() {
    const status = document.getElementById('updateStatus');
    status.innerHTML = '<div style="margin-top:8px;padding:10px;border-radius:8px;background:var(--accent-subtle);color:var(--accent);">Installing...</div>';
    try {
        await window.__TAURI_INTERNALS__.invoke('plugin:updater|download_and_install');
        await window.__TAURI_INTERNALS__.invoke('plugin:process|restart');
    } catch(e) { status.innerHTML = '<div style="color:var(--error);">Failed: ' + String(e).slice(0,60) + '</div>'; }
}

// ── Helpers ──
function escHtml(t) { if (!t) return ''; const d = document.createElement('div'); d.textContent = t; return d.innerHTML; }

// ── Status Rail Updates ──
function updateStatusRail() {
    const provider = document.getElementById('providerSelect').value;
    const providerNames = { bedrock_apikey: 'Bedrock', bedrock_iam: 'Bedrock (IAM)', anthropic: 'Anthropic', openai: 'OpenAI' };
    const el = (id) => document.getElementById(id);
    if (el('railProvider')) el('railProvider').innerHTML = '<strong>' + (providerNames[provider] || provider) + '</strong>';

    const modelSel = provider === 'anthropic' ? 'anthropicModelId' : provider === 'openai' ? 'openaiModelId' : 'modelId';
    const modelEl = document.getElementById(modelSel);
    if (el('railModel') && modelEl) {
        const txt = modelEl.options[modelEl.selectedIndex]?.text || '';
        el('railModel').textContent = txt.replace(/\s*\(.*\)/, '');
    }
    if (el('railInterval')) el('railInterval').textContent = document.getElementById('interval').value + ' min';

    const hasKey = provider === 'bedrock_iam' || (provider === 'anthropic' ? document.getElementById('anthropicApiKey').value : provider === 'openai' ? document.getElementById('openaiApiKey').value : document.getElementById('apiKey').value);
    const tag = el('railConnectionTag');
    if (tag) {
        if (hasKey) { tag.textContent = 'Configured'; tag.className = 'rail-tag'; }
        else { tag.textContent = 'Local'; tag.className = 'rail-tag inactive'; }
    }
}

// ── System Stats ──
async function updateSystemStats() {
    try {
        const stats = await window.__TAURI_INTERNALS__.invoke('get_system_stats');
        const el = (id) => document.getElementById(id);
        if (el('railMemory')) el('railMemory').textContent = stats.memory || '—';
        if (el('railCpu')) el('railCpu').textContent = stats.cpu || '—';
        if (el('railUptime')) el('railUptime').textContent = stats.uptime || '—';
        if (el('railMode')) el('railMode').textContent = stats.cpu === '—' ? 'Offline' : 'Running';
    } catch (e) {}
}
updateSystemStats();
setInterval(updateSystemStats, 10000);

// ── Quick Actions ──
function resetConversation() {
    if (!confirm('Reset conversation history?')) return;
    window.__TAURI_INTERNALS__.invoke('clear_chat_history').catch(() => {});
}
function clearMemory() {
    if (!confirm('Clear all local memory and activity logs?')) return;
    window.__TAURI_INTERNALS__.invoke('clear_activity_log').catch(() => {});
}
function exportLogs() {
    window.__TAURI_INTERNALS__.invoke('export_logs').catch(() => {});
}
function openDataFolder() {
    window.__TAURI_INTERNALS__.invoke('open_data_folder').catch(() => {});
}
function forceSave() {
    clearTimeout(saveTimer);
    autoSave();
}

// ── Bottom Strip ──
function resetToDefaults() {
    if (!confirm('Reset all settings to defaults?')) return;
    const defaults = {
        provider: 'bedrock_apikey', aws_region: 'us-east-1',
        model_id: 'us.anthropic.claude-haiku-4-5-20251001-v1:0',
        anthropic_model_id: 'claude-haiku-4-5-20251001', openai_model_id: 'gpt-4o-mini',
        openai_api_key: '', persona: '', interval_minutes: 5, nicknames: [],
        activity_enabled: true, dev_mode: false, auth_mode: 'apikey', api_key: '',
        user_nickname: '', chattiness: 3, sass_level: 3, language_mix: 'bilingual',
        health_enabled: true, health_interval_minutes: 45, bubble_duration_secs: 30,
        quiet_hours_start: 23, quiet_hours_end: 8,
    };
    window.__TAURI_INTERNALS__.invoke('set_config', { config: defaults }).then(() => {
        loadSettings();
        const el = document.getElementById('stripSaveStatus');
        if (el) { el.textContent = 'RESET'; setTimeout(() => el.textContent = '', 2000); }
    }).catch(() => {});
}

// ── CR Intelligence ──
async function loadCrIntelligence() {
    const panel = document.getElementById('crIntelPanel');
    if (!panel) return;
    try {
        const [stats, history] = await Promise.all([
            window.__TAURI_INTERNALS__.invoke('get_team_stats').catch(() => null),
            window.__TAURI_INTERNALS__.invoke('get_gossip_history').catch(() => ({ week_start: '', commented_ids: [] })),
        ]);

        let html = '';

        if (stats && stats.teammates) {
            const updated = stats.last_updated ? new Date(stats.last_updated).toLocaleString() : '—';
            html += '<div class="cr-intel-header">Team Stats <span style="font-weight:400;color:var(--text-muted);">· Updated ' + escHtml(updated) + '</span></div>';
            html += '<div class="cr-intel-grid">';
            const entries = Object.entries(stats.teammates).sort((a, b) => (b[1].total_crs || 0) - (a[1].total_crs || 0));
            for (const [alias, s] of entries) {
                const name = s.nickname || s.display_name || alias;
                html += '<div class="cr-intel-card">' +
                    '<div class="cr-intel-name">' + escHtml(name) + '</div>' +
                    '<div class="cr-intel-metrics">' +
                    '<span class="cr-metric"><strong>' + (s.total_crs || 0) + '</strong> CRs</span>' +
                    '<span class="cr-metric"><strong>' + (s.total_commits || 0) + '</strong> commits</span>' +
                    '<span class="cr-metric"><strong>' + (s.streak_days || 0) + '</strong>d streak</span>' +
                    '</div>' +
                    (s.last_cr_title ? '<div class="cr-intel-last">Last: ' + escHtml(s.last_cr_title.slice(0, 40)) + '</div>' : '') +
                    '</div>';
            }
            html += '</div>';

            if (stats.comparisons) {
                html += '<div class="cr-intel-comparisons">';
                if (stats.comparisons.most_crs) html += '<span>Most active: <strong>' + escHtml(stats.comparisons.most_crs) + '</strong></span>';
                if (stats.comparisons.quietest) html += '<span>Quietest: <strong>' + escHtml(stats.comparisons.quietest) + '</strong></span>';
                html += '</div>';
            }
        }

        if (history) {
            const ids = history.commented_ids || [];
            html += '<div class="cr-intel-header" style="margin-top:14px;">Gossip Memory <span style="font-weight:400;color:var(--text-muted);">· ' + ids.length + ' entries · resets ' + escHtml(history.week_start || 'next period') + '</span></div>';
            if (ids.length > 0) {
                html += '<div class="cr-intel-history">';
                ids.slice(-5).reverse().forEach(id => {
                    const parts = id.split(':');
                    html += '<div class="cr-history-item"><span class="cr-history-author">' + escHtml(parts[0] || '') + '</span> ' + escHtml(parts.slice(1).join(':').slice(0, 35)) + '</div>';
                });
                html += '</div>';
                if (ids.length > 5) {
                    html += '<div style="margin-top:6px;"><button class="btn-secondary" onclick="openDataFolder()" style="font-size:9px;height:20px;padding:0 6px;">View All in Finder</button></div>';
                }
            }
        }

        panel.innerHTML = html || '<span style="color:var(--text-muted);">No data yet. Click Refresh.</span>';
    } catch(e) {
        panel.innerHTML = '<span style="color:var(--text-muted);">Failed to load: ' + e + '</span>';
    }
}

async function refreshCrStats() {
    const panel = document.getElementById('crIntelPanel');
    if (panel) panel.innerHTML = '<span style="color:var(--accent);">Refreshing...</span>';
    try {
        await window.__TAURI_INTERNALS__.invoke('refresh_team_stats');
        await loadCrIntelligence();
    } catch(e) {
        if (panel) panel.innerHTML = '<span style="color:var(--red);">' + e + '</span>';
    }
}

async function clearGossipMemory() {
    if (!confirm('Clear all gossip memory?')) return;
    try {
        await window.__TAURI_INTERNALS__.invoke('clear_gossip_memory');
        await loadCrIntelligence();
    } catch(e) {}
}

// ── Pet Status ──
const asciiCats = {
    happy:   "  /\\_/\\\n ( ^.^ )\n  > ^ <\n /|   |\\\n(_|   |_)",
    love:    "  /\\_/\\\n ( ♥.♥ )\n  > ^ <\n /|♥ ♥|\\\n(_|   |_)",
    sad:     "  /\\_/\\\n ( ;.; )\n  > ~ <\n /|   |\\\n(_|___|_)",
    sleepy:  "  /\\_/\\\n ( -.- )zzz\n  > ~ <\n /|   |\\\n(_|___|_)",
    hungry:  "  /\\_/\\\n ( >`<´)\n  >!/ \\!<\n /|   |\\\n(_|   |_)",
    neutral: "  /\\_/\\\n ( o.o )\n  > ^ <\n /|   |\\\n(_|   |_)",
};

function updatePetDisplay(s) {
    if (!s) return;
    const setBar = (id, val) => { const el = document.getElementById(id); if (el) el.style.width = val + '%'; };
    const setVal = (id, val) => { const el = document.getElementById(id); if (el) el.textContent = val; };
    setBar('statHappiness', s.happiness); setVal('statHappinessVal', s.happiness);
    setBar('statEnergy', s.energy); setVal('statEnergyVal', s.energy);
    setBar('statHunger', s.hunger); setVal('statHungerVal', s.hunger);
    setBar('statLove', s.love); setVal('statLoveVal', s.love);

    const mood = s.mood || 'neutral';
    const catEl = document.getElementById('asciiCat');
    const moodEl = document.getElementById('asciiCatMood');
    if (catEl) catEl.textContent = asciiCats[mood] || asciiCats.neutral;
    if (moodEl) moodEl.textContent = mood;
}

async function loadPetStatus() {
    try {
        const s = await window.__TAURI_INTERNALS__.invoke('get_pet_status');
        updatePetDisplay(s);
    } catch(e) {}
}

// Cat blink animation
setInterval(() => {
    const catEl = document.getElementById('asciiCat');
    if (!catEl || !catEl.textContent.includes('(')) return;
    const original = catEl.textContent;
    const blinked = original.replace(/\( .\..\)/g, '( -.- )').replace(/\( ♥\.♥ \)/g, '( -.- )');
    if (blinked !== original) {
        catEl.textContent = blinked;
        setTimeout(() => { catEl.textContent = original; }, 200);
    }
}, 4000);

let activityExpanded = false;
let activityEntries = [];

async function loadActivityHistory() {
    try {
        const entries = await window.__TAURI_INTERNALS__.invoke('get_activity_log', { limit: 20 });
        activityEntries = (entries || []).reverse();
        renderActivityHistory();
    } catch(e) {
        const container = document.getElementById('activityHistory');
        if (container) container.innerHTML = '<div style="color:var(--text-muted);font-size:11px;padding:8px;">No activity yet</div>';
    }
}

function renderActivityHistory() {
    const container = document.getElementById('activityHistory');
    const btn = document.getElementById('activityToggleBtn');
    if (!container) return;
    if (activityEntries.length === 0) {
        container.innerHTML = '<div style="color:var(--text-muted);font-size:11px;padding:8px;">No activity yet</div>';
        if (btn) btn.style.display = 'none';
        return;
    }
    const shown = activityExpanded ? activityEntries : activityEntries.slice(0, 5);
    let html = '';
    shown.forEach(e => {
        const time = new Date(e.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        const delta = e.delta ? (e.delta > 0 ? '+' + e.delta : '' + e.delta) : '';
        const deltaClass = e.delta > 0 ? 'positive' : e.delta < 0 ? 'negative' : '';
        html += '<div class="activity-item">' +
            '<span class="activity-time">' + time + '</span>' +
            '<span class="activity-source">' + escHtml(e.source || '') + '</span>' +
            '<span class="activity-detail">' + escHtml(e.detail || e.event || '') + '</span>' +
            (delta ? '<span class="activity-delta ' + deltaClass + '">' + delta + '</span>' : '') +
            '</div>';
    });
    container.innerHTML = html;
    if (btn) {
        btn.style.display = activityEntries.length > 5 ? '' : 'none';
        btn.textContent = activityExpanded ? 'Collapse' : 'Show All';
    }
}

function toggleActivityHistory() {
    activityExpanded = !activityExpanded;
    renderActivityHistory();
}

async function petAction(cmd) {
    try {
        const s = await window.__TAURI_INTERNALS__.invoke(cmd);
        updatePetDisplay(s);
    } catch(e) { console.error('petAction:', e); }
}

// ── Init ──
loadSettings();
loadModules();
loadPriorities();
loadAmazonConfig();
loadCrIntelligence();
loadPetStatus();
loadActivityHistory();
