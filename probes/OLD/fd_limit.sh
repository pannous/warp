#!/bin/bash
# Targeted tests under macOS's default limit of 256 open files, as Codex and cron runs get it (card targeted-tests):
# cached modules mapped from their .cwasm files kept one descriptor each open, and the tests failed with EMFILE.
#   probes/fd_limit.sh [filter]     default filter text::
OPEN_FILES_LIMIT=256
ulimit -n $OPEN_FILES_LIMIT
cd "$(dirname "$0")/.." && tests/queue.sh -- "${1:-text::}" 2>&1 | grep -E "^test result|Too many open files"
