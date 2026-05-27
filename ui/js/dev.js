import { invoke, listen, escapeHtml } from './ipc.js';

const logArea = document.getElementById('logArea');

function addLog(tag, tagClass, msg) {
    const now = new Date();
    const time = now.getHours().toString().padStart(2, '0') + ':' +
                 now.getMinutes().toString().padStart(2, '0') + ':' +
                 now.getSeconds().toString().padStart(2, '0');

    const entry = document.createElement('div');
    entry.className = 'log-entry';

    const timeSpan = document.createElement('span');
    timeSpan.className = 'log-time';
    timeSpan.textContent = time;

    const tagSpan = document.createElement('span');
    tagSpan.className = 'log-tag ' + tagClass;
    tagSpan.textContent = '[' + tag + ']';

    const msgSpan = document.createElement('span');
    msgSpan.className = 'log-msg';
    msgSpan.textContent = ' ' + msg.replace(/\n/g, ' ');

    entry.appendChild(timeSpan);
    entry.appendChild(document.createTextNode(' '));
    entry.appendChild(tagSpan);
    entry.appendChild(msgSpan);

    logArea.appendChild(entry);
    logArea.scrollTop = logArea.scrollHeight;

    while (logArea.children.length > 200) {
        logArea.removeChild(logArea.firstChild);
    }
}

window.clearLog = () => { logArea.innerHTML = ''; };

let lastMemeTime = 0;
let lastCRTime = 0;
const BUTTON_COOLDOWN = 30000;

window.triggerMeme = async () => {
    const now = Date.now();
    if (now - lastMemeTime < BUTTON_COOLDOWN) {
        addLog('MEME', 'skip', 'cooldown — wait ' + Math.ceil((BUTTON_COOLDOWN - (now - lastMemeTime)) / 1000) + 's');
        return;
    }
    lastMemeTime = now;
    try {
        await invoke('trigger_meme');
        addLog('MEME', 'reaction', 'meme triggered');
    } catch (e) {
        addLog('MEME', 'error', '' + e);
    }
};

window.triggerCR = async () => {
    const now = Date.now();
    if (now - lastCRTime < BUTTON_COOLDOWN) {
        addLog('CR', 'skip', 'cooldown — wait ' + Math.ceil((BUTTON_COOLDOWN - (now - lastCRTime)) / 1000) + 's');
        return;
    }
    lastCRTime = now;
    addLog('CR', 'event', '正在八卦队友代码...');
    try {
        const result = await invoke('trigger_cr_comment');
        addLog('CR', 'reaction', result || 'done');
    } catch (e) {
        addLog('CR', 'error', '八卦失败: ' + e);
    }
};

listen('dev-log', (event) => {
    const data = event.payload;
    if (data) {
        addLog(data.tag || 'INFO', data.tag_class || 'event', data.message || '');
    }
});

listen('cat-status', (event) => {
    const data = event.payload;
    if (data && data.status) {
        const el = document.getElementById('catStatus');
        el.textContent = data.status;
        el.dataset.lastSet = Date.now();
    }
});

const idleBehaviors = [
    '😴 趴着睡觉中...', '🐾 玩毛线球ing...', '🐟 Catching fish in the data stream...',
    '💤 打盹中...zzz', '🧶 追自己尾巴转圈圈...', '👀 盯着你的光标看...',
    '🐱 舔爪子洗脸中...', '😺 安静陪伴中...', '🌙 蜷成一团取暖...',
    '🦋 追蝴蝶ing...差一点！', '📦 钻进纸箱探险...', '🍃 被落叶吸引了注意力...',
    '🐟 Fishing for bugs in the code...', '🧪 Mixing potions in the terminal...',
    '🎵 Humming a compile song...', '🔍 Sniffing for memory leaks...',
    '🐾 Leaving paw prints on your keyboard...', '☕ Knocking over your coffee (oops)...',
    '🖥 Sitting on your warm laptop...', '📡 Intercepting your WiFi signals...',
    '🎣 Hooking commits from the stream...', '🌊 Surfing the event bus...',
    '🐈 假装没看你其实在偷看...', '🎮 偷偷玩你的鼠标...',
    '🌸 在键盘上踩梅花步...', '🍙 想吃小鱼干想吃小鱼干...',
    '💭 思考人生的意义(其实在发呆)...', '🐾 Debugging your code with vibes...',
    '🧊 Pushing things off the table...', '🎪 Performing for invisible audience...',
    '🌈 Chasing the cursor rainbow...', '🐛 Found a bug! (literally eating it)...',
    '📋 Reviewing your PR with judgy eyes...', '🛸 Beaming up your uncommitted changes...',
    '🧲 被显示器的光吸引了...', '🎭 练习高冷表情中...', '🌿 假装自己是植物不动了...',
    '🐾 Compiling... just kidding, licking paws...', '🎯 Staring at a dot only I can see...',
    '🧁 Dreaming about forbidden treats...', '🎪 Being dramatic for no reason...',
    '📎 I see you have an error. Would you like help? (no) 📎',
    '🐾 偷偷把你的袜子藏起来了...', '🌀 Spinning in circles... it\'s called exercise...',
    '🎶 尾巴随着你打字的节奏摇摆中...', '🕵️ Investigating suspicious cursor movement...',
    '🐱 在你椅子底下伸懒腰...好舒服...', '🎁 Planning world domination... I mean napping...',
];
let idleIdx = 0;
setInterval(() => {
    const el = document.getElementById('catStatus');
    if (el) {
        const lastSet = parseInt(el.dataset.lastSet || '0');
        if (Date.now() - lastSet > 60000) {
            idleIdx = (idleIdx + 1) % idleBehaviors.length;
            el.textContent = idleBehaviors[idleIdx];
        }
    }
}, 120000);

addLog('SYS', 'event', 'dev console ready');
