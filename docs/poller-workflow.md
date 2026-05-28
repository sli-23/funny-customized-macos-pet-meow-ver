# Poller Workflow

The poller is the core engine that runs every 2 seconds, detects what you're doing, and triggers reactions.

## Main Loop

```mermaid
flowchart TD
    START[Poller tick every 2s] --> FETCH[Fetch active window via AppleScript]
    FETCH --> SELF{Is ClaudeMeow itself?}
    SELF -->|Yes| SKIP[Skip this tick]
    SELF -->|No| BLANK{Title blank + same app?}
    BLANK -->|Yes, same app| SKIP
    BLANK -->|No, or different app| CHANGED{Window info changed?}

    CHANGED -->|No change| HEARTBEAT{Heartbeat due? 5min}
    HEARTBEAT -->|Yes| POLL_LOG[Log POLL + update cat status]
    HEARTBEAT -->|No| PERIODIC{Periodic eval? 30s + cooldown passed?}
    PERIODIC -->|Yes| REEVAL[Re-evaluate reactions on current app]
    PERIODIC -->|No| SKIP

    CHANGED -->|Changed| APP_SWITCH{App name different?}
    APP_SWITCH -->|No, title only| TITLE_LOG[Log APP title change]
    APP_SWITCH -->|Yes, real switch| SWITCH_LOG[Log SWITCH]

    SWITCH_LOG --> SCREEN_TIME[Update screen time tracker]
    SWITCH_LOG --> CONTEXT[Update context engine]
    SWITCH_LOG --> BROWSER{Is browser app?}

    BROWSER -->|Yes| FETCH_URL[Fetch URL via AppleScript]
    FETCH_URL --> SITE_MATCH[Extract site from URL]
    SITE_MATCH --> EVAL_SITE[Evaluate site reactions]

    BROWSER -->|No| EVAL_APP[Evaluate app_name reactions]

    EVAL_SITE --> CHECK_COOLDOWN{Cooldown check}
    EVAL_APP --> CHECK_COOLDOWN

    CHECK_COOLDOWN -->|Global cooldown active| SKIP_LOG[Log SKIP]
    CHECK_COOLDOWN -->|Ready| MATCH{Any module matches?}

    MATCH -->|No match| MISS_LOG[Log MISS]
    MATCH -->|Match but on cooldown| COOLDOWN_LOG[Log COOLDOWN module::id]
    MATCH -->|Match + ready| FIRE[FIRE → emit module-reaction to frontend]

    FIRE --> BUBBLE[Frontend shows BUBBLE]
```

## Subsystems (also tick each cycle)

```mermaid
flowchart TD
    TICK[Each 2s tick] --> CONFIG{Config reload? 60s}
    CONFIG -->|Yes| RELOAD[Reload config, update health/calendar settings]
    CONFIG -->|No| SKIP1[ ]

    TICK --> DAILY{Commits stale? 24h}
    DAILY -->|Yes| FETCH_COMMITS[Auto-fetch fresh commits from API]
    DAILY -->|No| SKIP2[ ]

    TICK --> HEALTH[Health Reminder tick]
    HEALTH --> H_ENABLED{Enabled?}
    H_ENABLED -->|No| SKIP3[ ]
    H_ENABLED -->|Yes| H_QUIET{Quiet hours?}
    H_QUIET -->|Yes| SKIP3
    H_QUIET -->|No| H_INTERVAL{Interval passed? 15min}
    H_INTERVAL -->|Yes| H_FIRE[Emit health message pri=9]
    H_INTERVAL -->|No| SKIP3[ ]

    TICK --> CALENDAR[Calendar Reminder tick]
    CALENDAR --> C_ENABLED{Enabled + alias set?}
    C_ENABLED -->|No| SKIP4[ ]
    C_ENABLED -->|Yes| C_DAILY{Morning fetch? 8h}
    C_DAILY -->|Due| C_FETCH[Fetch 12h of events]
    C_DAILY -->|Not due| C_CHECK[ ]
    C_CHECK --> C_INTERVAL{Check interval? 2min}
    C_INTERVAL -->|Yes| C_SCAN[Scan cached events]
    C_SCAN --> C_MEETING{Meeting within 10 min?}
    C_MEETING -->|Yes| C_FIRE[Emit meeting reminder pri=8]
    C_MEETING -->|No| SKIP4[ ]

    TICK --> SECRET[Secret Meow tick]
    SECRET --> S_LOADED{.meow loaded?}
    S_LOADED -->|No| SKIP5[ ]
    S_LOADED -->|Yes| S_INTERVAL{30 min passed?}
    S_INTERVAL -->|Yes| S_FIRE[Emit one secret message pri=6]
    S_INTERVAL -->|No| SKIP5[ ]

    TICK --> FLUSH[Flush cooldowns to disk if dirty]
```

## Frontend Bubble Priority

```mermaid
flowchart TD
    EVENT[module-reaction event arrives] --> GUARD{Channel guard}
    GUARD -->|channel = touch| DROP[Dropped — user interacting]
    GUARD -->|channel = chat| DROP
    GUARD -->|isBusy? 2s post-hide| DROP
    GUARD -->|channel = activity/module/null| REPLACE[Clear timers, show new bubble]

    REPLACE --> SHOW[Display bubble for bubbleDurationMs]
    SHOW --> HIDE[Hide after timeout]
    HIDE --> BUSY[Set busyUntil = now + 2s]
    BUSY --> IDLE_GAP[Wait idleGapMs × chattiness]
    IDLE_GAP --> NEXT_IDLE[nextIdle — check built-in modules]
    NEXT_IDLE --> WISDOM{Random 20%?}
    WISDOM -->|Yes| QUOTE[Show wisdom/API quote]
    WISDOM -->|No 80%| AI[Call AI generate_message]
```

## Startup Sequence

```mermaid
sequenceDiagram
    participant App as App (main.rs)
    participant Poller as Poller (Rust)
    participant Frontend as Frontend (index.js)
    participant Backend as Backend events

    App->>App: Load config, modules, SecretMeow
    App->>Poller: Spawn polling task
    App->>Frontend: Load index.html

    Frontend->>Frontend: scheduler.start()
    Frontend->>Frontend: Wait 5s...

    Poller->>Poller: First tick (2s)
    Poller->>Poller: Detect active app
    Poller->>Backend: Emit module-reaction (if match)
    Backend->>Frontend: module-reaction event

    Frontend->>Frontend: 5s passed, channel set?
    alt Module fired within 5s
        Frontend->>Frontend: Skip fallback greeting
    else No module fired
        Frontend->>Frontend: Show fallback greeting
    end
```

## Reaction Evaluation

```mermaid
flowchart TD
    EVENT[Event to evaluate] --> LOOP[For each module+reaction pair]
    LOOP --> ACTIVE{Module active?}
    ACTIVE -->|No| NEXT[Next pair]
    ACTIVE -->|Yes| DISABLED{Reaction in disabled list?}
    DISABLED -->|Yes| NEXT
    DISABLED -->|No| PERMISSION{Module has permission for event type?}
    PERMISSION -->|No| NEXT
    PERMISSION -->|Yes| TRIGGER{Event matches trigger condition?}
    TRIGGER -->|No| NEXT
    TRIGGER -->|Yes| COOLDOWN{On per-reaction cooldown?}
    COOLDOWN -->|Yes| BLOCKED[Mark as blocked]
    COOLDOWN -->|No| CANDIDATE[Add to candidates]

    CANDIDATE --> SORT[Sort by priority descending]
    SORT --> WINNER[Pick highest priority]
    WINNER --> SET_COOLDOWN[Set cooldown for this reaction]
    WINNER --> EMIT[Return FiredReaction]
```
