import { invoke, listen, escapeHtml } from './ipc.js';

// ── Helpers ──
let nicknames = ['人类', 'hooman', '铲屎官', '主人', 'buddy'];
let userNickname = '';
let secretMessages = [];
const kaomojis = [
    '^ↀᴥↀ^', 'ฅ•ω•ฅ', '(ฅ\'ω\'ฅ)', '(=ↀωↀ=)', '=^∇^*=',
    '(^･ｪ･^)', '(=^･^=)', '(^・ω・^ )', '(=^-ω-^=)', '(^=◕ᴥ◕=^)',
    '(＾• ω •＾)', 'ฅ(•ㅅ•❀)ฅ', '(=^ ◡ ^=)', '(=^･ω･^=)',
    '<(*ΦωΦ*)>', '(^˵◕ω◕˵^)', '~(=^‥^)/', 'ଲ(ⓛ ω ⓛ)ଲ',
];
function pickNick() {
    const pool = nicknames.length > 0 ? nicknames : (userNickname ? [userNickname] : ['hooman']);
    return pool[Math.floor(Math.random() * pool.length)];
}
function fillNick(msg) { return msg.replace(/\{n\}/g, pickNick()).replace(/\{k\}/g, pickKao()); }
function pick(arr) { return arr[Math.floor(Math.random() * arr.length)]; }
function pickKao() { return pick(kaomojis); }

// ── Message pools ──
const tapMessages = [
    '喵？🐱', '嗯？', '哎～', '呜～', 'meow~',
    '嘻嘻～😸', '喵呜!', '♪～', '哼!', 'nya~',
    '摸摸～🐾', '嗷呜?', '...💤', '嘿!', '~喵',
];
const pokeMessages = [
    '别碰我！😾', '哎呀！别戳{n}！🐾', '喵！！疼！😿',
    '又戳！不理你了！💢', '别摸了别摸了！🙀',
    '哼～再碰咬你哦！😤', '手拿开！是{n}的毛！',
    '喵呜～{n}其实喜欢～😽', '再碰就告诉Claude！🤖',
    '你好烦！但{n}原谅你～', '讨厌！才不疼呢…😿',
    '呜呜～{n}被欺负了！', '轻点！{n}很娇贵的！✨',
    '啊！{n}的尾巴！🙀', '你完了！{n}记仇的！📝',
    '{n}不是玩具！哼！', '痒痒～哈哈别挠！😹',
    '好啦好啦～摸一下而已嘛', '喵喵喵！投降！🏳️',
];
const fallbackMessages = [
    'HI! {n}来啦~ {k}', '{n}, 坐直啦！{k}', '{n}, drink water~ {k}',
    '{n}, 别喝Monster！🚫', '{n}, 加油鸭～{k}', '{n}, 摸鱼吗？🐟',
    '{n}, 该休息了～{k}', '{n}, Meow! {k}', '{n}, 别驼背！{k}',
    '{n}, Monster放下！喝水！💧', '{n}, 又偷喝Monster了？😾',
    '{n}, 喝水喝水喝水！💧{k}',
];
const activityFallbacks = {
    coding: ['{n}, 写代码呢！加油～💪', '{n}, bug写完了吗？😸', '{n}, 代码写累了喝口水～', '{n}, coding好认真！🐱'],
    social: ['{n}, 又在摸鱼！🐟', '{n}, 我看到你在刷手机～', '{n}, 别看了！回来工作！😾', '{n}, 摸鱼被我抓到了！'],
    communication: ['{n}, 在聊天呢！为什么不找我！😿', '{n}, Slack里打字呢～', '{n}, 跟谁聊呢？我也要！', '{n}, 开会辛苦了～🐱'],
    browsing: ['{n}, 在看什么呢？👀', '{n}, 上网冲浪中～🏄', '{n}, 浏览器开了好多tab吧！', '{n}, 别逛了来陪我！'],
    typing: ['{n}, 打字好快！🐾', '{n}, 噼里啪啦打字中～', '{n}, 手指不累吗？休息下～', '{n}, 在写什么呀？好好奇！'],
};

function getActivityFallback(context) {
    const lower = (context || '').toLowerCase();
    let pool = fallbackMessages;
    if (lower.includes('[typing]')) pool = activityFallbacks.typing;
    else if (lower.includes('coding')) pool = activityFallbacks.coding;
    else if (lower.includes('social') || lower.includes('slacking')) pool = activityFallbacks.social;
    else if (lower.includes('communication')) pool = activityFallbacks.communication;
    else if (lower.includes('browsing')) pool = activityFallbacks.browsing;
    return fillNick(pick(pool));
}

// ── Animation ──
const petEl = document.getElementById('pet');
const anims = ['bounce', 'shake', 'wiggle'];
function playAnim() {
    const a = pick(anims);
    petEl.classList.remove(...anims);
    void petEl.offsetWidth;
    petEl.classList.add(a);
    setTimeout(() => petEl.classList.remove(a), 700);
}

// ── DOM refs ──
const bubbleEl = document.getElementById('bubble');
const chatWrapper = document.getElementById('chatWrapper');
const chatInput = document.getElementById('chatInput');
const floatContainer = document.getElementById('floatNumbers');

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MESSAGE SCHEDULER — priority-based channels
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
const scheduler = {
    channel: null,
    busyUntil: 0,
    timers: {},
    modules: [],

    clearTimers() {
        Object.values(this.timers).forEach(clearTimeout);
        this.timers = {};
    },

    isBusy() {
        return Date.now() < this.busyUntil;
    },

    show(text, opts) {
        const cleanText = text.replace(/\n/g, ' ').trim();
        // Convert **bold** and *italic* markdown to HTML
        const htmlText = cleanText
            .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
            .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
            .replace(/\*([^*]+)\*/g, '<em>$1</em>');
        const cls = (opts && opts.cls) ? ' ' + opts.cls : '';
        const inner = document.createElement('div');
        inner.className = 'pixel-bubble-inner';
        inner.innerHTML = htmlText;
        const bubble = document.createElement('div');
        bubble.className = 'pixel-bubble' + cls;
        bubble.appendChild(inner);
        bubbleEl.innerHTML = '';
        bubbleEl.appendChild(bubble);
        bubbleEl.classList.remove('pop');
        bubbleEl.classList.add('show');
        if (opts && opts.animate) {
            void bubbleEl.offsetWidth;
            bubbleEl.classList.add('pop');
        }
    },

    hide() {
        bubbleEl.classList.remove('show', 'pop');
        this.channel = null;
        this.busyUntil = Date.now() + 2000;
    },

    onTouch(text) {
        this.clearTimers();
        this.channel = 'touch';
        this.show(text, { animate: true });
        invoke('emit_dev_log', { tag: 'BUBBLE', tagClass: 'reaction', message: '[touch] "' + text + '"' }).catch(() => {});
        invoke('emit_cat_status', { status: '😸 开心地咕噜咕噜...' }).catch(() => {});
        this.timers.hide = setTimeout(() => {
            this.hide();
            this.timers.next = setTimeout(() => this.nextIdle(), this.idleGapMs || 180000);
        }, 5000);
    },

    onChat(text) {
        this.clearTimers();
        this.channel = 'chat';
        const actionMatch = text.match(/\*([^*]+)\*/);
        let cleaned = text.replace(/\*[^*]+\*\s*/g, '').trim();
        if (!cleaned) cleaned = text;
        if (actionMatch) {
            invoke('emit_cat_status', { status: '🐱 ' + actionMatch[1] }).catch(() => {});
        } else {
            invoke('emit_cat_status', { status: '💬 正在回复你...' }).catch(() => {});
        }
        this.show(cleaned, { cls: 'chat-reply' });
        invoke('emit_dev_log', { tag: 'BUBBLE', tagClass: 'reaction', message: '[chat] "' + cleaned + '"' }).catch(() => {});
        this.timers.hide = setTimeout(() => {
            this.hide();
            invoke('emit_cat_status', { status: '😺 聊完了～开心！' }).catch(() => {});
            this.timers.next = setTimeout(() => this.nextIdle(), this.idleGapMs || 180000);
        }, this.bubbleDurationMs || 30000);
    },

    onActivity(text) {
        this.clearTimers();
        this.channel = 'activity';
        let cleaned = text.replace(/\*[^*]+\*\s*/g, '').trim();
        if (!cleaned) cleaned = text;
        const actionMatch = text.match(/\*([^*]+)\*/);
        if (actionMatch) {
            invoke('emit_cat_status', { status: '🐱 ' + actionMatch[1] }).catch(() => {});
        } else {
            invoke('emit_cat_status', { status: '💬 在跟你说话中...' }).catch(() => {});
        }
        this.show(cleaned);
        invoke('emit_dev_log', { tag: 'BUBBLE', tagClass: 'event', message: '"' + cleaned + '"' }).catch(() => {});
        this.timers.hide = setTimeout(() => {
            this.hide();
            invoke('emit_cat_status', { status: '😺 安静陪伴中...' }).catch(() => {});
            this.timers.next = setTimeout(() => this.nextIdle(), this.idleGapMs || 180000);
        }, this.bubbleDurationMs || 30000);
    },

    onModule(text) {
        this.clearTimers();
        this.channel = 'module';
        this.show(text);
        invoke('emit_dev_log', { tag: 'BUBBLE', tagClass: 'url', message: '[module] "' + text + '"' }).catch(() => {});
        invoke('emit_cat_status', { status: '💬 在跟你说话中...' }).catch(() => {});
        this.timers.hide = setTimeout(() => {
            this.hide();
            invoke('emit_cat_status', { status: '😺 说完了～安静陪伴中...' }).catch(() => {});
            this.timers.next = setTimeout(() => this.nextIdle(), this.idleGapMs || 180000);
        }, this.bubbleDurationMs || 30000);
    },

    onCR(text) {
        this.clearTimers();
        this.channel = 'cr';
        this.show(text, { animate: true });
        const crStatuses = [
            '😼 正在八卦队友代码...', '😼 Judging your teammate\'s code...',
            '😼 翻队友的commit记录...', '😼 Snooping on PRs...',
            '😼 八卦时间到！谁又写bug了？', '😼 Code gossip mode activated...',
            '😼 偷看别人写了什么...嘿嘿', '😼 Reading commits like a tabloid...',
        ];
        invoke('emit_cat_status', { status: crStatuses[Math.floor(Math.random() * crStatuses.length)] }).catch(() => {});
        this.timers.hide = setTimeout(() => {
            this.hide();
            invoke('emit_cat_status', { status: '😺 安静陪伴中...' }).catch(() => {});
            this.timers.next = setTimeout(() => this.nextIdle(), this.idleGapMs || 180000);
        }, 15000);
    },

    onMeme(dataUrl) {
        this.clearTimers();
        this.channel = 'module';
        const inner = document.createElement('div');
        inner.className = 'pixel-bubble-inner meme';
        const img = document.createElement('img');
        img.src = dataUrl;
        inner.appendChild(img);
        const bubble = document.createElement('div');
        bubble.className = 'pixel-bubble';
        bubble.appendChild(inner);
        bubbleEl.innerHTML = '';
        bubbleEl.appendChild(bubble);
        bubbleEl.classList.add('show', 'pop');
        invoke('emit_dev_log', { tag: 'MEME', tagClass: 'reaction', message: '[meme] shown' }).catch(() => {});
        this.timers.hide = setTimeout(() => {
            this.hide();
            this.timers.next = setTimeout(() => this.nextIdle(), this.idleGapMs || 180000);
        }, this.bubbleDurationMs || 30000);
    },

    async nextIdle() {
        if (this.channel === 'touch' || this.channel === 'chat') return;
        for (const mod of this.modules) {
            const now = Date.now();
            let interval;
            if (mod.frequency === 'daily') interval = 86400000;
            else if (mod.frequency === 'hourly') interval = 3600000;
            else interval = (mod.intervalMinutes || 30) * 60000;
            if (now - (mod.lastRun || 0) < interval) continue;
            try {
                const msg = await mod.getMessage();
                mod.lastRun = Date.now();
                try { localStorage.setItem('mod_' + mod.id, String(mod.lastRun)); } catch (e) {}
                if (msg) {
                    invoke('emit_dev_log', { tag: 'MOD', tagClass: 'url', message: 'built-in "' + mod.id + '" fired: "' + msg.slice(0, 50) + '"' }).catch(() => {});
                    this.onModule(msg);
                    return;
                }
            } catch (e) {}
        }
        invoke('emit_dev_log', { tag: 'IDLE', tagClass: 'skip', message: 'no built-in module due, calling AI...' }).catch(() => {});
        await this.runActivity();
    },

    async runActivity() {
        if (this.channel === 'module') {
            invoke('emit_dev_log', { tag: 'AI', tagClass: 'skip', message: 'skipped — module reaction active' }).catch(() => {});
            return;
        }
        let context = 'User is at their computer';
        try { context = await invoke('get_activity_context'); } catch (e) {}
        try {
            const message = await invoke('generate_message', { context });
            invoke('emit_dev_log', { tag: 'AI', tagClass: 'reaction', message: 'ctx: ' + context.slice(0, 60) }).catch(() => {});
            this.onActivity(message);
        } catch (e) {
            const fb = getActivityFallback(context);
            invoke('emit_dev_log', { tag: 'AI', tagClass: 'error', message: 'AI failed, using fallback' }).catch(() => {});
            this.onActivity(fb);
        }
    },

    registerModule(config) {
        const saved = parseInt(localStorage.getItem('mod_' + config.id)) || 0;
        config.lastRun = saved;
        this.modules.push(config);
    },

    async start() {
        try {
            const config = await invoke('get_config');
            const idleGap = (config.idle_gap_minutes || config.interval_minutes || 3) * 60 * 1000;
            const chattiness = config.chattiness || 3;
            const multipliers = { 1: 3, 2: 2, 3: 1, 4: 0.6, 5: 0.3 };
            this.idleGapMs = idleGap * (multipliers[chattiness] || 1);
            this.bubbleDurationMs = (config.bubble_duration_secs || 30) * 1000;
        } catch (e) {
            this.idleGapMs = 180000;
            this.bubbleDurationMs = 30000;
        }
        this.onActivity(fillNick(pick(fallbackMessages)));
        setTimeout(() => this.runActivity(), 15000);
    }
};

// ━━━━━━━━━━━━━━━━━━━━━━
// REGISTER MODULES
// ━━━━━━━━━━━━━━━━━━━━━━
scheduler.registerModule({
    id: 'weather', frequency: 'daily',
    getMessage: async () => {
        const ctx = await invoke('get_all_context');
        if (!ctx.weather || ctx.weather.includes('unavailable')) return null;
        return await invoke('generate_message', { context: ctx.weather });
    }
});
scheduler.registerModule({
    id: 'spotify', frequency: 'hourly',
    getMessage: async () => {
        const ctx = await invoke('get_all_context');
        if (!ctx.spotify || ctx.spotify.includes('not running')) return null;
        return await invoke('generate_message', { context: ctx.spotify });
    }
});
scheduler.registerModule({
    id: 'battery', intervalMinutes: 15,
    getMessage: async () => {
        const ctx = await invoke('get_all_context');
        if (ctx.system) {
            const match = ctx.system.match(/(\d+)%/);
            if (match && parseInt(match[1]) < 20) return fillNick('{n}, 电量只有' + match[1] + '%了！快充电！🔋');
        }
        return null;
    }
});
scheduler.registerModule({
    id: 'quotes', intervalMinutes: 30,
    getMessage: async () => {
        const ctx = await invoke('get_all_context');
        if (ctx.quotes) return fillNick('{n}, ') + ctx.quotes;
        return null;
    }
});
scheduler.registerModule({
    id: 'zoom', intervalMinutes: 10,
    getMessage: async () => {
        const ctx = await invoke('get_all_context');
        if (!ctx.zoom || ctx.zoom.includes('not running')) return null;
        if (ctx.zoom.includes('in a meeting')) {
            return fillNick(pick(['{n}, 开会辛苦了！加油～🐱', '{n}, 在开会呢！我安静等你～', '{n}, meeting中！记得喝水～', '{n}, 开完会来找我玩！😸']));
        }
        return null;
    }
});
scheduler.registerModule({
    id: 'performance', intervalMinutes: 5,
    getMessage: async () => {
        const ctx = await invoke('get_all_context');
        if (ctx.performance && ctx.performance.includes('WARNING')) {
            return await invoke('generate_message', { context: ctx.performance + '\nWarn the user their laptop is overloaded! Use words like 超载, 太热了, 要爆炸了' });
        }
        return null;
    }
});

// ━━━━━━━━━━━━━━━━━━━━━━
// TOUCH HANDLERS
// ━━━━━━━━━━━━━━━━━━━━━━
let clickTimer = null;
let tapCount = 0;
let tapResetTimer = null;
let dragStartPos = null;
let wasDragged = false;

const angryMessages = [
    '够了！！！别碰我！！😡🔥 {k}', '你有完没完！！！💢💢💢',
    '再碰一下试试！！！(=`ω´=)凸', '暴怒模式启动！！！🐱‍👤💢',
    '我要咬人了！！！啊啊啊！😾💢', 'STOP IT!!! I WILL BITE!!! 💢{k}',
    '你是不是皮痒了！！！(=｀ェ´=)', '够了够了够了！！！炸毛了！！🙀💥',
];

petEl.addEventListener('mousedown', (e) => { dragStartPos = { x: e.screenX, y: e.screenY }; wasDragged = false; });
petEl.addEventListener('mousemove', (e) => {
    if (dragStartPos) {
        if (Math.abs(e.screenX - dragStartPos.x) > 5 || Math.abs(e.screenY - dragStartPos.y) > 5) wasDragged = true;
    }
});
petEl.addEventListener('mouseup', () => { dragStartPos = null; });

petEl.addEventListener('click', (e) => {
    e.stopPropagation();
    if (wasDragged) { wasDragged = false; return; }
    playAnim();
    tapCount++;
    clearTimeout(tapResetTimer);
    tapResetTimer = setTimeout(() => { tapCount = 0; }, 3000);
    clearTimeout(clickTimer);
    clickTimer = setTimeout(async () => {
        if (tapCount >= 5) {
            scheduler.onTouch(fillNick(pick(angryMessages)));
            invoke('pet_angry').catch(() => {});
            invoke('context_on_rage').catch(() => {});
            invoke('log_status_change', { source: 'touch', detail: 'rage tap (5x)', delta: -15 }).catch(() => {});
            showFloatingNumber(-15);
            tapCount = 0;
        } else {
            scheduler.onTouch(pick(tapMessages));
            invoke('pet_touched').catch(() => {});
            invoke('log_status_change', { source: 'touch', detail: 'pet touched', delta: 3 }).catch(() => {});
            showFloatingNumber(+3);
        }
    }, 250);
});

petEl.addEventListener('dblclick', (e) => {
    e.stopPropagation();
    clearTimeout(clickTimer);
    playAnim();
    scheduler.onTouch(fillNick(pick(pokeMessages)));
    invoke('pet_touched').catch(() => {});
    invoke('log_status_change', { source: 'touch', detail: 'pet poked', delta: 3 }).catch(() => {});
    showFloatingNumber(+3);
});

// ━━━━━━━━━━━━━━━━━━━━━━
// CHAT (Ctrl+Cmd+C)
// ━━━━━━━━━━━━━━━━━━━━━━
function openChat() {
    bubbleEl.classList.remove('show');
    chatWrapper.classList.add('show');
    chatInput.value = '';
    setTimeout(() => chatInput.focus(), 50);
}
function closeChat() {
    chatWrapper.classList.remove('show');
    chatInput.blur();
}

listen('open-chat', () => openChat());

async function submitChat() {
    const text = chatInput.value.trim();
    if (!text) return;
    closeChat();
    scheduler.onChat('thinking~🤔');
    try {
        const reply = await invoke('chat_message', { userText: text });
        scheduler.onChat(reply || fillNick('{n}：...嗯？🤔'));
        invoke('pet_chatted').catch(() => {});
        invoke('context_on_chat').catch(() => {});
        invoke('log_status_change', { source: 'chat', detail: 'chatted with pet', delta: 5 }).catch(() => {});
        showFloatingNumber(+5);
    } catch (err) {
        scheduler.onChat('Error: ' + String(err).slice(0, 60));
    }
}

chatInput.addEventListener('keydown', (e) => { e.stopPropagation(); if (e.key === 'Escape') closeChat(); });
chatInput.addEventListener('keypress', (e) => { e.stopPropagation(); if (e.key === 'Enter') { e.preventDefault(); submitChat(); } });
chatInput.addEventListener('mousedown', (e) => e.stopPropagation());
chatInput.addEventListener('click', (e) => { e.stopPropagation(); chatInput.focus(); });

// ━━━━━━━━━━━━━━━━━━━━━━
// TYPING & TIME WATCHER
// ━━━━━━━━━━━━━━━━━━━━━━
let lastTypingAlert = 0;
let lastTimeAlert = {};

setInterval(async () => {
    if (scheduler.channel === 'touch' || scheduler.channel === 'chat') return;
    if (scheduler.channel === 'activity' || scheduler.channel === 'module') return;
    try {
        const ctx = await invoke('get_activity_context');
        const now = Date.now();
        if (ctx.includes('[typing]') && now - lastTypingAlert > 900000) {
            lastTypingAlert = now;
            scheduler.onActivity(getActivityFallback(ctx));
            return;
        }
        const hour = new Date().getHours();
        let timeMsg = null, slot = null;
        if ((hour >= 23 || hour < 5) && now - (lastTimeAlert.night || 0) > 21600000) {
            slot = 'night';
            timeMsg = pick(['{n}, 都这么晚了！早点休息吧～ {k}', '{n}, 猫猫困了...晚安～ {k}', '{n}, 别熬夜了！🌙 {k}']);
        } else if (hour >= 9 && hour < 10 && now - (lastTimeAlert.morning || 0) > 43200000) {
            slot = 'morning';
            timeMsg = pick(['{n}, 早上好！打工人加油～☀️ {k}', '{n}, good morning! {k}', '{n}, 早！记得吃早餐哦～ {k}']);
        } else if (hour === 12 && now - (lastTimeAlert.lunch || 0) > 43200000) {
            slot = 'lunch';
            timeMsg = pick(['{n}, 午饭时间到！去吃饭！🍚 {k}', '{n}, 12点了！该吃lunch了～ {k}', '{n}, 别忘了吃午饭！{k}']);
        } else if (hour >= 18 && hour < 19 && now - (lastTimeAlert.evening || 0) > 43200000) {
            slot = 'evening';
            timeMsg = pick(['{n}, 下班了！该回家了！🏠 {k}', '{n}, 收工收工！回家撸猫！{k}']);
        }
        if (timeMsg && slot) {
            lastTimeAlert[slot] = now;
            scheduler.onActivity(fillNick(timeMsg));
        }
    } catch (e) {}
}, 60000);

// ━━━━━━━━━━━━━━━━━━━━━━
// FLOATING NUMBERS
// ━━━━━━━━━━━━━━━━━━━━━━
let floatCount = 0;
function showFloatingNumber(delta) {
    if (delta === 0 || floatCount >= 5) return;
    const el = document.createElement('div');
    el.className = 'float-num ' + (delta > 0 ? 'positive' : 'negative');
    el.textContent = (delta > 0 ? '+' : '') + delta;
    el.style.bottom = (floatCount * 20) + 'px';
    floatContainer.appendChild(el);
    floatCount++;
    setTimeout(() => { el.remove(); floatCount--; }, 1200);
}

// ━━━━━━━━━━━━━━━━━━━━━━
// RUNTIME EVENT LISTENERS
// ━━━━━━━━━━━━━━━━━━━━━━
listen('module-reaction', (event) => {
    const data = event.payload;
    if (!data || !data.message) return;
    if (scheduler.channel === 'touch' || scheduler.channel === 'chat') return;
    if (scheduler.isBusy()) return;
    scheduler.clearTimers();
    playAnim();
    if (data.priority >= 7 && data.module_id === 'amazon-internal') {
        scheduler.onCR(data.message);
    } else {
        scheduler.onModule(data.message);
    }
});

listen('pet-status-changed', (event) => {
    const data = event.payload;
    if (data && data.delta) showFloatingNumber(data.delta);
});

listen('meme-reaction', (event) => {
    const data = event.payload;
    if (!data || !data.data_url) return;
    if (scheduler.channel === 'touch' || scheduler.channel === 'chat') return;
    if (scheduler.isBusy()) return;
    scheduler.clearTimers();
    playAnim();
    scheduler.onMeme(data.data_url);
});

// ━━━━━━━━━━━━━━━━━━━━━━
// BOOT
// ━━━━━━━━━━━━━━━━━━━━━━
(async () => {
    try {
        const config = await invoke('get_config');
        if (config.user_nickname) userNickname = config.user_nickname;
        if (config.nicknames && config.nicknames.length > 0) nicknames = config.nicknames;
    } catch (e) {}
    try {
        const meowNicks = await invoke('get_meow_nicknames');
        if (meowNicks && meowNicks.length > 0) nicknames = meowNicks;
    } catch (e) {}
    scheduler.start();
})();
