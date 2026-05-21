# Contextual Pet Personality

ClaudeMeow's AI messages adapt based on accumulated context throughout the day. The pet feels more alive because it remembers what happened and adjusts its tone.

## Context Signals

### Time of Day
The pet's personality shifts with the clock:
- **Morning (6-8)**: Energetic, cheerful greetings
- **Mid-morning (9-11)**: Productive, encouraging
- **Lunch (12-13)**: Hungry, reminds about food
- **Afternoon (14-16)**: Gentle encouragement, slightly tired
- **Evening (17-18)**: Suggests going home
- **Night (19-21)**: Cozy, relaxed
- **Late night (22+)**: Very sleepy, yawning, nags about sleep

### Activity Streaks
The pet notices patterns:
- **Coding streak (30+ min)**: "都写了45分钟了！站起来走走嘛！"
- **Rapid app switching (10+ in 10 min)**: "怎么坐不住？是不是在摸鱼呀？"
- **Long duration on one app (20+ min)**: Mentions the specific app

### Mood Memory
The pet remembers interactions:
- **After rage tap**: Cautious for 10 minutes, approaches gently
- **After multiple rages**: Wary all day
- **After chatting**: Happy and playful
- **After many chats (5+)**: Extra clingy and affectionate

## How It Works

A `ContextEngine` in Rust tracks:
- Current app and duration
- App switch frequency
- Coding streak length
- Rage count and recency
- Chat count and recency

This context is injected into every AI prompt as personality instructions. Claude then generates messages that match the current emotional/temporal state.

## Resets
- Streaks reset when you switch apps
- Mood memory resets on app restart
- Rage/chat counts reset daily at 9 AM
