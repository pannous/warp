# Remote Control sessions missing from the Claude iPad/iOS app

Our sessions are interactive `claude --remote-control` processes in tmux (~/dev/bin/claude-remote.sh). Sometimes
they don't show up in the Code tab of the iPad app or in the claude.ai/code sidebar, although the session URL works.
A known, recurring client/list bug without an Anthropic answer; most issues get auto-closed as stale. No documented
cap on listed sessions and no documented sync delay.

## Workarounds (most reliable first)
1. Open the printed session URL (`https://claude.ai/code/session_…`) once on the iPad; afterwards the session usually
   appears in the list too. claude-remote.sh appends every spawned session's URL to iCloud
   Drive/claude-sessions.html (Files app → tap the file → tap a link).
2. Safari: open claude.ai/code and tap "Open in app" (lands in the Code tab; the QR code can land in chat instead).
3. Archived filter: reconnecting after compaction or a `/resume` switch archives the old server session; archived
   sessions are hidden from the default list. Long-running workers that compact are likely candidates.
4. Force-quit and reopen the app; also look under "All chats". Sometimes a session only appears after a delay, or
   only while it is generating.
5. Re-authenticate (`claude auth logout && claude auth login`), same account and organization on both ends
   (`/status` shows the CLI's org).
6. A stable name (`--name` / `--remote-control <name>`, which claude-remote.sh passes) makes sessions findable.
7. If claude.ai/code redirects to GitHub onboarding, the list sits behind it: connecting GitHub shows it.

## Docs facts (code.claude.com/docs/en/remote-control)
- Title order: `--name`/`--remote-control <name>` → `/rename` → last message → `host-adjective-noun`.
- `claude remote-control` server mode exits after about 10 minutes offline; interactive mode (ours) retries forever.

## Issues (anthropics/claude-code)
Open: #28402 (not visible, cannot reconnect), #28758 (mobile, workarounds), #84929 (never appear on claude.ai/code),
#97226 (GitHub onboarding gate), #101132 (iOS hangs on desktop-served RC). Closed as stale: #78601 (list shows 3 of
50 sessions: closest to our symptom), #30691 (appear only after opening the URL). A fresh report should reference
#78601 and #28402 with the CLI version, the session count and whether the missing sessions had compacted.
