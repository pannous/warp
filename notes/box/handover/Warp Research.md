# Warp Research handover (2026-10-23)
Done: Claude Code telemetry -> local Mac Phoenix (project claude-code, warp.role tag via ~/dev/bin/claude-remote.sh); verified spans arriving.
Done: summarize/todo prompt caching (cache_control on system block); measured ~5.8k cached tokens on repeat calls. pannous/bin main @ 88bb466, pushed.
Done: server Phoenix admin password reset; saved in Mac login keychain "Phoenix (localhost:16006)", account admin@localhost.
Done: Worker warp-assistant deployed (version 665ad23e) from warp branch assistant-prompt-cache @ 76b8a651b1.
Open: assistant-prompt-cache awaiting warp-integrator merge (asked 2026-10-22 16:39 CEST).
Next: after merge, `git worktree remove /tmp/warp-assistant-cache` and delete branch assistant-prompt-cache.
Open (inferred, unchanged): 400s from workspace wrkspc_01GS... likely Worker key ANTHROPIC_KEY_WARP.
Dropped by user: live site/playground caching verification (not important).
Not possible: per-script tagging on Anthropic API trace path without per-script API keys/workspaces.
Note: running claude sessions pick up telemetry only at next swap (not restarted, per rules).
Backups: ~/backups-20261022/ on Mac. Box data: /workspace/phoenix/.
