# Priority System

ClaudeMeow uses a 4-level priority system to decide which module reacts first when multiple modules match the same event.

## Levels

| Level | Label | Description |
|-------|-------|-------------|
| 1 | Critical | Always fires first. Reserved for health reminders. |
| 2 | Important | Main work reactions. Coding, Amazon Internal, Zoom. |
| 3 | Normal | Standard reactions. Slack, GitHub, Spotify, etc. |
| 4 | Low | Only fires when nothing above matches. |

## How It Works

1. An event occurs (e.g., user switches to Chrome with GitHub open)
2. The engine collects all matching reactions from all enabled modules
3. Groups them by priority level
4. Picks the highest level group that has matches
5. Randomly picks one from that group
6. Fires it as the pet's reaction

## Same-Level Behavior

If multiple modules at the same level both match, one is picked **randomly**. No ordering within the same level.

## Configuring

Open Settings → Priority tab. Each module shows 4 buttons [1] [2] [3] [4]. Click to assign.

New modules default to level 3 (Normal) unless specified in their manifest.

## Storage

Priority overrides are saved to `~/Library/Application Support/claude-meow-pet/priority_overrides.json`.
