# Remote Control sessions missing from the Claude iPad/iOS app

Research 2026-10-15. Context: our sessions are interactive `claude --remote-control` processes in tmux
(~/dev/bin/claude-remote.sh). Sometimes they don't show up in the Code tab of the iPad app or in the
claude.ai/code sidebar.

## Verdict
This is a known, recurring bug that Anthropic has not acknowledged. The CLI registers the session (the URL works),
but the session-list endpoint or client leaves some sessions out. Most issues get auto-closed as stale
("not planned"), and no issue has an Anthropic answer. The docs list no limit on the number of listed sessions.

## Best workarounds (most reliable first)
1. **Open the printed session URL** (`https://claude.ai/code/session_…`) once, on the iPad. After that, the
   session usually also appears in the app's list (#30691, #28402). claude-remote.sh already prints it: keep the URLs
   in one place, e.g. a notes file or the supervisor's reply.
2. **Go through Safari**: open claude.ai/code in Safari and tap "Open in app". That lands in the Code tab with
   the list. Scanning the QR code can land in the normal chat view instead (#28758).
3. **Check the archived filter**: the docs say Claude Code *archives* the server session when you reconnect
   with `/remote-control` after compaction or a `/resume` switch, and when org policy turns Remote Control
   off. Archived sessions are hidden from the default list (sidebar → filter archived). Long-running workers
   that compact are likely candidates.
4. **Send the session a message / let it work**: one report says the row only shows while the model is
   generating and vanishes again when the session goes idle (#28402, comment 2026-04-21).
5. **Re-authenticate**: `claude auth logout && claude auth login` on the Mac fixed it for some people (#28758).
   Use the same account *and organization* on both ends; `/status` shows which org the CLI uses.
6. **Set a stable name**: `--name` / `--remote-control <name>` sets the list title (docs), so a session can be
   found by name instead of as `host-random-words`.
7. **GitHub gate**: if claude.ai/code redirects to GitHub onboarding, the list sits behind it. Connecting GitHub
   shows the list (#28758, #97226 is still open).
8. Sometimes the list comes back by itself after a server-side fix (#30691, 2026-04-17). Updating the iOS app and
   CLI doesn't hurt.

## Issues (anthropics/claude-code)
| # | Title (short) | Status |
|---|---|---|
| [28402](https://github.com/anthropics/claude-code/issues/28402) | RC session not visible in list, cannot reconnect after leaving | OPEN (21 comments; reported "fixed in 2.1.76" but still reproduced; variants: crash, compaction, idle) |
| [28758](https://github.com/anthropics/claude-code/issues/28758) | session not connecting from mobile app (not in Sessions list) | OPEN (28 comments; workarounds: re-login, Safari → Open in app, GitHub connect) |
| [84929](https://github.com/anthropics/claude-code/issues/84929) | sessions register locally but never appear on claude.ai/code or mobile | OPEN (macOS, Max plan) |
| [97226](https://github.com/anthropics/claude-code/issues/97226) | RC sessions not reachable from claude.ai/code without session URL (GitHub onboarding gate) | OPEN |
| [101132](https://github.com/anthropics/claude-code/issues/101132) | iOS sessions hang on desktop-served RC ("That mode isn't available") | OPEN (works with terminal `claude remote-control --spawn same-dir`) |
| [30691](https://github.com/anthropics/claude-code/issues/30691) | RC sessions don't appear in mobile app until URL opened manually | CLOSED not planned (stale); regression around 2.1.111 |
| [78601](https://github.com/anthropics/claude-code/issues/78601) | list shows far fewer entries (3) than registered sessions (50) | CLOSED not planned (stale) — closest match to our symptom |
| [56343](https://github.com/anthropics/claude-code/issues/56343) | iOS list doesn't show `claude remote-control` sessions from Linux | CLOSED not planned (stale) |
| [64841](https://github.com/anthropics/claude-code/issues/64841) | iPhone "No sessions found" with RC-by-default on | CLOSED not planned |
| [87756](https://github.com/anthropics/claude-code/issues/87756) | RC session active but lookup fails on claude.ai/code | CLOSED not planned |
| [96591](https://github.com/anthropics/claude-code/issues/96591) | /remote-control no confirmation, sessions never appear | CLOSED not planned (1 day) |
| [41670](https://github.com/anthropics/claude-code/issues/41670) | RC doesn't work on iPhone 17 Pro | CLOSED not planned |
| [30922](https://github.com/anthropics/claude-code/issues/30922) | `--remote` cloud sessions orphaned, no mobile visibility | CLOSED not planned |
| [29006](https://github.com/anthropics/claude-code/issues/29006) | RC for Claude Desktop sessions | CLOSED completed |
| [78429](https://github.com/anthropics/claude-code/issues/78429) | feature: show RC status in session lists | CLOSED not planned |

No Reddit or HN threads found. The Zenn article ([iphone-claude-code-remote-control](https://zenn.dev/shimo4228/articles/iphone-claude-code-remote-control?locale=en))
confirms that `claude --remote-control` in detached tmux adds each session as its own list entry, which is our setup.

## Official docs (code.claude.com/docs/en/remote-control)
- The list is at claude.ai/code. In the mobile app it's under the **Code** tab. An online RC session shows a computer icon with a green dot.
- Title order: `--name`/`--remote-control <name>` → `/rename` → last message → `host-adjective-noun`.
- Archiving: reconnecting after compaction or a `/resume` switch archives the old server session. So does an org policy turning RC off,
  and so does stopping a `--no-create-session-in-dir` server. Sessions archived from another device report "ended or archived".
- `claude remote-control` *server* mode exits after about 10 minutes offline. Interactive mode (ours) retries indefinitely.
- No documented cap on the number of sessions listed, and no documented sync delay.

## Possible follow-ups for us
- claude-remote.sh already passes `--name`/`--remote-control <tmux name>`. It could also append each printed URL
  to a list file the user can open on the iPad.
- If it keeps happening: file a fresh issue that references #78601 and #28402 (stale-closed). Include the CLI version,
  the session count, and whether the missing sessions had compacted.
