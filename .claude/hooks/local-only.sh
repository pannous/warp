#!/bin/sh
# Runs a repo hook only on the local Mac. GitHub's Copilot coding agent reads these Claude hook settings too, but on its
# Actions runner the hooks error (no CLAUDE_PROJECT_DIR, no node_modules), and an erroring hook denies every tool call.
[ -n "$GITHUB_ACTIONS" ] && exit 0
exec "$@"
